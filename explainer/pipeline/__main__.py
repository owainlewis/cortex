"""Explainer video pipeline.

    python -m pipeline build specs/rope.json            # full render
    python -m pipeline build specs/rope.json --draft    # fast, silent, half size
    python -m pipeline build specs/rope.json --format all
    python -m pipeline footage talk.mp4                 # motion graphics over your own video
    python -m pipeline footage talk.mp4 --plan plan.json --format vertical
    python -m pipeline write "how B-trees work"         # draft a spec with Claude
    python -m pipeline voices
"""

from __future__ import annotations

import argparse
import json
import shutil
import subprocess
import sys
import time
from pathlib import Path

import numpy as np

from . import spec as specs

ROOT = Path(__file__).resolve().parent.parent


class Step:
    def __init__(self, name: str):
        self.name = name

    def __enter__(self):
        self.t = time.time()
        print(f"• {self.name}...", flush=True)
        return self

    def __exit__(self, *exc):
        if exc[0] is None:
            print(f"  done in {time.time() - self.t:.1f}s", flush=True)


def build(args: argparse.Namespace) -> None:
    from . import mix, music, render, sfx, timeline, voice

    spec = specs.load(args.spec)
    if args.theme:
        spec["theme"] = args.theme
    formats = list(specs.FORMATS) if args.format == "all" else [args.format or spec["format"]]
    out = ROOT / "out" / spec["slug"]
    public = out / "public"
    public.mkdir(parents=True, exist_ok=True)
    cache = ROOT / "out" / ".cache" / "voice"

    with Step("Narration" if not args.draft else "Estimating narration timing"):
        narrations = []
        for i, scene in enumerate(spec["scenes"]):
            pronounce = {**spec["pronounce"], **scene.get("pronounce", {})}
            if args.draft:
                narr = voice.estimate(scene["narration"], spec["speed"])
            else:
                narr = voice.narrate(scene["narration"], scene.get("voice", spec["voice"]), spec["speed"], pronounce, cache)
            narrations.append(narr)
            print(f"  {i + 1:>2}. {scene['type']:<10} {narr.seconds:5.1f}s")

    tl, events = timeline.build(spec, narrations, None if args.draft else "mix.wav")
    seconds = tl["durationInFrames"] / tl["fps"]
    print(f"  total {seconds:.1f}s, {len(spec['scenes'])} scenes")

    if not args.draft and not args.stills:
        with Step("Music and sound design"):
            fps = tl["fps"]
            scenes = tl["scenes"]
            mood = spec["music"].get("mood", "focus")
            bed, root_hz = music.render(
                seconds,
                mood,
                spec["music"].get("bpm"),
                drums_in=scenes[1]["from"] / fps if len(scenes) > 1 else 0,
                drums_out=scenes[-1]["from"] / fps,
            )
            effects = sfx.render(events, seconds, root_hz)
        with Step("Mixing"):
            stem = mix.voice_stem(narrations, timeline.voice_offsets(tl), seconds)
            loud = mix.mix(stem, bed, effects, public / "mix.wav", out / "stems")
            print(f"  loudness {loud['lufs']:.1f} LUFS, true peak {loud['true_peak']:.1f} dBTP")

    (out / "captions.srt").write_text(timeline.srt(tl))
    post = spec.get("post", {})
    tags = " ".join(f"#{t.lstrip('#')}" for t in post.get("hashtags", []))
    (out / "post.txt").write_text("\n\n".join(x for x in (spec["title"], post.get("caption", ""), tags) if x) + "\n")

    if args.stills:
        w, h = specs.FORMATS[formats[0]]
        props = out / f"timeline-{formats[0]}.json"
        props.write_text(json.dumps({**tl, "format": formats[0], "width": w, "height": h}))
        render.stills(props, public, out / "stills")
        return

    hook_end = tl["scenes"][0]["duration"] - 4
    for fmt in formats:
        w, h = specs.FORMATS[fmt]
        props = out / f"timeline-{fmt}.json"
        props.write_text(json.dumps({**tl, "format": fmt, "width": w, "height": h}))
        suffix = "-draft" if args.draft else ""
        video = out / f"{spec['slug']}-{fmt}{suffix}.mp4"
        with Step(f"Rendering {fmt} {w}x{h}" + (" (draft)" if args.draft else "")):
            render.video(props, public, video, args.draft, not args.draft)
        if not args.draft:
            with Step("Cover image"):
                render.still(props, public, out / f"cover-{fmt}.png", hook_end)
        print(f"  → {video.relative_to(ROOT)}")

    if args.sample:
        shutil.copy(out / f"timeline-{formats[0]}.json", ROOT / "src" / "sample-timeline.json")


