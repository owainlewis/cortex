"""Load and validate explainer specs.

A spec is a JSON file describing one video: global settings plus an ordered
list of scenes. Each scene has a visual `type`, the `narration` the voice
reads, and type specific content. See README.md for the full format.
"""

from __future__ import annotations

import json
import re
from pathlib import Path

FORMATS = {
    "vertical": (1080, 1920),
    "square": (1080, 1080),
    "landscape": (1920, 1080),
}
THEMES = {"studio", "night"}
MOODS = {"focus", "uplift", "dark"}

# Required content fields per scene type.
SCENE_FIELDS = {
    "hook": ["title"],
    "statement": ["text"],
    "stat": ["value", "label"],
    "list": ["items"],
    "compare": ["left", "right"],
    "bars": ["items"],
    "code": ["code"],
    "terminal": ["lines"],
    "flow": ["nodes"],
    "tree": ["root"],
    "array": ["text"],
    "outro": ["title"],
}

DEFAULTS = {
    "voice": "am_michael",
    "speed": 1.0,
    "theme": "studio",
    "format": "vertical",
    "fps": 30,
    "handle": None,
    "music": {},
    "pronounce": {},
    "post": {},
}


class SpecError(Exception):
    pass


def slugify(text: str) -> str:
    return re.sub(r"[^a-z0-9]+", "-", text.lower()).strip("-")[:60] or "explainer"


def load(path: str | Path, footage: bool = False) -> dict:
    path = Path(path)
    try:
        spec = json.loads(path.read_text())
    except json.JSONDecodeError as e:
        raise SpecError(f"{path}: invalid JSON: {e}") from e
    # Remember whether the video chose its own theme, so a brand default can apply otherwise.
    spec["_theme_set"] = "theme" in spec
    for key, value in DEFAULTS.items():
        spec.setdefault(key, value)
    spec.setdefault("slug", slugify(spec.get("title", path.stem)))
    validate(spec, footage)
    return spec


def _tree_depth(node: dict) -> int:
    kids = node.get("children") or []
    return 0 if not kids else 1 + max(_tree_depth(k) for k in kids)


def _tree_path_ok(node: dict, path: list) -> bool:
    for i in path:
        kids = node.get("children") or []
        if not isinstance(i, int) or i < 0 or i >= len(kids):
            return False
        node = kids[i]
    return True


