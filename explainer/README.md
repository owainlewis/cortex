# Explainer

An automated pipeline that adds motion graphics, captions and sound design to explainer videos for horizontal and vertical social formats.

It has two modes:

- **Footage.** Give it your own talking-head video. It transcribes you word by word, places graphics on the exact phrases where they belong, and renders an edit where you play full frame between graphics and tuck into a picture-in-picture card while one is on screen.
- **Narrated.** Give it a script (or a topic, and Claude drafts the script). A local neural voice reads it and the graphics play over a clean canvas.

Either way you get an upload-ready package:

- an H.264 MP4 at 1920x1080, 1080x1920 or 1080x1080, mixed to -14 LUFS;
- `captions.srt` for platforms that take caption files;
- for narrated videos, a cover frame and `post.txt` with a caption and hashtags.

Everything renders locally.
Only the Claude planning and writing steps need an API key.

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
8. **Render.** [Remotion](https://www.remotion.dev) renders the React composition: type that assembles in time with the voice, captions that follow each spoken word, and diagrams that animate the mechanism being explained.

## Design

The look is a calm studio style: a warm canvas inside a soft frame, white stacked cards, line icons, one muted olive accent and black pills for emphasis.
Headlines are set in Instrument Serif, body text in Manrope, and code and numbers in Geist Mono.
Icons come from Lucide and are named in the spec (`"icon": "file-text"`).
Motion is soft and deliberate: words settle in as they are spoken, cards rise into place, arrows draw themselves, and rejected options get struck through.
Scenes show the mechanism itself where they can: the `array` scene moves every byte of an insert, the `tree` scene walks a lookup path, the `code` scene types and then points at the lines that matter.

There are two themes: `studio` (light, the default) and `night` (dark).

## Why Remotion

The graphics are React components rendered by [Remotion](https://www.remotion.dev).
It was chosen over the alternatives for the automated parts:

- It renders headless from a JSON timeline, so one plan produces every format with no manual step.
- It plays and crops your footage (`OffthreadVideo`) and lays the mixed audio under the picture.
- Claude writes React well, and Remotion publishes an official agent skill (`npx skills add remotion-dev/skills`) for adding new scene types.

Claude Motion (in claude.ai) is good for one-off animations made in chat, but it has no API and does not take your footage, so it cannot run this pipeline.
HyperFrames (HTML and GSAP, Apache-2.0) is the alternative if Remotion's license is a problem: Remotion is free for individuals and companies of up to three people and needs a company license above that.

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
./explain build specs/rope.json --stills       # two PNG stills per scene, for design review
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

## Footage mode

```sh
./explain footage talk.mp4                         # transcribe, plan with Claude, render
./explain footage talk.mp4 --format vertical       # 1080x1920
./explain footage talk.mp4 --format all            # every format from one plan
./explain footage talk.mp4 --plan my-plan.json     # use a plan you wrote or edited
./explain footage talk.mp4 --stills                # review stills, no render
./explain footage talk.mp4 --no-music              # sound effects only
```

The first run writes `out/<name>/transcript.txt` and, when no plan is given, asks Claude to draft `out/<name>/plan.json`.
Edit the plan and run again to change what appears and when; transcription is cached.

A plan anchors each graphic to words you actually say:

```json
{
  "title": "From psychology to epidemiology",
  "theme": "studio",
  "music": {"mood": "uplift", "enabled": true},
  "focus": [0.63, 0.35],
  "scenes": [
    {
      "type": "compare",
      "at": "Those are super smart people",
      "until": "less to do with being super smart",
      "heading": "What it *actually* took",
      "left": {"label": "What she assumed", "items": [{"text": "Being super smart", "icon": "brain"}], "cue": "super smart"},
      "right": {"label": "What it took", "items": [{"text": "Sticking with it", "icon": "repeat"}], "cue": "sticking with it"}
    }
  ]
}
```

- `at` is the phrase where the graphic lands; it must appear in the transcript after the previous scene's `at`.
- `until` is optional; without it a graphic holds until the next one or for 12 seconds.
- `focus` is where your face sits in the frame, as fractions, so the picture-in-picture and vertical crops keep you in shot.
- Scene content uses the same scene types as narrated specs, below.
- Cues inside a scene are phrases from that stretch of your speech.

`specs/footage-example.json` is a complete plan for a public-domain NIOSH interview clip.
Set `EXPLAINER_WHISPER=base.en` for faster, less accurate transcription (the default is `small.en`).

## Writing a spec

See `specs/rope.json` and `specs/git-snapshots.json` for complete examples.

```json
{
  "title": "Why typing in a huge file doesn't lag",
  "voice": "am_michael",
  "speed": 1.05,
  "theme": "studio",
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
| `theme` | `studio`, `night` | `studio` |
| `format` | `vertical`, `square`, `landscape` | `vertical` |
| `music.mood` | `focus`, `uplift`, `dark` | `focus` |
| `music.bpm` | number | mood default |
| `voice` | any Kokoro voice, see `./explain voices` | `am_michael` |
| `speed` | speaking rate | `1.0` |
| `handle` | shown under the progress bar and on the outro button | none |
| `pronounce` | display token to spoken words | `{}` |

Text fields accept `*word*` for a serif-italic accent and `==words==` for a solid highlight block.
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
| `array` | `heading?`, `text` (2 to 16 chars), `insertAt`, `insert`, `counterLabel?`, `note?`, `cue?` | when the shift starts |
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
  src/fonts/             Manrope, Instrument Serif and Geist Mono (SIL Open Font License)
  scripts/stills.mjs     renders review stills from a timeline
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