def footage(args: argparse.Namespace) -> None:
    from . import footage as fx
    from . import mix, music, render, sfx, timeline

    video_in = Path(args.video).resolve()
    seconds = fx.probe(video_in)
    slug = specs.slugify(Path(args.video).stem)
    out = ROOT / "out" / slug
    public = out / "public"
    public.mkdir(parents=True, exist_ok=True)

    with Step("Transcribing"):
        words = fx.transcribe(video_in, ROOT / "out" / ".cache" / "transcripts")
        (out / "transcript.txt").write_text(fx.transcript_text(words) + "\n")
        print(f"  {len(words)} words, {seconds:.1f}s")

    plan_path = Path(args.plan) if args.plan else out / "plan.json"
    if not plan_path.exists():
        from . import writer

        with Step("Planning graphics with Claude"):
            drafted = writer.plan(
                fx.transcript_text(words),
                title=Path(args.video).stem,
                check_anchors=lambda d: fx.build({**specs.DEFAULTS, "format": "landscape", **d}, words, seconds, 30),
            )
            plan_path.write_text(json.dumps(drafted, indent=2) + "\n")
            print(f"  wrote {plan_path.relative_to(ROOT) if plan_path.is_relative_to(ROOT) else plan_path}; edit it and re-run to adjust")
    plan = json.loads(plan_path.read_text())
    plan.setdefault("title", slug)
    for key, value in specs.DEFAULTS.items():
        plan.setdefault(key, value)
    plan["format"] = args.format if args.format and args.format != "all" else plan.get("format", "landscape")
    if args.theme:
        plan["theme"] = args.theme
    specs.validate(plan, footage=True)

    tl, events = fx.build(plan, words, seconds, plan["fps"])
    print(f"  {len(tl['scenes'])} graphics over {seconds:.1f}s")

    speaker = public / "speaker.mp4"
    if not speaker.exists() or speaker.stat().st_mtime < video_in.stat().st_mtime:
        with Step("Preparing footage"):
            subprocess.run(
                ["ffmpeg", "-nostdin", "-loglevel", "error", "-y", "-i", str(video_in), "-an", "-c:v", "libx264", "-preset", "veryfast", "-crf", "16", "-pix_fmt", "yuv420p", str(speaker)],
                check=True,
            )

    if not args.stills:
        with Step("Sound design and mix"):
            (out / "stems").mkdir(parents=True, exist_ok=True)
            voice_audio = fx.extract_audio(video_in, out / "stems" / "source.wav", 48000)
            mood = plan["music"].get("mood", "focus")
            if plan["music"].get("enabled", True) and not args.no_music:
                bed, root_hz = music.render(seconds, mood, plan["music"].get("bpm"), drums_in=tl["scenes"][0]["from"] / tl["fps"] if tl["scenes"] else 0)
            else:
                bed, root_hz = np.zeros((int(seconds * 48000), 2), dtype=np.float32), music.midi_hz(music.MOODS[mood]["root"])
            effects = sfx.render(events, seconds, root_hz)
            loud = mix.mix(voice_audio, bed, effects, public / "mix.wav", out / "stems", music_db=-20.0)
            print(f"  loudness {loud['lufs']:.1f} LUFS, true peak {loud['true_peak']:.1f} dBTP")

    (out / "captions.srt").write_text(timeline.srt(tl))
    formats = list(specs.FORMATS) if args.format == "all" else [plan["format"]]
    for fmt in formats:
        w, h = specs.FORMATS[fmt]
        props = out / f"timeline-{fmt}.json"
        props.write_text(json.dumps({**tl, "format": fmt, "width": w, "height": h}))
        if args.stills:
            render.stills(props, public, out / f"stills-{fmt}")
            continue
        video = out / f"{slug}-{fmt}.mp4"
        with Step(f"Rendering {fmt} {w}x{h}"):
            render.video(props, public, video, False, True)
        print(f"  → {video.relative_to(ROOT)}")


def write(args: argparse.Namespace) -> None:
    from . import writer

    spec = writer.write(args.topic, args.notes)
    path = Path(args.out) if args.out else ROOT / "specs" / f"{specs.slugify(spec['title'])}.json"
    path.write_text(json.dumps(spec, indent=2) + "\n")
    print(f"Wrote {path}")


def main(argv: list[str] | None = None) -> None:
    parser = argparse.ArgumentParser(prog="explain", description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = parser.add_subparsers(dest="command", required=True)

    b = sub.add_parser("build", help="render a spec to video")
    b.add_argument("spec")
    b.add_argument("--format", choices=[*specs.FORMATS, "all"])
    b.add_argument("--theme", choices=sorted(specs.THEMES))
    b.add_argument("--draft", action="store_true", help="skip TTS and audio, render at half size")
    b.add_argument("--stills", action="store_true", help="render two stills per scene instead of the video, for design review")
    b.add_argument("--sample", action="store_true", help="also copy the timeline to src/sample-timeline.json for the studio")
    b.set_defaults(fn=build)

    f = sub.add_parser("footage", help="add motion graphics, captions and sound to your own video")
    f.add_argument("video")
    f.add_argument("--plan", help="graphics plan JSON; drafted with Claude when missing")
    f.add_argument("--format", choices=[*specs.FORMATS, "all"])
    f.add_argument("--theme", choices=sorted(specs.THEMES))
    f.add_argument("--no-music", action="store_true", help="sound effects only, no music bed")
    f.add_argument("--stills", action="store_true", help="render review stills instead of the video")
    f.set_defaults(fn=footage)

    w = sub.add_parser("write", help="draft a spec from a topic with Claude")
    w.add_argument("topic")
    w.add_argument("--notes", default="", help="facts, angle or source material to use")
    w.add_argument("--out")
    w.set_defaults(fn=write)

    v = sub.add_parser("voices", help="list narrator voices")
    v.set_defaults(fn=lambda _: print("\n".join(__import__("pipeline.voice", fromlist=["voices"]).voices())))

    c = sub.add_parser("check", help="validate a spec without rendering")
    c.add_argument("spec")
    c.set_defaults(fn=lambda a: print(f"OK: {specs.load(a.spec)['title']}"))

    args = parser.parse_args(argv)
    try:
        args.fn(args)
    except specs.SpecError as e:
        print(e, file=sys.stderr)
        sys.exit(2)


if __name__ == "__main__":
    main()
