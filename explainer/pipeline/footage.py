"""Motion graphics over a speaker's own footage.

The input video is transcribed with word timestamps. A plan (written by hand
or drafted by Claude) anchors each graphic to the phrase where it should
appear. The speaker plays full frame between graphics and tucks into a
picture-in-picture card while one is on screen.
"""

from __future__ import annotations

import hashlib
import json
import random
import subprocess
from pathlib import Path

import numpy as np
import soundfile as sf

from . import spec as specs
from .timeline import _find_phrase, resolve_cues, scene_events, scene_extra
from .voice import _whisper

LEAD_FRAMES = 6  # graphics land just before the anchor word
MIN_SECONDS = 2.5
MAX_SECONDS = 12.0  # without an explicit `until`
MAX_UNTIL_SECONDS = 25.0


def probe(path: Path) -> float:
    out = subprocess.run(
        ["ffprobe", "-v", "error", "-show_entries", "format=duration", "-of", "json", str(path)],
        capture_output=True,
        text=True,
        check=True,
    )
    return float(json.loads(out.stdout)["format"]["duration"])


def extract_audio(video: Path, out: Path, rate: int, channels: int = 1) -> np.ndarray:
    subprocess.run(
        ["ffmpeg", "-nostdin", "-loglevel", "error", "-y", "-i", str(video), "-vn", "-ac", str(channels), "-ar", str(rate), "-c:a", "pcm_f32le", str(out)],
        check=True,
    )
    audio, _ = sf.read(out, dtype="float32")
    return audio


def transcribe(video: Path, cache: Path) -> list[dict]:
    """Words with start and end in seconds, cached by file content."""
    digest = hashlib.sha256(video.read_bytes()).hexdigest()[:20]
    path = cache / f"{digest}.json"
    if path.exists():
        return json.loads(path.read_text())
    cache.mkdir(parents=True, exist_ok=True)
    audio = extract_audio(video, cache / f"{digest}-16k.wav", 16000)
    segments, _ = _whisper().transcribe(audio, language="en", word_timestamps=True, vad_filter=True)
    words = [{"text": w.word.strip(), "start": float(w.start), "end": float(w.end)} for s in segments for w in (s.words or []) if w.word.strip()]
    path.write_text(json.dumps(words))
    return words


def transcript_text(words: list[dict]) -> str:
    """Readable transcript with a timestamp at each sentence, for the planner."""
    lines, cur, start = [], [], None
    for w in words:
        if start is None:
            start = w["start"]
        cur.append(w["text"])
        if w["text"].endswith((".", "?", "!")):
            lines.append(f"[{start:6.1f}s] {' '.join(cur)}")
            cur, start = [], None
    if cur:
        lines.append(f"[{start:6.1f}s] {' '.join(cur)}")
    return "\n".join(lines)


def build(plan: dict, words_sec: list[dict], seconds: float, fps: int) -> tuple[dict, list[dict]]:
    words = [{"text": w["text"], "start": int(round(w["start"] * fps)), "end": int(round(w["end"] * fps))} for w in words_sec]
    total = int(seconds * fps)
    scenes = plan["scenes"]

    # Anchor every scene to the first match of its phrase after the previous one.
    anchors: list[int] = []
    pointer = 0
    problems = []
    for i, scene in enumerate(scenes):
        hit = _find_phrase(words, scene["at"], pointer)
        if hit is None:
            problems.append(f"scene {i + 1} ({scene['type']}): phrase {scene['at']!r} not found after the previous scene")
            anchors.append(-1)
            continue
        anchors.append(hit)
        pointer = hit + 1
    if problems:
        raise specs.SpecError("Plan does not match the transcript:\n  - " + "\n  - ".join(problems))

    starts = [max(0, words[a]["start"] - LEAD_FRAMES) for a in anchors]
    out_scenes, events = [], []
    rng = random.Random(7)
    for i, scene in enumerate(scenes):
        start = starts[i]
        limit = starts[i + 1] if i + 1 < len(scenes) else total
        end = min(limit, start + int(MAX_SECONDS * fps))
        if scene.get("until"):
            end = min(limit, start + int(MAX_UNTIL_SECONDS * fps))
            hit = _find_phrase(words, scene["until"], anchors[i])
            if hit is not None:
                last = hit + len(scene["until"].split()) - 1
                end = min(limit, words[min(last, len(words) - 1)]["end"] + int(0.5 * fps))
        end = max(end, min(limit, start + int(MIN_SECONDS * fps)))
        duration = end - start
        rel = [{**w, "start": w["start"] - start, "end": w["end"] - start} for w in words if start <= w["start"] < end]
        cues = resolve_cues(scene, rel, duration)
        # Open the graphic just before its first element, not on an empty card.
        if cues and cues[0] > 18 and scene["type"] not in ("hook", "statement", "stat"):
            delta = cues[0] - 12
            start += delta
            duration -= delta
            cues = [c - delta for c in cues]
            rel = [{**w, "start": w["start"] - delta, "end": w["end"] - delta} for w in rel if w["start"] >= delta]
        extra = scene_extra(scene, cues)
        t0 = start / fps
        events.append({"t": max(t0 - 0.12, 0), "kind": "whoosh", "gain": 0.35})
        events += scene_events(scene, cues, extra, t0, duration, fps, rng)
        if i + 1 == len(scenes) or starts[i + 1] - end > 20:
            events.append({"t": end / fps - 0.1, "kind": "whoosh", "gain": 0.22})
        out_scenes.append(
            {
                "type": scene["type"],
                "data": {k: v for k, v in scene.items() if k not in ("type", "at", "until")},
                "from": start,
                "duration": duration,
                "cues": cues,
                "words": rel,
                "extra": extra,
                "captions": True,
            }
        )

    width, height = specs.FORMATS[plan["format"]]
    timeline = {
        "title": plan["title"],
        "fps": fps,
        "width": width,
        "height": height,
        "format": plan["format"],
        "theme": plan["theme"],
        "durationInFrames": total,
        "audio": "mix.wav",
        "video": "speaker.mp4",
        "focus": plan.get("focus", [0.5, 0.4]),
        "handle": plan.get("handle"),
        "scenes": out_scenes,
        "words": words,
    }
    return timeline, events
