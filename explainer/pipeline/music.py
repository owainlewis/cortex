"""Procedural music bed.

Writes an original, royalty-free track for every video, cut to its exact
length: warm detuned pads, a sub bass, FM plucks through a ping-pong delay,
and a light drum kit with sidechain pumping. Drums enter after the hook and
drop out for the outro so the arrangement follows the story.
"""

from __future__ import annotations

import numpy as np

from .dsp import SR, fade, highpass, lowpass, midi_hz, reverb, stereo, t_axis, to_rms

# Each mood: tempo and a four-chord loop of (bass note, pad voicing) in MIDI.
MOODS = {
    "focus": {
        "bpm": 96,
        "root": 57,
        "chords": [
            (33, [60, 64, 67, 71]),  # Am9
            (29, [57, 60, 64, 67]),  # Fmaj9
            (36, [55, 60, 62, 64]),  # Cadd9
            (31, [55, 59, 62, 64]),  # G6
        ],
    },
    "uplift": {
        "bpm": 112,
        "root": 62,
        "chords": [
            (38, [54, 57, 62, 64]),  # Dadd9
            (33, [52, 57, 61, 64]),  # A
            (35, [54, 57, 59, 62]),  # Bm7
            (31, [54, 55, 59, 62]),  # Gmaj7
        ],
    },
    "dark": {
        "bpm": 88,
        "root": 52,
        "chords": [
            (28, [55, 59, 64, 66]),  # Em(add9)
            (36, [55, 59, 60, 64]),  # Cmaj7
            (33, [55, 57, 60, 64]),  # Am7
            (35, [54, 57, 59, 63]),  # B7sus
        ],
    },
}

ARP = [0, 2, 1, 3, 2, 1, 3, 2]  # chord tone order, eighth notes


