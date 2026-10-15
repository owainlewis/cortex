"""Final mix: voice chain, music ducking, effects, loudness.

Stems are written as WAVs and mixed with ffmpeg so the voice gets a proper
broadcast chain and the music ducks under speech via a sidechain compressor.
The result is normalised to -14 LUFS integrated, -1 dBTP, which is what
TikTok, Reels, Shorts and YouTube normalise to.
"""

from __future__ import annotations

import json
import subprocess
from pathlib import Path

import numpy as np
import soundfile as sf
from scipy.signal import resample_poly

from .dsp import SR, place, to_rms
from .voice import TTS_RATE

TARGET_LUFS = -14.0
TRUE_PEAK = -1.0

VOICE_CHAIN = (
    "highpass=f=75,"
    "equalizer=f=250:t=q:w=1.2:g=-2,"
    "equalizer=f=3200:t=q:w=1.0:g=2.5,"
    "equalizer=f=9000:t=h:w=0.7:g=1.5,"
    "acompressor=threshold=-22dB:ratio=3:attack=4:release=90:makeup=3dB,"
    "deesser=i=0.35"
)


def voice_stem(narrations, offsets: list[float], seconds: float) -> np.ndarray:
    out = np.zeros(int(seconds * SR))
    for narr, at in zip(narrations, offsets):
        up = resample_poly(narr.audio, SR, TTS_RATE)
        place(out, up, at)
    return out.astype(np.float32)


def _ffmpeg(args: list[str]) -> str:
    proc = subprocess.run(["ffmpeg", "-hide_banner", "-nostdin", "-y", *args], capture_output=True, text=True)
    if proc.returncode != 0:
        raise RuntimeError(f"ffmpeg failed:\n{proc.stderr[-3000:]}")
    return proc.stderr


def mix(voice: np.ndarray, music: np.ndarray, effects: np.ndarray, out: Path, work: Path, music_db: float = -13.0) -> dict:
    work.mkdir(parents=True, exist_ok=True)
    n = max(len(voice), len(music), len(effects))

    def fit(x):
        if len(x) < n:
            pad = np.zeros((n - len(x),) + x.shape[1:], dtype=x.dtype)
            x = np.concatenate([x, pad])
        return x

    voice_p, music_p, sfx_p = work / "voice.wav", work / "music.wav", work / "sfx.wav"
    sf.write(voice_p, fit(to_rms(voice, -21.0) if np.any(voice) else voice), SR, subtype="FLOAT")
    sf.write(music_p, fit(music), SR, subtype="FLOAT")
    sf.write(sfx_p, fit(effects * 0.5), SR, subtype="FLOAT")

    graph = (
        f"[0:a]{VOICE_CHAIN},aformat=channel_layouts=stereo,asplit=2[v][key];"
        f"[1:a]volume={music_db}dB[m];"
        "[m][key]sidechaincompress=threshold=0.02:ratio=8:attack=20:release=500:makeup=1[duck];"
        "[2:a]volume=-3dB[fx];"
        "[v][duck][fx]amix=inputs=3:normalize=0,alimiter=limit=0.89:attack=3:release=60[mix]"
    )
    pre = work / "premaster.wav"
    _ffmpeg(["-i", str(voice_p), "-i", str(music_p), "-i", str(sfx_p), "-filter_complex", graph, "-map", "[mix]", "-c:a", "pcm_f32le", str(pre)])

    # Two-pass loudness normalisation for an exact integrated target.
    measure = _ffmpeg(["-i", str(pre), "-af", f"loudnorm=I={TARGET_LUFS}:TP={TRUE_PEAK}:LRA=11:print_format=json", "-f", "null", "-"])
    stats = json.loads(measure[measure.rindex("{") : measure.rindex("}") + 1])
    norm = (
        f"loudnorm=I={TARGET_LUFS}:TP={TRUE_PEAK}:LRA=11:"
        f"measured_I={stats['input_i']}:measured_TP={stats['input_tp']}:"
        f"measured_LRA={stats['input_lra']}:measured_thresh={stats['input_thresh']}:"
        f"offset={stats['target_offset']}:linear=true"
    )
    _ffmpeg(["-i", str(pre), "-af", norm, "-ar", str(SR), "-c:a", "pcm_s24le", str(out)])
    check = _ffmpeg(["-i", str(out), "-af", "loudnorm=print_format=json", "-f", "null", "-"])
    final = json.loads(check[check.rindex("{") : check.rindex("}") + 1])
    return {"lufs": float(final["input_i"]), "true_peak": float(final["input_tp"])}
