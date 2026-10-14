"""Turn a spec plus narration into the frame-accurate timeline the renderer
plays, and the list of sound events the audio mix needs.

The timeline is the single source of truth for timing: React only reads it,
so picture and sound always agree.
"""

from __future__ import annotations

import random
import re

from . import spec as specs
from .voice import Narration

LEAD = 0.12  # silence before each scene's narration
TAIL = {"hook": 0.35, "outro": 1.8}
DEFAULT_TAIL = 0.5
CUE_LEAD = 3  # frames an element appears before its word is spoken
MIN_CUE_GAP = 6
TERMINAL_CPF = 1.1  # typed characters per frame

STOP = set(
    "the a an and or but of to in on at for with by from is are was were be been it its this that these those "
    "you your we our they their i my as into than then so if not no do does did can will just".split()
)


def _norm(w: str) -> str:
    return re.sub(r"[^a-z0-9]", "", w.lower())


def _find_phrase(words: list[dict], phrase: str, after: int) -> int | None:
    target = [_norm(w) for w in phrase.split() if _norm(w)]
    if not target:
        return None
    for i in range(after, len(words) - len(target) + 1):
        if all(_norm(words[i + k]["text"]) == t for k, t in enumerate(target)):
            return i
    return None


def _find_keyword(words: list[dict], text: str, after: int) -> int | None:
    """First narrated word that shares a 5-letter stem with a content word of
    `text`, so "Inserting" in the narration matches an item called "Insert"."""
    keys = {_norm(w)[:5] for w in text.split() if _norm(w) and _norm(w) not in STOP and len(_norm(w)) > 2}
    if not keys:
        return None
    for i in range(after, len(words)):
        if _norm(words[i]["text"])[:5] in keys:
            return i
    return None


def resolve_cues(scene: dict, words: list[dict], duration: int) -> list[int]:
    """Frames, relative to the scene, at which each element is revealed."""
    targets = specs.cue_targets(scene)
    if not targets:
        return []
    frames: list[int | None] = []
    pointer = 0
    for phrase, text in targets:
        hit = None
        if phrase:
            hit = _find_phrase(words, phrase, pointer)
            if hit is None:
                print(f"  warning: cue {phrase!r} not found in narration of {scene['type']} scene; spacing evenly")
        elif text:
            hit = _find_keyword(words, text, pointer)
        if hit is None:
            frames.append(None)
        else:
            frames.append(max(words[hit]["start"] - CUE_LEAD, 4))
            pointer = hit + 1

    # Unmatched cues are spread evenly between their matched neighbours.
    lo, hi = 6, int(duration * 0.78)
    i = 0
    while i < len(frames):
        if frames[i] is not None:
            i += 1
            continue
        j = i
        while j < len(frames) and frames[j] is None:
            j += 1
        count = j - i
        right = frames[j] if j < len(frames) else hi
        if i == 0:
            # No earlier anchor: start at `lo` and step toward the right one.
            right = max(right, lo + count * MIN_CUE_GAP)
            for k in range(count):
                frames[k] = lo + (right - lo) * k / count
        else:
            left = frames[i - 1]
            for k in range(count):
                frames[i + k] = left + (max(right, left) - left) * (k + 1) / (count + 1)
        i = j

    out: list[int] = []
    for f in frames:
        f = int(f)  # type: ignore[arg-type]
        if out and f < out[-1] + MIN_CUE_GAP:
            f = out[-1] + MIN_CUE_GAP
        out.append(f)
    return out


