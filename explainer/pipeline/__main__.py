"""Explainer video pipeline.

    python -m pipeline build specs/rope.json            # full render
    python -m pipeline build specs/rope.json --draft    # fast, silent, half size
    python -m pipeline build specs/rope.json --format all
    python -m pipeline footage talk.mp4                 # motion graphics over your own video
    python -m pipeline footage talk.mp4 --plan plan.json --format vertical
    python -m pipeline inbox                            # render every new video in inbox/
    python -m pipeline transcribe talk.mp4              # transcript only, for writing a plan by hand
    python -m pipeline write "how B-trees work"         # draft a spec with Claude
    python -m pipeline voices
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
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


def apply_brand(args: argparse.Namespace, doc: dict, chose_theme: bool) -> dict:
    """Load the brand and resolve theme and handle: command line, then the
    video's own setting, then the brand default."""
    from . import brand as brands

    b = brands.load(args.brand)
    if args.theme:
        doc["theme"] = args.theme
    elif not chose_theme and b.get("theme"):
        doc["theme"] = b["theme"]
    if not doc.get("handle") and b.get("handle"):
        doc["handle"] = b["handle"]
    return b


def soundtrack(b: dict, music_cfg: dict, slug: str, seconds: float, synth_kwargs: dict):
    """Music bed and sound library for a video. Uses your files when the brand's
    folders have them, the synthesiser otherwise."""
    from . import brand as brands
    from . import library, music

    cache = ROOT / "out" / ".cache" / "audio"
    sounds = library.SoundLibrary(brands.folder(b, "sounds", "../sounds"), cache)
    mood = music_cfg.get("mood", "focus")
    root_hz = music.midi_hz(music.MOODS[mood]["root"])
    music_dir = brands.folder(b, "music", "../music")
    if music_cfg.get("track"):
        track = music_dir / music_cfg["track"]
        if not track.exists():
            raise SystemExit(f"Music track not found: {track}")
        tracks = [track]
    else:
        tracks = library.music_files(music_dir, mood)
    if tracks:
        # Same video, same track; different videos spread across the library.
        track = tracks[int(hashlib.sha256(slug.encode()).hexdigest(), 16) % len(tracks)]
        print(f"  music: {track.name}")
        bed = library.music_bed(track, seconds, cache)
    else:
        print("  music: synthesised")
        bed, root_hz = music.render(seconds, mood, music_cfg.get("bpm"), **synth_kwargs)
    print(f"  sounds: {sounds.summary()}")
    return bed, root_hz, sounds


def build(args: argparse.Namespace) -> None:
    from . import brand as brands
    from . import mix, render, sfx, timeline, voice

    spec = specs.load(args.spec)
    b = apply_brand(args, spec, spec.pop("_theme_set"))
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
    tl["brand"] = brands.publish(b, public)
    seconds = tl["durationInFrames"] / tl["fps"]
    print(f"  total {seconds:.1f}s, {len(spec['scenes'])} scenes")

    if not args.draft and not args.stills:
        with Step("Music and sound design"):
            fps = tl["fps"]
            scenes = tl["scenes"]
            bed, root_hz, sounds = soundtrack(
                b,
                spec["music"],
                spec["slug"],
                seconds,
                {"drums_in": scenes[1]["from"] / fps if len(scenes) > 1 else 0, "drums_out": scenes[-1]["from"] / fps},
            )
            effects = sfx.render(events, seconds, root_hz, sounds)
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
    from . import brand as brands
    from . import footage as fx
    from . import mix, render, sfx, timeline

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
    b = apply_brand(args, plan, "theme" in json.loads(plan_path.read_text()))
    default_format = (b.get("formats") or ["landscape"])[0]
    plan["format"] = args.format if args.format and args.format != "all" else plan.get("format", default_format)
    specs.validate(plan, footage=True)

    tl, events = fx.build(plan, words, seconds, plan["fps"])
    tl["brand"] = brands.publish(b, public)
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
            if plan["music"].get("enabled", True) and not args.no_music:
                first = tl["scenes"][0]["from"] / tl["fps"] if tl["scenes"] else 0
                bed, root_hz, sounds = soundtrack(b, plan["music"], slug, seconds, {"drums_in": first})
            else:
                from . import library, music

                sounds = library.SoundLibrary(brands.folder(b, "sounds", "../sounds"), ROOT / "out" / ".cache" / "audio")
                root_hz = music.midi_hz(music.MOODS[plan["music"].get("mood", "focus")]["root"])
                bed = np.zeros((int(seconds * 48000), 2), dtype=np.float32)
                print(f"  music: off\n  sounds: {sounds.summary()}")
            effects = sfx.render(events, seconds, root_hz, sounds)
            loud = mix.mix(voice_audio, bed, effects, public / "mix.wav", out / "stems", music_db=float(b.get("musicLevel", -20)))
            print(f"  loudness {loud['lufs']:.1f} LUFS, true peak {loud['true_peak']:.1f} dBTP")

    (out / "captions.srt").write_text(timeline.srt(tl))
    formats = list(specs.FORMATS) if args.format == "all" else [plan["format"]] if args.format else (b.get("formats") or [plan["format"]])
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


