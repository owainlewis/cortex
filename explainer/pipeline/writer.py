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

SYSTEM = """You write scripts for short vertical explainer videos (TikTok, Reels, Shorts) about software and technical topics.

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
  "theme": "midnight" | "ember" | "paper",
  "music": {"mood": "focus" | "uplift" | "dark"},
  "pronounce": {"word": "how to say it"},   // single display tokens whose TTS reading is wrong, e.g. "MB": "megabyte", "O(n)": "O of n"
  "post": {"caption": "...", "hashtags": ["..."]},
  "scenes": [ ... ]
}

Every scene has "type", "narration" and an optional "kicker" (2-3 word uppercase label). Text fields accept *word* for accent colour and ==words== for a highlighter swipe; use them on the one or two words that matter.

Scene types:
- hook: {"title"} big kinetic title. Keep the title close to the narration wording so it assembles in sync with the voice. Must be scene 1.
- statement: {"text", "sub"?} one big sentence, optional supporting line.
- stat: {"value": number, "prefix"?, "suffix"?, "decimals"?, "label", "sub"?, "cue"?} animated counter.
- list: {"heading", "items": [string or {"text", "cue"}]} 2 to 5 items, each under 7 words.
- compare: {"heading"?, "left": {"label", "items", "tone": "bad"|"good"}, "right": {...}} two panels, 2-3 short items each.
- bars: {"heading", "unit"?, "prefix"?, "decimals"?, "items": [{"label", "value", "highlight"?, "cue"?}], "note"?} up to 5 bars.
- code: {"filename", "language", "code", "highlights": [{"lines": [a, b], "cue", "note"}]} 12 lines max, 38 characters per line max.
- terminal: {"heading"?, "lines": [{"cmd", "out": [..], "cue"?}]} real commands with realistic output.
- flow: {"heading"?, "nodes": [{"label", "sub"?}]} 2 to 5 steps of a process.
- tree: {"heading"?, "root": {"label", "children": [...]}, "levelCues"?: [...], "highlight"?: {"path": [child indexes], "cue", "note"}} up to 3 levels, 8 leaves max.
- outro: {"title", "sub"?, "cta"?} payoff line.

Cues: a "cue" is a word or short phrase from that scene's narration; the element appears the moment the narrator says it. Use the exact words as they appear in the narration. If you omit cues, the pipeline matches item words against the narration or spreads them evenly.

Use visual scene types over statement scenes wherever the idea has structure (numbers, steps, comparisons, code, hierarchy)."""


def _extract_json(text: str) -> dict:
    match = re.search(r"\{.*\}", text, re.S)
    if not match:
        raise ValueError("no JSON object in response")
    return json.loads(match.group(0))


def _ask(client, messages: list[dict]) -> str:
    with client.beta.messages.stream(
        model=MODEL,
        max_tokens=32000,
        system=SYSTEM,
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


def write(topic: str, notes: str = "") -> dict:
    import anthropic

    client = anthropic.Anthropic()
    example = (ROOT / "specs" / "rope.json").read_text()
    prompt = (
        f"Topic: {topic}\n\n"
        + (f"Notes and source material to use:\n{notes}\n\n" if notes else "")
        + "Here is an example of a finished spec for a different topic, for format and tone:\n"
        + example
    )
    messages = [{"role": "user", "content": prompt}]
    for attempt in range(2):
        text = _ask(client, messages)
        try:
            spec = _extract_json(text)
            checked = {**spec}
            for key, value in specs.DEFAULTS.items():
                checked.setdefault(key, value)
            specs.validate(checked)
            return spec
        except (ValueError, specs.SpecError) as e:
            if attempt == 1:
                raise SystemExit(f"Draft still invalid after a repair round:\n{e}") from e
            messages += [
                {"role": "assistant", "content": text},
                {"role": "user", "content": f"That spec does not validate:\n{e}\n\nReturn the corrected JSON only."},
            ]
    raise AssertionError("unreachable")
