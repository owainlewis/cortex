"""Narration: neural TTS with Kokoro, word timing with Whisper.

Each scene is synthesised separately so scene boundaries are exact. Whisper
then recovers per-word timestamps, which are mapped back onto the words as
written in the spec so captions show the author's spelling ("O(n)") while the
voice says the pronounced form ("O of n").
"""

from __future__ import annotations

import difflib
import hashlib
import json
import os
import re
from dataclasses import dataclass
from functools import lru_cache
from pathlib import Path

import numpy as np
import soundfile as sf
from scipy.signal import resample_poly

ROOT = Path(__file__).resolve().parent.parent
MODELS = ROOT / "models"
TTS_RATE = 24000


@dataclass
class Narration:
    audio: np.ndarray  # mono float32 at TTS_RATE, silence trimmed
    words: list[dict]  # {"text", "start", "end"} in seconds

    @property
    def seconds(self) -> float:
        return len(self.audio) / TTS_RATE


@lru_cache(maxsize=1)
def _kokoro():
    from kokoro_onnx import Kokoro

    model = MODELS / "kokoro-v1.0.onnx"
    voices = MODELS / "voices-v1.0.bin"
    if not model.exists() or not voices.exists():
        raise SystemExit("Kokoro model files are missing. Run ./setup.sh first.")
    return Kokoro(str(model), str(voices))


@lru_cache(maxsize=1)
def _whisper():
    from faster_whisper import WhisperModel

    # small.en is noticeably more accurate on real recordings than base.en.
    name = os.environ.get("EXPLAINER_WHISPER", "small.en")
    return WhisperModel(name, device="cpu", compute_type="int8", download_root=str(MODELS / "whisper"))


def voices() -> list[str]:
    return sorted(_kokoro().get_voices())


def _norm(word: str) -> str:
    return re.sub(r"[^a-z0-9]", "", word.lower())


def spoken_form(narration: str, pronounce: dict[str, str]) -> tuple[list[str], list[list[str]]]:
    """Split narration into display tokens and the words spoken for each."""
    display = narration.split()
    spoken: list[list[str]] = []
    for token in display:
        core = token.strip(".,!?;:\"'()")
        lead = token[: token.find(core)] if core else ""
        trail = token[len(lead) + len(core):] if core else ""
        # Keys may match the bare word or the token with its punctuation.
        if token in pronounce:
            spoken.append(pronounce[token].split())
        elif core in pronounce:
            words = pronounce[core].split()
            words[-1] += trail
            spoken.append(words)
        else:
            spoken.append([token])
    return display, spoken


def _trim(audio: np.ndarray, threshold_db: float = -42.0, keep: float = 0.03) -> np.ndarray:
    frame = int(TTS_RATE * 0.01)
    n = len(audio) // frame
    if n == 0:
        return audio
    rms = np.sqrt(np.mean(audio[: n * frame].reshape(n, frame) ** 2, axis=1) + 1e-12)
    loud = np.where(20 * np.log10(rms) > threshold_db)[0]
    if len(loud) == 0:
        return audio
    pad = int(keep * TTS_RATE)
    start = max(loud[0] * frame - pad, 0)
    end = min((loud[-1] + 1) * frame + pad, len(audio))
    return audio[start:end]


def _tighten(audio: np.ndarray, max_gap: float = 0.26, floor_db: float = -36.0) -> np.ndarray:
    """Shorten long pauses, the way an editor jump-cuts dead air.

    TTS leaves up to a second of silence at commas; social pacing wants a
    quarter of that. Each pause longer than `max_gap` keeps its first and
    last halves of `max_gap` and loses the middle, joined with a short
    crossfade.
    """
    hop = int(TTS_RATE * 0.01)
    n = len(audio) // hop
    if n < 3:
        return audio
    rms = np.sqrt(np.mean(audio[: n * hop].reshape(n, hop) ** 2, axis=1) + 1e-12)
    level = 20 * np.log10(rms / (rms.max() + 1e-12))
    silent = level < floor_db
    keep = int(max_gap / 2 * TTS_RATE)
    fade = int(0.01 * TTS_RATE)
    pieces, cursor, i = [], 0, 0
    while i < n:
        if not silent[i]:
            i += 1
            continue
        j = i
        while j < n and silent[j]:
            j += 1
        a, b = i * hop, j * hop
        if (b - a) > 2 * keep + fade:
            cut_from, cut_to = a + keep, b - keep
            head = audio[cursor:cut_from].copy()
            tail = audio[cut_to : cut_to + fade]
            ramp = np.linspace(1, 0, min(fade, len(head), len(tail)))
            if len(ramp):
                head[-len(ramp):] = head[-len(ramp):] * ramp + tail[: len(ramp)] * (1 - ramp)
            pieces.append(head)
            cursor = cut_to + len(ramp)
        i = j
    pieces.append(audio[cursor:])
    return np.concatenate(pieces)


