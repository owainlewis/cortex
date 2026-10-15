"""Brand settings: everything that makes the videos look and sound like yours.

A brand is a folder with a `brand.json` and the files it points at (fonts,
logo). Paths inside it are relative to that folder. Every key is optional;
anything missing falls back to the built-in studio look.
"""

from __future__ import annotations

import json
import shutil
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DEFAULT = ROOT / "brand"

COLOR_KEYS = {
    "frame", "canvas", "card", "border", "text", "muted", "faint", "accent", "accentSoft", "accentInk",
    "ink", "inkText", "inkMuted", "captionBg", "captionText",
}
FONT_KEYS = ("sans", "serif", "serifItalic", "mono")


class BrandError(Exception):
    pass


def load(path: str | Path | None = None) -> dict:
    folder = Path(path) if path else DEFAULT
    if folder.is_file():
        folder = folder.parent
    file = folder / "brand.json"
    data = json.loads(file.read_text()) if file.exists() else {}
    data["_dir"] = str(folder.resolve())
    problems = []
    unknown = set(data.get("colors", {})) - COLOR_KEYS
    if unknown:
        problems.append(f"unknown colors {sorted(unknown)}; expected some of {sorted(COLOR_KEYS)}")
    for key in FONT_KEYS:
        f = (data.get("fonts") or {}).get(key)
        if f and not f.startswith("local:") and not (folder / f).exists():
            problems.append(f"font {key!r} file not found: {folder / f}")
    if data.get("logo") and not (folder / data["logo"]).exists():
        problems.append(f"logo not found: {folder / data['logo']}")
    style = data.get("style") or {}
    allowed = {
        "surface": ("card", "outline"),
        "background": ("plain", "dots", "grid"),
        "labelColor": ("muted", "accent"),
        "labelFont": ("sans", "mono"),
        "arrows": ("curved", "straight"),
        "corners": (True, False),
        "figures": (True, False),
        "divider": (True, False),
    }
    for key, options in allowed.items():
        if key in style and style[key] not in options:
            problems.append(f"style.{key} must be one of {options}")
    if data.get("captions", "box") not in ("box", "off"):
        problems.append("captions must be 'box' or 'off'")
    if problems:
        raise BrandError(f"{file}:\n  - " + "\n  - ".join(problems))
    return data


def folder(brand: dict, key: str, default: str) -> Path:
    """Sound or music folder, relative to the brand folder."""
    return (Path(brand["_dir"]) / brand.get(key, default)).resolve()


def publish(brand: dict, public: Path) -> dict:
    """Copy brand assets next to the render and return what the renderer needs."""
    src = Path(brand["_dir"])
    dest = public / "brand"
    dest.mkdir(parents=True, exist_ok=True)
    fonts = {}
    for key in FONT_KEYS:
        f = (brand.get("fonts") or {}).get(key)
        if f and f.startswith("local:"):
            fonts[key] = f  # installed on the rendering machine
        elif f:
            shutil.copy(src / f, dest / Path(f).name)
            fonts[key] = f"brand/{Path(f).name}"
    logo = None
    if brand.get("logo"):
        shutil.copy(src / brand["logo"], dest / Path(brand["logo"]).name)
        logo = f"brand/{Path(brand['logo']).name}"
    return {
        "colors": {**brand.get("colors", {}), **(brand.get("style") or {})},
        "fonts": fonts,
        "logo": logo,
        "radius": brand.get("radius"),
        "captions": brand.get("captions", "box"),
    }