VIDEO = {".mp4", ".mov", ".m4v", ".mkv", ".webm"}


def transcribe(args: argparse.Namespace) -> None:
    from . import footage as fx

    video_in = Path(args.video).resolve()
    out = ROOT / "out" / specs.slugify(video_in.stem)
    out.mkdir(parents=True, exist_ok=True)
    with Step("Transcribing"):
        words = fx.transcribe(video_in, ROOT / "out" / ".cache" / "transcripts")
        (out / "transcript.txt").write_text(fx.transcript_text(words) + "\n")
    print(f"  → {(out / 'transcript.txt').relative_to(ROOT)}")


def inbox(args: argparse.Namespace) -> None:
    """Render every new video in the inbox folder.

    A plan next to the video (talk.plan.json beside talk.mp4) is used when
    present; otherwise the plan saved from an earlier run, otherwise Claude
    drafts one. Finished videos are skipped until the video or its plan changes.
    """
    from . import brand as brands

    folder = Path(args.dir).resolve()
    videos = sorted(p for p in folder.iterdir() if p.suffix.lower() in VIDEO) if folder.is_dir() else []
    if not videos:
        print(f"No videos in {folder}")
        return
    b = brands.load(args.brand)
    for video in videos:
        slug = specs.slugify(video.stem)
        out = ROOT / "out" / slug
        side_plan = video.with_suffix(".plan.json")
        saved_plan = out / "plan.json"
        plan = side_plan if side_plan.exists() else saved_plan if saved_plan.exists() else None
        done = out / ".done"
        inputs = [video] + ([plan] if plan else [])
        if done.exists() and all(p.stat().st_mtime <= done.stat().st_mtime for p in inputs):
            print(f"= {video.name}: up to date")
            continue
        if plan is None and not os.environ.get("ANTHROPIC_API_KEY"):
            print(f"! {video.name}: no plan. Add {side_plan.name} or set ANTHROPIC_API_KEY so Claude can draft one.")
            continue
        print(f"> {video.name}")
        formats = b.get("formats") or ["landscape"]
        for fmt in formats:
            footage(argparse.Namespace(video=str(video), plan=str(plan) if plan else None, format=fmt, theme=None, brand=args.brand, no_music=False, stills=False))
            plan = plan or saved_plan
        done.touch()


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
    b.add_argument("--brand", help="brand folder (default: brand/)")
    b.add_argument("--draft", action="store_true", help="skip TTS and audio, render at half size")
    b.add_argument("--stills", action="store_true", help="render two stills per scene instead of the video, for design review")
    b.add_argument("--sample", action="store_true", help="also copy the timeline to src/sample-timeline.json for the studio")
    b.set_defaults(fn=build)

    f = sub.add_parser("footage", help="add motion graphics, captions and sound to your own video")
    f.add_argument("video")
    f.add_argument("--plan", help="graphics plan JSON; drafted with Claude when missing")
    f.add_argument("--format", choices=[*specs.FORMATS, "all"])
    f.add_argument("--theme", choices=sorted(specs.THEMES))
    f.add_argument("--brand", help="brand folder (default: brand/)")
    f.add_argument("--no-music", action="store_true", help="sound effects only, no music bed")
    f.add_argument("--stills", action="store_true", help="render review stills instead of the video")
    f.set_defaults(fn=footage)

    t = sub.add_parser("transcribe", help="write a timestamped transcript of a video")
    t.add_argument("video")
    t.set_defaults(fn=transcribe)

    i = sub.add_parser("inbox", help="render every new video in the inbox folder")
    i.add_argument("--dir", default=str(ROOT / "inbox"))
    i.add_argument("--brand", help="brand folder (default: brand/)")
    i.set_defaults(fn=inbox)

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

    from .brand import BrandError

    args = parser.parse_args(argv)
    try:
        args.fn(args)
    except (specs.SpecError, BrandError) as e:
        print(e, file=sys.stderr)
        sys.exit(2)


if __name__ == "__main__":
    main()
