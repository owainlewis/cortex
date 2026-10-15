"""Draft a spec from a topic with Claude.

The writer is held to the same validator as hand-written specs: if the draft
fails validation, the errors go back to Claude for one repair round.
"""

from __future__ import annotations

import json
import re
from pathlib import Path

from . import spec as specs

ROOT = Path(__file__).resolve().parent.parent
MODEL = "claude-opus-5-5"

SCENE_GUIDE = """Every scene has a "type" and an optional "kicker" (2-4 word label shown in tracked capitals). Text fields accept *word* for an italic accent and ==words== for the accent colour; use them on the one word that matters, never more.

Scene types:
- hook: {"title", "subtitle"?} big serif title; the subtitle appears as a black pill. Keep the title close to the narration wording so it assembles in sync with the voice. Must be scene 1.
- statement: {"text", "sub"?} one big sentence, optional supporting line.
- stat: {"value": number, "prefix"?, "suffix"?, "decimals"?, "label", "sub"?, "cue"?} animated counter.
- list: {"heading", "items": [string or {"text", "icon"?, "detail"?, "badge"?, "cue"?}]} 2 to 5 option cards, each title under 6 words.
- compare: {"heading"?, "left": {"label", "items", "cue"?}, "right": {"label", "items", "cue"?}} the left items are shown, then struck through when the right side lands; "right.label" becomes a black pill over the winner. 1-3 short items per side, each item a string or {"text", "icon"}.
- bars: {"heading", "unit"?, "prefix"?, "decimals"?, "items": [{"label", "value", "highlight"?, "cue"?}], "note"?} up to 5 bars.
- code: {"filename", "language", "code", "highlights": [{"lines": [a, b], "cue", "note"}]} 12 lines max, 38 characters per line max.
- terminal: {"heading"?, "lines": [{"cmd", "out": [..], "cue"?}]} real commands with realistic output.
- flow: {"heading"?, "nodes": [{"label", "sub"?, "icon"?, "badge"?}]} 2 to 4 cards joined by hand-drawn arrows.
- tree: {"heading"?, "root": {"label", "children": [...]}, "levelCues"?: [...], "highlight"?: {"path": [child indexes], "cue", "note"}} up to 3 levels, 8 leaves max.
- array: {"heading"?, "text", "insertAt", "insert", "cue"?, "counterLabel"?, "note"?} a contiguous buffer (2-16 chars) taking one insert; cells after the cursor visibly shift one by one with a counter. Use it to show why contiguous memory is expensive to edit.
- outro: {"title", "sub"?, "cta"?} payoff line.

Cues: a "cue" is a word or short phrase from that scene's narration; the element appears the moment the narrator says it. Use the exact words as they appear in the narration. If you omit cues, the pipeline matches item words against the narration or spreads them evenly.

Use visual scene types over statement scenes wherever the idea has structure (numbers, steps, comparisons, code, hierarchy). The best scenes show the mechanism itself moving, not a label describing it.
Icons are Lucide names in kebab case, e.g. "file-text", "bot", "rocket", "user", "folder", "database", "git-branch", "terminal", "chart-line", "message-square", "sparkles", "wrench", "clock", "shield-check", "zap". Pick one that names the thing, not a decoration.

"""

SYSTEM_TEMPLATE = """You write scripts for short vertical explainer videos (TikTok, Reels, Shorts) about software and technical topics.

Your output is a JSON spec that a rendering pipeline turns into a narrated motion-graphics video. Return only the JSON object, with no prose around it.

What makes these videos worth watching:
- The hook scene states a concrete, surprising claim or question in under 3 seconds of speech. No greetings, no "in this video".
- Every scene earns its place by teaching one idea. Cut anything that does not move the viewer toward the payoff.
- Prefer concrete numbers, real examples, and real code over abstractions. Never invent statistics; if you are not sure a number is right, use a qualitative claim instead.
- Narration is spoken English: short sentences, plain words, contractions. Read it aloud in your head.
- Total narration is 110 to 170 words (about 45 to 65 seconds). 6 to 9 scenes.
- End with a one-line payoff that restates the core idea, then a short follow call.

Spec format (all keys shown are supported; omit optional ones you do not need):
{
  "title": "...",                      // also the video file name
  "voice": "am_michael",               // Kokoro voice; am_michael, am_fenrir, af_heart, af_bella, bm_george, bf_emma
  "speed": 1.05,
  "theme": "studio" | "night",
  "music": {"mood": "focus" | "uplift" | "dark"},
  "pronounce": {"word": "how to say it"},   // single display tokens whose TTS reading is wrong, e.g. "MB": "megabyte", "O(n)": "O of n"
  "post": {"caption": "...", "hashtags": ["..."]},
  "scenes": [ ... ]
}

{SCENE_GUIDE}"""


