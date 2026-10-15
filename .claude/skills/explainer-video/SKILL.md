---
name: explainer-video
description: Turn a talking-head video into an edited explainer with motion graphics, captions, sound effects and music, in horizontal and vertical formats. Use when the user gives a video file (or a folder of them) and asks for an explainer, motion graphics, b-roll graphics, captions or a social edit.
---

# Explainer video from footage

The pipeline lives in `explainer/`. Run every command from that folder.
Read `explainer/README.md` (sections "Footage mode" and "Writing a spec") once per session for the scene types and their fields.

## Steps

1. Transcribe the video:
   `./explain transcribe <video>`
   This writes `out/<name>/transcript.txt` with a timestamp on each sentence.

2. Write the graphics plan yourself at `out/<name>/plan.json`.
   Use `specs/footage-example.json` as the format reference.
   Planning rules:
   - Put graphics on moments that carry an idea: an enumerated list, a before and after, a number, a sequence of steps, a contrast.
   - Leave personal or emotional moments on the speaker's face.
   - Cover roughly half the runtime with graphics; each holds 3 to 10 seconds.
   - Use only the speaker's own words and numbers. Never invent facts.
   - Keep on-screen text short: titles under 6 words, labels under 4.
   - `at`, `until` and every `cue` are copied exactly from the transcript, 3 to 6 consecutive words, in transcript order.
   - Pick Lucide icon names that name the thing (`file-text`, `bot`, `database`), not decoration.
   - Set `focus` to where the speaker's face sits in their frame. Grab a frame with `ffmpeg -ss 5 -i <video> -frames:v 1 /tmp/frame.png` and look at it.

3. Review before rendering:
   `./explain footage <video> --plan out/<name>/plan.json --stills`
   Look at every PNG in `out/<name>/stills-<format>/`.
   Fix overlaps, empty cards, text that wraps badly, or graphics that land on the wrong words, then re-run.

4. Render the formats the brand asks for (`formats` in `brand/brand.json`), or what the user asked for:
   `./explain footage <video> --plan out/<name>/plan.json --format landscape`
   `./explain footage <video> --plan out/<name>/plan.json --format vertical`

5. Report the output paths in `out/<name>/` and anything you could not place well.

## Notes

- If `./explain` says to run `./setup.sh`, run it first.
- Transcription is cached, so re-running after a plan edit only re-renders.
- A folder of videos: put them in `explainer/inbox/` with a `<name>.plan.json` beside each, then run `./explain inbox`.
- Brand colours, fonts, logo and caption style live in `explainer/brand/brand.json`; sounds and music in `explainer/sounds/` and `explainer/music/`.