def validate(spec: dict, footage: bool = False) -> None:
    """Check a spec. In footage mode scenes are anchored to the speaker's words
    with `at` instead of carrying their own narration."""
    errors: list[str] = []
    if not spec.get("title"):
        errors.append("missing top-level 'title'")
    if spec["format"] not in FORMATS:
        errors.append(f"format must be one of {sorted(FORMATS)}")
    if spec["theme"] not in THEMES:
        errors.append(f"theme must be one of {sorted(THEMES)}")
    mood = spec["music"].get("mood", "focus")
    if mood not in MOODS:
        errors.append(f"music.mood must be one of {sorted(MOODS)}")
    scenes = spec.get("scenes")
    if not isinstance(scenes, list) or not scenes:
        errors.append("'scenes' must be a non-empty list")
        scenes = []
    for i, scene in enumerate(scenes):
        where = f"scene {i + 1}"
        kind = scene.get("type")
        if kind not in SCENE_FIELDS:
            errors.append(f"{where}: unknown type {kind!r}; expected one of {sorted(SCENE_FIELDS)}")
            continue
        where = f"scene {i + 1} ({kind})"
        if footage and not str(scene.get("at", "")).strip():
            errors.append(f"{where}: missing 'at' (the phrase in the transcript where it starts)")
        if not footage and not str(scene.get("narration", "")).strip():
            errors.append(f"{where}: missing 'narration'")
        for field in SCENE_FIELDS[kind]:
            if field not in scene:
                errors.append(f"{where}: missing '{field}'")
        if kind in ("list", "bars") and not 1 <= len(scene.get("items", [])) <= 6:
            errors.append(f"{where}: 'items' needs 1 to 6 entries")
        if kind == "flow" and not 2 <= len(scene.get("nodes", [])) <= 6:
            errors.append(f"{where}: 'nodes' needs 2 to 6 entries")
        if kind == "bars":
            for item in scene.get("items", []):
                if not isinstance(item, dict) or not isinstance(item.get("value"), (int, float)):
                    errors.append(f"{where}: every bar needs a numeric 'value'")
                    break
        if kind == "stat" and not isinstance(scene.get("value"), (int, float)):
            errors.append(f"{where}: 'value' must be a number")
        if kind == "compare":
            for side in ("left", "right"):
                s = scene.get(side) or {}
                if not s.get("label") or not s.get("items"):
                    errors.append(f"{where}: '{side}' needs 'label' and 'items'")
        if kind == "code":
            n = len(str(scene.get("code", "")).split("\n"))
            if n > 16:
                errors.append(f"{where}: code has {n} lines; keep it to 16 or fewer so it stays legible")
            for h in scene.get("highlights", []):
                lines = h.get("lines") or []
                if not lines or min(lines) < 1 or max(lines) > n:
                    errors.append(f"{where}: highlight lines {lines} are outside 1..{n}")
        if kind == "array" and not 2 <= len(str(scene.get("text", ""))) <= 16:
            errors.append(f"{where}: 'text' needs 2 to 16 characters so the cells stay legible")
        if kind == "tree" and "root" in scene:
            if _tree_depth(scene["root"]) > 4:
                errors.append(f"{where}: tree is deeper than 4 levels")
            hl = scene.get("highlight")
            if hl and not _tree_path_ok(scene["root"], hl.get("path", [])):
                errors.append(f"{where}: highlight.path {hl.get('path')} does not exist in the tree")
    if scenes and not footage and scenes[0].get("type") != "hook":
        errors.append("scene 1 should be a 'hook': the first two seconds decide whether anyone watches")
    if errors:
        raise SpecError("Invalid spec:\n  - " + "\n  - ".join(errors))


def _cue(item) -> str | None:
    return item.get("cue") if isinstance(item, dict) else None


def _text(item) -> str:
    if isinstance(item, str):
        return item
    return str(item.get("text") or item.get("label") or "")


def cue_targets(scene: dict) -> list[tuple[str | None, str]]:
    """The reveal points a scene needs, as (explicit cue phrase, item text).

    The pipeline turns each into a frame: the explicit phrase when given,
    otherwise a word from the item text heard in the narration, otherwise an
    even spread across the scene.
    """
    kind = scene["type"]
    if kind == "list":
        return [(_cue(x), _text(x)) for x in scene["items"]]
    if kind == "flow":
        return [(_cue(x), _text(x)) for x in scene["nodes"]]
    if kind == "bars":
        out = [(_cue(x), _text(x)) for x in scene["items"]]
        if scene.get("note"):
            out.append((scene.get("noteCue"), scene["note"]))
        return out
    if kind == "compare":
        return [(_cue(scene[s]), _text(scene[s])) for s in ("left", "right")]
    if kind == "stat":
        out = [(scene.get("cue"), "")]
        if scene.get("sub"):
            out.append((scene.get("subCue"), scene["sub"]))
        return out
    if kind == "hook":
        return [(scene.get("subtitleCue"), scene["subtitle"])] if scene.get("subtitle") else []
    if kind == "statement":
        return [(scene.get("subCue"), scene["sub"])] if scene.get("sub") else []
    if kind == "code":
        return [(_cue(h), h.get("note", "")) for h in scene.get("highlights", [])]
    if kind == "array":
        return [(scene.get("cue"), "shift shifting move moving copy")]
    if kind == "terminal":
        return [(_cue(x), str(x.get("cmd", ""))) for x in scene["lines"]]
    if kind == "tree":
        levels = scene.get("levelCues") or []
        depth = _tree_depth(scene["root"])
        out = [(levels[i] if i < len(levels) else None, "") for i in range(depth + 1)]
        if scene.get("highlight"):
            out.append((scene["highlight"].get("cue"), scene["highlight"].get("note", "")))
        return out
    return []