SYSTEM = SYSTEM_TEMPLATE.replace("{SCENE_GUIDE}", SCENE_GUIDE)

PLAN_SYSTEM = """You are a motion designer planning graphics for a talking-head video.

You get a timestamped transcript of what the speaker says. You return a JSON plan: which graphic appears, anchored to the exact phrase where it should land. The speaker plays full frame between graphics and shrinks to a picture-in-picture card while a graphic is on screen. Return only the JSON object.

What good plans do:
- Graphics land on the moments that carry an idea: a list being enumerated, a before and after, a number, a sequence of steps, a contrast. Personal or emotional moments stay on the speaker's face.
- Cover roughly half the runtime with graphics. Each graphic holds 3 to 10 seconds.
- Graphics use the speaker's own words and numbers. Never invent facts they did not say.
- Text on graphics is short: titles under 6 words, labels under 4.

Plan format:
{
  "title": "...",
  "theme": "studio" | "night",
  "music": {"mood": "focus" | "uplift" | "dark", "enabled": true},
  "focus": [x, y],                    // where the speaker's face sits in their frame, as fractions; [0.5, 0.4] if unsure
  "scenes": [
    {"type": "...", "at": "exact words from the transcript where it lands", "until": "exact words where it should end"?, ...content}
  ]
}

"at" and "until" must be copied exactly from the transcript, three to six consecutive words, in transcript order. Scene cues are also exact phrases from inside that scene's stretch of transcript.

{SCENE_GUIDE}"""
PLAN_SYSTEM = PLAN_SYSTEM.replace("{SCENE_GUIDE}", SCENE_GUIDE)


def _extract_json(text: str) -> dict:
    match = re.search(r"\{.*\}", text, re.S)
    if not match:
        raise ValueError("no JSON object in response")
    return json.loads(match.group(0))


def _ask(client, messages: list[dict], system: str = SYSTEM) -> str:
    with client.beta.messages.stream(
        model=MODEL,
        max_tokens=32000,
        system=system,
        thinking={"type": "adaptive"},
        output_config={"effort": "high"},
        betas=["server-side-fallback-2026-07-01"],
        fallbacks="default",
        messages=messages,
    ) as stream:
        response = stream.get_final_message()
    if response.stop_reason == "refusal":
        raise SystemExit("Claude declined to write this topic.")
    return "".join(b.text for b in response.content if b.type == "text")


def _draft(system: str, prompt: str, check) -> dict:
    """Ask for JSON, validate it, and give Claude one round to fix errors."""
    import anthropic

    client = anthropic.Anthropic()
    messages = [{"role": "user", "content": prompt}]
    for attempt in range(2):
        text = _ask(client, messages, system)
        try:
            draft = _extract_json(text)
            check(draft)
            return draft
        except (ValueError, specs.SpecError) as e:
            if attempt == 1:
                raise SystemExit(f"Draft still invalid after a repair round:\n{e}") from e
            messages += [
                {"role": "assistant", "content": text},
                {"role": "user", "content": f"That does not validate:\n{e}\n\nReturn the corrected JSON only."},
            ]
    raise AssertionError("unreachable")


def _with_defaults(draft: dict) -> dict:
    checked = {**draft}
    for key, value in specs.DEFAULTS.items():
        checked.setdefault(key, value)
    return checked


def write(topic: str, notes: str = "") -> dict:
    example = (ROOT / "specs" / "rope.json").read_text()
    prompt = (
        f"Topic: {topic}\n\n"
        + (f"Notes and source material to use:\n{notes}\n\n" if notes else "")
        + "Here is an example of a finished spec for a different topic, for format and tone:\n"
        + example
    )
    return _draft(SYSTEM, prompt, lambda d: specs.validate(_with_defaults(d)))


def plan(transcript: str, title: str, check_anchors=None) -> dict:
    """Plan graphics for a talking-head video from its timestamped transcript.

    `check_anchors` optionally raises SpecError when an `at` phrase is not in
    the transcript, so Claude gets a chance to fix it.
    """
    example = (ROOT / "specs" / "footage-example.json").read_text()

    def check(d: dict) -> None:
        d.setdefault("title", title)
        specs.validate(_with_defaults({**d, "format": d.get("format", "landscape")}), footage=True)
        if check_anchors:
            check_anchors(d)

    prompt = f"Video: {title}\n\nTranscript:\n{transcript}\n\nHere is an example plan for a different video, for format:\n{example}"
    return _draft(PLAN_SYSTEM, prompt, check)