def _saw(freq: float, seconds: float, rng: np.random.Generator, top_hz: float = 5000) -> np.ndarray:
    """Band-limited supersaw: three detuned additive saws."""
    t = t_axis(seconds)
    out = np.zeros_like(t)
    for cents in (-9, 0, 9):
        f = freq * 2 ** (cents / 1200)
        phase = rng.random() * 2 * np.pi
        for h in range(1, int(top_hz // f) + 1):
            out += np.sin(2 * np.pi * f * h * t + phase * h) / h
    return out / 3


def _pad_chord(notes: list[int], seconds: float, seed: int) -> np.ndarray:
    rng = np.random.default_rng(seed)
    total = seconds + 1.2  # release overlaps the next chord
    t = t_axis(total)
    env = np.minimum(1, t / 0.5) * np.where(t < seconds, 1.0, np.exp(-(t - seconds) * 4))
    left = sum(_saw(midi_hz(n), total, rng) for n in notes) * env
    right = sum(_saw(midi_hz(n), total, rng) for n in notes) * env
    return np.stack([left, right], axis=1) / len(notes)


def _pluck(freq: float, seconds: float = 0.9) -> np.ndarray:
    t = t_axis(seconds)
    index = 1.6 * np.exp(-t * 9)
    env = np.exp(-t * 6.5) * (1 - np.exp(-t * 500))
    return np.sin(2 * np.pi * freq * t + index * np.sin(2 * np.pi * freq * 2 * t)) * env


def _kick() -> np.ndarray:
    t = t_axis(0.5)
    f = 46 + 120 * np.exp(-t * 32)
    body = np.sin(2 * np.pi * np.cumsum(f) / SR) * np.exp(-t * 7)
    click = highpass(np.random.default_rng(1).standard_normal(len(t)), 2500) * np.exp(-t * 350) * 0.2
    return np.tanh(1.6 * (body + click)) / np.tanh(1.6)


def _clap() -> np.ndarray:
    t = t_axis(0.35)
    noise = np.random.default_rng(2).standard_normal(len(t))
    env = np.zeros_like(t)
    for k, off in enumerate((0.0, 0.011, 0.022)):
        env += (t >= off) * np.exp(-np.clip(t - off, 0, None) * (180 if k < 2 else 16))
    return np.asarray(highpass(lowpass(noise, 5200), 900) * env * 0.6)


def _hat(open_: bool = False) -> np.ndarray:
    t = t_axis(0.3 if open_ else 0.07)
    noise = np.random.default_rng(3 + open_).standard_normal(len(t))
    return np.asarray(highpass(noise, 7500) * np.exp(-t * (14 if open_ else 70)))


def render(seconds: float, mood: str = "focus", bpm: float | None = None, drums_in: float = 0.0, drums_out: float | None = None) -> tuple[np.ndarray, float]:
    """Return a stereo music bed of `seconds` length and its key root in Hz."""
    m = MOODS[mood]
    bpm = bpm or m["bpm"]
    beat = 60 / bpm
    step = beat / 4  # sixteenth note
    bar = beat * 4
    length = seconds + 2.5
    n = int(length * SR)
    pad = np.zeros((n, 2))
    bass = np.zeros(n)
    arp = np.zeros((n, 2))
    drums = np.zeros((n, 2))
    drums_out = seconds if drums_out is None else drums_out
    # Snap drum entry and exit to the bar grid so they land on a downbeat.
    drums_in = round(drums_in / bar) * bar
    drums_out = round(drums_out / bar) * bar

    chords = m["chords"]
    pads = [_pad_chord(v, bar, seed=i) for i, (_, v) in enumerate(chords)]
    kick, clap, hat, ohat = _kick(), _clap(), _hat(), _hat(True)
    plucks: dict[int, np.ndarray] = {}
    rng = np.random.default_rng(11)
    kicks: list[float] = []

    def put(dst, src, at, gain=1.0):
        i = int(at * SR)
        if i >= len(dst):
            return
        k = min(len(src), len(dst) - i)
        dst[i : i + k] += src[:k] * gain

    bars = int(np.ceil(length / bar))
    for b in range(bars):
        t0 = b * bar
        root, voicing = chords[b % len(chords)]
        put(pad, pads[b % len(chords)], t0)
        # Sub bass with a small octave hop at the end of the bar.
        for s, dur, octave in ((0, 6, 0), (6, 2, 0), (8, 6, 0), (14, 2, 12)):
            tt = t_axis(dur * step)
            f = midi_hz(root + 12 + octave)
            env = np.minimum(1, tt / 0.01) * np.exp(-tt * 1.5) * np.minimum(1, (dur * step - tt) / 0.02)
            put(bass, (np.sin(2 * np.pi * f * tt) + 0.25 * np.sin(4 * np.pi * f * tt)) * env, t0 + s * step)
        # Plucked arpeggio, one octave above the pad, alternating sides.
        tones = sorted(voicing) + [voicing[0] + 12]
        for e, idx in enumerate(ARP):
            note = tones[idx % len(tones)] + 12
            if note not in plucks:
                plucks[note] = _pluck(midi_hz(note))
            vel = (0.9 if e % 2 == 0 else 0.6) * (0.85 + 0.15 * rng.random())
            put(arp, stereo(plucks[note], -0.45 if e % 2 == 0 else 0.45), t0 + e * 2 * step, vel)
        # Drums.
        if drums_in <= t0 < drums_out:
            for s in (0, 8) + ((10,) if b % 2 == 1 else ()):
                put(drums, stereo(kick), t0 + s * step, 0.95 if s != 10 else 0.6)
                kicks.append(t0 + s * step)
            for s in (4, 12):
                put(drums, stereo(clap, 0.05), t0 + s * step, 0.55)
            for s in range(0, 16, 2):
                swing = step * 0.12 if (s // 2) % 2 == 1 else 0
                put(drums, stereo(hat, 0.3), t0 + s * step + swing, 0.22 if s % 4 == 2 else 0.12)
            if b % 4 == 3:
                put(drums, stereo(ohat, 0.3), t0 + 14 * step, 0.18)

    # Intro: the pad opens up from a dark filter over the first few seconds.
    dark, bright = lowpass(pad, 650), lowpass(pad, 3200)
    tt = t_axis(length)[:, None]
    openness = np.clip(tt / 4.0, 0, 1) ** 1.5
    pad = dark * (1 - openness) + bright * openness

    # Sidechain pump keyed from the kick.
    pump = np.ones(n)
    tk = t_axis(0.35)
    shape = 1 - 0.55 * np.exp(-tk / 0.09)
    for k in kicks:
        i = int(k * SR)
        j = min(n, i + len(shape))
        pump[i:j] = np.minimum(pump[i:j], shape[: j - i])
    bass = lowpass(bass * pump, 400)

    # Ping-pong dotted-eighth delay on the plucks.
    delay = int(beat * 0.75 * SR)
    wet = np.zeros_like(arp)
    src = arp
    for tap in range(1, 5):
        shifted = np.zeros_like(arp)
        shifted[delay * tap :] = src[: n - delay * tap]
        side = shifted[:, ::-1] if tap % 2 else shifted
        wet += side * (0.42**tap)
    arp = lowpass(arp + wet, 5500)

    mix = (
        reverb(pad * pump[:, None], 0.5, 3.0) * 0.55
        + stereo(bass) * 0.75
        + reverb(arp, 0.6, 2.4) * 0.32
        + reverb(drums, 0.18, 1.2) * 0.62
    )
    mix = highpass(mix, 28)
    mix = np.tanh(mix * 1.3) / np.tanh(1.3)
    mix = mix[: int(seconds * SR)]
    mix = fade(mix, 0.05, 1.8)
    return to_rms(mix, -20.0).astype(np.float32), midi_hz(m["root"])
