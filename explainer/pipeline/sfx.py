"""Sound effects, triggered by timeline events: samples from your sound
library where you have them, synthesised otherwise."""

from __future__ import annotations

import numpy as np

from .dsp import SR, bandpass, highpass, lowpass, place, reverb, stereo, t_axis

PENTATONIC = [0, 2, 4, 7, 9]


def whoosh(seconds: float = 0.5, peak: float = 0.16) -> np.ndarray:
    t = t_axis(seconds)
    noise = np.random.default_rng(5).standard_normal(len(t))
    # Sweep a band through the noise by crossfading fixed bands.
    centers = np.geomspace(250, 5000, 7)
    pos = np.clip(t / peak, 0, 1) * 0.75 + np.clip((t - peak) / (seconds - peak), 0, 1) * -0.45
    out = np.zeros_like(t)
    for k, c in enumerate(centers):
        weight = np.exp(-(((pos * (len(centers) - 1)) - k) ** 2) / 1.2)
        out += bandpass(noise, c / 1.5, c * 1.5) * weight
    env = np.where(t < peak, (t / peak) ** 2.2, np.exp(-(t - peak) * 9))
    pan = np.clip(t / seconds * 1.6 - 0.8, -0.8, 0.8)
    a = (pan + 1) * np.pi / 4
    sig = out * env
    return np.stack([sig * np.cos(a), sig * np.sin(a)], axis=1)


def pop(step: int = 0, base_hz: float = 660) -> np.ndarray:
    t = t_axis(0.18)
    semis = PENTATONIC[step % 5] + 12 * (step // 5)
    f = base_hz * 2 ** (semis / 12) * (1 + 0.5 * np.exp(-t * 70))
    tone = np.sin(2 * np.pi * np.cumsum(f) / SR) + 0.2 * np.sin(4 * np.pi * np.cumsum(f) / SR)
    env = np.exp(-t * 30) * np.minimum(1, t / 0.002)
    click = highpass(np.random.default_rng(step).standard_normal(len(t)), 3000) * np.exp(-t * 600) * 0.3
    return reverb(stereo((tone * env + click) * 0.8), 0.25, 1.0)


def thud() -> np.ndarray:
    t = t_axis(0.4)
    f = 50 + 70 * np.exp(-t * 25)
    body = np.sin(2 * np.pi * np.cumsum(f) / SR) * np.exp(-t * 10)
    return stereo(np.tanh(body * 1.5))


def impact() -> np.ndarray:
    t = t_axis(1.8)
    f = 36 + 90 * np.exp(-t * 14)
    boom = np.sin(2 * np.pi * np.cumsum(f) / SR) * np.exp(-t * 3.2)
    noise = lowpass(np.random.default_rng(9).standard_normal(len(t)), 1800) * np.exp(-t * 14) * 0.5
    hit = np.tanh((boom + noise) * 1.8)
    return reverb(stereo(hit), 0.45, 2.2, 3500)


def riser(seconds: float) -> np.ndarray:
    seconds = max(seconds, 0.3)
    t = t_axis(seconds)
    shape = (t / seconds) ** 2.5
    f = 220 * 2 ** (2.5 * t / seconds)
    tone = np.sin(2 * np.pi * np.cumsum(f) / SR) * 0.35
    noise = highpass(np.random.default_rng(4).standard_normal(len(t)), 2500) * 0.25
    return reverb(stereo((tone + noise) * shape), 0.4, 1.5)


def key(seed: float) -> np.ndarray:
    rng = np.random.default_rng(int(seed * 1e6))
    t = t_axis(0.05)
    center = 2200 + 2500 * seed
    click = bandpass(rng.standard_normal(len(t)), center / 1.4, center * 1.4) * np.exp(-t * 320)
    thock = np.sin(2 * np.pi * (170 + 60 * seed) * t) * np.exp(-t * 160) * 0.5
    return stereo((click + thock) * 0.8, (seed - 0.5) * 0.5)


def blip() -> np.ndarray:
    t = t_axis(0.12)
    tone = np.sin(2 * np.pi * 1320 * t) + 0.3 * np.sin(2 * np.pi * 2640 * t)
    return reverb(stereo(tone * np.exp(-t * 40) * np.minimum(1, t / 0.003) * 0.6), 0.2, 0.8)


def chime(root_hz: float) -> np.ndarray:
    t = t_axis(2.6)
    out = np.zeros_like(t)
    base = root_hz * 4
    while base > 1400:
        base /= 2
    for k, ratio in enumerate((1, 1.5, 2, 3)):
        delay = int(k * 0.05 * SR)
        tt = t[: len(t) - delay]
        f = base * ratio
        bell = np.sin(2 * np.pi * f * tt + 1.2 * np.exp(-tt * 3) * np.sin(2 * np.pi * f * 3.5 * tt))
        out[delay:] += bell * np.exp(-tt * 2.2) * (0.6 / (k + 1))
    return reverb(stereo(out * np.minimum(1, t / 0.002)), 0.5, 2.5)


def render(events: list[dict], seconds: float, root_hz: float, library=None) -> np.ndarray:
    """Mix every event into one stereo track. Kinds with files in the sound
    library use them; everything else is synthesised."""
    out = np.zeros((int(seconds * SR), 2))
    cache: dict[tuple, np.ndarray] = {}
    counts: dict[str, int] = {}
    for e in events:
        kind = e["kind"]
        if library is not None and library.has(kind):
            index = counts.get(kind, 0) + (e.get("step", 0) if kind == "pop" else 0)
            counts[kind] = counts.get(kind, 0) + 1
            sound = library.pick(kind, index)
            at = e["t"]
            if kind == "riser":
                # Real risers are cut to end on the hit rather than start on it.
                at = e["t"] + e.get("dur", 1.0) - len(sound) / SR
            place(out, sound, at, e.get("gain", 1.0))
            continue
        if kind == "pop":
            k = ("pop", e.get("step", 0))
            if k not in cache:
                cache[k] = pop(e.get("step", 0), base_hz=root_hz * 4)
            sound = cache[k]
        elif kind == "key":
            sound = key(e.get("seed", 0.5))
        elif kind == "riser":
            sound = riser(e.get("dur", 1.0))
        elif kind == "chime":
            sound = cache.setdefault(("chime",), chime(root_hz))
        else:
            fn = {"whoosh": whoosh, "thud": thud, "impact": impact, "blip": blip}[kind]
            sound = cache.setdefault((kind,), fn())
        place(out, sound, e["t"], e.get("gain", 1.0))
    return out.astype(np.float32)