def _align(display: list[str], spoken: list[list[str]], audio: np.ndarray) -> list[dict]:
    flat = [w for group in spoken for w in group]
    owner = [i for i, group in enumerate(spoken) for _ in group]
    audio16 = resample_poly(audio, 2, 3).astype(np.float32)
    segments, _ = _whisper().transcribe(
        audio16,
        language="en",
        word_timestamps=True,
        initial_prompt=" ".join(flat),
        condition_on_previous_text=False,
        vad_filter=False,
    )
    heard = [w for s in segments for w in (s.words or [])]

    starts: list[float | None] = [None] * len(flat)
    ends: list[float | None] = [None] * len(flat)
    matcher = difflib.SequenceMatcher(a=[_norm(w) for w in flat], b=[_norm(w.word) for w in heard], autojunk=False)
    for block in matcher.get_matching_blocks():
        for k in range(block.size):
            w = heard[block.b + k]
            starts[block.a + k] = float(w.start)
            ends[block.a + k] = float(w.end)

    # Words Whisper missed or heard differently get a share of the gap
    # between their matched neighbours, weighted by length.
    total = len(audio) / TTS_RATE
    i = 0
    while i < len(flat):
        if starts[i] is not None:
            i += 1
            continue
        j = i
        while j < len(flat) and starts[j] is None:
            j += 1
        left = ends[i - 1] if i > 0 else 0.0
        right = starts[j] if j < len(flat) else total
        weights = [max(len(flat[k]), 2) for k in range(i, j)]
        span = max(right - left, 0.05 * (j - i))
        t = left
        for k, wgt in zip(range(i, j), weights):
            dur = span * wgt / sum(weights)
            starts[k], ends[k] = t, t + dur
            t += dur
        i = j

    out = []
    for idx, text in enumerate(display):
        ks = [k for k, o in enumerate(owner) if o == idx]
        out.append({"text": text, "start": min(starts[k] for k in ks), "end": max(ends[k] for k in ks)})
    # Keep timings monotonic; Whisper occasionally overlaps neighbours.
    for a, b in zip(out, out[1:]):
        if b["start"] < a["start"]:
            b["start"] = a["start"]
        if a["end"] > b["start"]:
            a["end"] = b["start"]
        if b["end"] < b["start"]:
            b["end"] = b["start"]
    return out


def narrate(text: str, voice: str, speed: float, pronounce: dict[str, str], cache: Path) -> Narration:
    display, spoken = spoken_form(text, pronounce)
    spoken_text = " ".join(w for group in spoken for w in group)
    key = hashlib.sha256(json.dumps([spoken_text, display, voice, speed, 3]).encode()).hexdigest()[:20]
    wav, meta = cache / f"{key}.wav", cache / f"{key}.json"
    if wav.exists() and meta.exists():
        audio, _ = sf.read(wav, dtype="float32")
        return Narration(audio, json.loads(meta.read_text()))

    audio, rate = _kokoro().create(spoken_text, voice=voice, speed=speed, lang="en-us")
    assert rate == TTS_RATE, rate
    audio = _tighten(_trim(np.asarray(audio, dtype=np.float32)))
    words = _align(display, spoken, audio)
    cache.mkdir(parents=True, exist_ok=True)
    sf.write(wav, audio, TTS_RATE)
    meta.write_text(json.dumps(words))
    return Narration(audio, words)


def estimate(text: str, speed: float) -> Narration:
    """Draft timing without TTS: about 2.7 words per second."""
    display = text.split()
    t = 0.0
    words = []
    for token in display:
        dur = (0.16 + 0.045 * len(token)) / speed
        words.append({"text": token, "start": t, "end": t + dur})
        t += dur + (0.22 if re.search(r"[.!?]$", token) else 0.05 if re.search(r"[,;:]$", token) else 0.0)
    return Narration(np.zeros(int(t * TTS_RATE), dtype=np.float32), words)
