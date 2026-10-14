# Explainer

An automated pipeline that turns a short JSON script into a narrated, motion-graphics explainer video for TikTok, Reels and Shorts.

One command produces a finished, upload-ready package:

- `<slug>-vertical.mp4`: 1080x1920, 30 fps, H.264 CRF 16, AAC 320 kbps.
- `cover-vertical.png`: a cover frame taken at the end of the hook.
- `captions.srt`: captions for platforms that take an SRT upload.
- `post.txt`: title, caption and hashtags.

Everything runs locally.
No API keys are needed to render.

## What it does

1. **Script.** A spec describes the video as scenes.
   Each scene has a visual type and the narration for it.
   `./explain write "topic"` drafts a spec with Claude, or you write one by hand.
2. **Voice.** Each scene is read by [Kokoro](https://huggingface.co/hexgrad/Kokoro-82M), a local neural TTS model.
   Long pauses are cut down to a quarter second, the way an editor jump-cuts dead air.
3. **Word timing.** Whisper (`faster-whisper`, `base.en`) recovers a timestamp for every word.
   Timings are mapped back onto the words as written in the spec, so captions show `O(n)` while the voice says "O of n".
4. **Timeline.** Every on-screen reveal is tied to the moment its word is spoken.
   A cue is either a phrase you give, or an item word heard in the narration ("Inserting" matches an item called "Insert"), or an even spread as a fallback.
5. **Music.** An original track is synthesised for each video and cut to its exact length: detuned pads, sub bass, FM plucks through a ping-pong delay, and a drum kit with sidechain pumping.
   Drums enter after the hook and drop out for the outro.
6. **Sound design.** Whooshes on scene changes, pitched pops on each reveal, a riser and impact on stats, key clicks while code types, a chime on the call to action.
   All synthesised and placed from the same timeline as the picture.
7. **Mix.** The voice gets a broadcast chain (high-pass, EQ, compression, de-essing).
   Music ducks under speech with a sidechain compressor.
   The master is normalised in two passes to -14 LUFS integrated and -1 dBTP, the level the platforms normalise to.
8. **Render.** [Remotion](https://www.remotion.dev) renders the React composition: kinetic typography synced to the voice, karaoke captions, animated diagrams, a story-style progress bar, and per-scene lighting changes.

## Setup

Requires macOS with Homebrew `ffmpeg`, Node.js 20+ and Python 3.11+.

```sh
cd explainer
./setup.sh
```

`setup.sh` creates `.venv`, installs Node dependencies, and downloads the Kokoro model (about 350 MB) into `models/`.
Whisper downloads its model on first use.

## Usage

```sh
./explain build specs/rope.json                # full render
./explain build specs/rope.json --draft        # no voice or audio, half size, fast
./explain build specs/rope.json --format all   # vertical, square and landscape
./explain build specs/rope.json --theme paper  # override the theme
./explain check specs/rope.json                # validate only
./explain voices                               # list narrator voices
./explain write "how B-trees work" --notes "angle: why databases use them"
npm run studio                                 # live preview in Remotion Studio
```

Output goes to `out/<slug>/`.

`write` needs Anthropic credentials (`ANTHROPIC_API_KEY` or an `ant auth login` profile).
It uses Claude Opus 5.5 with the server-side refusal fallback enabled.
Drafts that fail validation are sent back to Claude with the errors for one repair round.
Read the draft before rendering: the writer is told not to invent statistics, but you are the fact checker.

`build --sample` also copies the timeline to `src/sample-timeline.json`, which is what Remotion Studio previews.

## Writing a spec

See `specs/rope.json` and `specs/git-snapshots.json` for complete examples.

```json
{
  "title": "Why typing in a huge file doesn't lag",
  "voice": "am_michael",
  "speed": 1.05,
  "theme": "midnight",
  "format": "vertical",
  "music": {"mood": "focus"},
  "pronounce": {"MB": "megabyte"},
  "post": {"caption": "...", "hashtags": ["programming"]},
  "scenes": [
    {"type": "hook", "title": "Type into a *100 MB* file. ==No lag.==", "narration": "..."}
  ]
}
```

Top-level settings:

| Key | Values | Default |
| --- | --- | --- |
| `theme` | `midnight`, `ember`, `paper` | `midnight` |
| `format` | `vertical`, `square`, `landscape` | `vertical` |
| `music.mood` | `focus`, `uplift`, `dark` | `focus` |
| `music.bpm` | number | mood default |
| `voice` | any Kokoro voice, see `./explain voices` | `am_michael` |
| `speed` | speaking rate | `1.0` |
| `handle` | shown under the progress bar and on the outro button | none |
| `pronounce` | display token to spoken words | `{}` |

Text fields accept `*word*` for the accent colour and `==words==` for a highlighter swipe.
Marked phrases never break across lines.

Scene types and their fields:

| Type | Fields | Cues |
| --- | --- | --- |
| `hook` | `title`, `subtitle?` | title words appear as they are spoken |
| `statement` | `text`, `sub?`, `subCue?` | `sub` |
| `stat` | `value`, `prefix?`, `suffix?`, `decimals?`, `label`, `sub?`, `cue?`, `subCue?` | count-up start, `sub` |
| `list` | `heading`, `items` (string or `{text, cue}`) | one per item |
| `compare` | `heading?`, `left`/`right` (`{label, items, tone, cue}`) | one per side |
| `bars` | `heading`, `items` (`{label, value, highlight?, cue?}`), `unit?`, `prefix?`, `decimals?`, `note?` | one per bar |
| `code` | `filename`, `language`, `code`, `highlights` (`{lines, cue, note}`), `reveal?` | one per highlight |
| `terminal` | `lines` (`{cmd, out, cue}`) | one per command |
| `flow` | `heading?`, `nodes` (`{label, sub?}`) | one per node |
| `tree` | `heading?`, `root` (`{label, children}`), `levelCues?`, `highlight?` (`{path, cue, note}`), `quoteLeaves?` | one per level, then the highlight |
| `outro` | `title`, `sub?`, `cta?` | none |

Every scene also takes `kicker` (a small label), `narration`, `captions` (defaults to on except for `hook` and `outro`), `voice` and `pronounce`.

The validator rejects specs that will not render well: a first scene that is not a hook, more than six list items, code longer than 16 lines, tree highlight paths that do not exist.

## Layout

```
explainer/
  explain, setup.sh      entry point and one-time setup
  specs/                 example scripts
  pipeline/              Python: spec, voice, timeline, music, sfx, mix, render, writer
  src/                   Remotion: composition, scenes, captions, background, theme
  src/fonts/             Inter Tight and JetBrains Mono (SIL Open Font License)
  tests/                 unit tests for the pure pipeline logic
```

The timeline JSON is the contract between the two halves.
The pipeline computes every frame of timing; the React side only reads it, so picture and sound cannot drift apart.

## Tests

```sh
.venv/bin/python -m unittest discover -s tests -t .
npm run typecheck
```

## Notes

- Fonts are bundled so renders never depend on the network and look the same on every machine.
- Remotion is free for individuals and companies of up to three people; larger companies need a [company license](https://www.remotion.dev/license).
- Kokoro (Apache 2.0) and Whisper (MIT) model weights are downloaded, not committed.
- Rendering a 60-second vertical video takes about 6 minutes on a 4-core Linux machine and less on Apple Silicon.