def build(spec: dict, narrations: list[Narration], audio: str | None) -> tuple[dict, list[dict]]:
    fps = spec["fps"]
    width, height = specs.FORMATS[spec["format"]]
    scenes_out = []
    events: list[dict] = []
    all_words = []
    cursor = 0  # frames
    rng = random.Random(7)

    for index, (scene, narr) in enumerate(zip(spec["scenes"], narrations)):
        kind = scene["type"]
        lead = 0.0 if index == 0 else LEAD
        seconds = lead + narr.seconds + TAIL.get(kind, DEFAULT_TAIL)
        duration = max(int(round(seconds * fps)), fps)
        words = [
            {
                "text": w["text"],
                "start": int(round((lead + w["start"]) * fps)),
                "end": int(round((lead + w["end"]) * fps)),
            }
            for w in narr.words
        ]
        cues = resolve_cues(scene, words, duration)
        extra: dict[str, float] = {}
        t0 = cursor / fps

        if kind == "stat":
            extra["countFrames"] = 32
        if kind == "code":
            chars = len(str(scene["code"]))
            start = 6
            natural = start + chars / 2.2
            end = natural
            if cues:
                end = min(natural, cues[0] - 4)
            end = max(end, start + chars / 6)
            if scene.get("reveal") == "instant":
                end = start
            extra["typeStart"], extra["typeEnd"] = start, int(end)
        if kind == "terminal":
            extra["charsPerFrame"] = TERMINAL_CPF
        if kind == "array":
            extra["shiftStagger"] = 2

        # Sound design follows the same cues the picture uses.
        if index == 0:
            events.append({"t": 0.0, "kind": "impact", "gain": 0.8})
        else:
            events.append({"t": max(t0 - 0.16, 0), "kind": "whoosh", "gain": 0.45})
        if kind in ("list", "flow", "bars", "tree"):
            for k, c in enumerate(cues):
                events.append({"t": t0 + c / fps, "kind": "pop", "step": k, "gain": 0.55})
        if kind == "compare":
            for k, c in enumerate(cues):
                events.append({"t": t0 + c / fps, "kind": "pop", "step": k * 2, "gain": 0.6})
            if len(cues) > 1:
                events.append({"t": t0 + (cues[1] - 4) / fps, "kind": "thud", "gain": 0.5})
        if kind == "stat":
            start = cues[0] if cues else 6
            # Build into the number, hit as it lands.
            events.append({"t": t0 + (start - 20) / fps, "kind": "riser", "dur": 20 / fps, "gain": 0.45})
            events.append({"t": t0 + start / fps, "kind": "impact", "gain": 0.6})
        if kind == "code":
            f = extra["typeStart"]
            while f < extra["typeEnd"]:
                events.append({"t": t0 + f / fps, "kind": "key", "seed": rng.random(), "gain": 0.32})
                f += rng.choice([1, 2, 2, 3])
            for c in cues:
                events.append({"t": t0 + c / fps, "kind": "blip", "gain": 0.45})
        if kind == "terminal":
            for k, c in enumerate(cues):
                cmd = str(scene["lines"][k].get("cmd", ""))
                n = len(cmd)
                for ch in range(0, n, 1):
                    if rng.random() < 0.8:
                        events.append({"t": t0 + (c + ch / TERMINAL_CPF) / fps, "kind": "key", "seed": rng.random(), "gain": 0.32})
                if scene["lines"][k].get("out"):
                    events.append({"t": t0 + (c + n / TERMINAL_CPF + 5) / fps, "kind": "blip", "gain": 0.35})
        if kind == "array":
            shift = cues[0] if cues else 20
            n = len(str(scene["text"])) - min(int(scene.get("insertAt", 0)), len(str(scene["text"])))
            for k in range(n):
                events.append({"t": t0 + (shift + k * 2) / fps, "kind": "key", "seed": rng.random(), "gain": 0.4})
            events.append({"t": t0 + (shift + n * 2 + 6) / fps, "kind": "pop", "step": 2, "gain": 0.6})
        if kind == "outro":
            events.append({"t": t0 + min(duration * 0.45, 24) / fps, "kind": "chime", "gain": 0.6})

        captions = scene.get("captions", kind not in ("hook", "outro"))
        scenes_out.append(
            {
                "type": kind,
                "data": {k: v for k, v in scene.items() if k not in ("narration", "type")},
                "from": cursor,
                "duration": duration,
                "cues": cues,
                "words": words,
                "extra": extra,
                "captions": bool(captions),
            }
        )
        all_words += [{**w, "start": w["start"] + cursor, "end": w["end"] + cursor} for w in words]
        cursor += duration

    timeline = {
        "title": spec["title"],
        "fps": fps,
        "width": width,
        "height": height,
        "format": spec["format"],
        "theme": spec["theme"],
        "durationInFrames": cursor,
        "audio": audio,
        "handle": spec.get("handle"),
        "scenes": scenes_out,
        "words": all_words,
    }
    return timeline, events


def voice_offsets(timeline: dict) -> list[float]:
    """Seconds at which each scene's narration starts."""
    fps = timeline["fps"]
    return [s["from"] / fps + (0.0 if i == 0 else LEAD) for i, s in enumerate(timeline["scenes"])]


def srt(timeline: dict) -> str:
    """Sentence-ish caption blocks for platforms that take an SRT upload."""
    fps = timeline["fps"]

    def stamp(frame: int) -> str:
        ms = int(frame / fps * 1000)
        return f"{ms // 3600000:02}:{ms // 60000 % 60:02}:{ms // 1000 % 60:02},{ms % 1000:03}"

    blocks, cur = [], []
    for w in timeline["words"]:
        cur.append(w)
        if re.search(r"[.!?]$", w["text"]) or len(cur) >= 8:
            blocks.append(cur)
            cur = []
    if cur:
        blocks.append(cur)
    lines = []
    for i, b in enumerate(blocks, 1):
        lines += [str(i), f"{stamp(b[0]['start'])} --> {stamp(b[-1]['end'])}", " ".join(w["text"] for w in b), ""]
    return "\n".join(lines)
