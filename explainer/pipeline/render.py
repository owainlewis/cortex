"""Drive the Remotion renderer."""

from __future__ import annotations

import os
import shutil
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent


def _base(kind: str, props: Path, public: Path) -> tuple[list[str], list[str]]:
    npx = shutil.which("npx")
    if not npx:
        raise SystemExit("npx not found. Install Node.js 20+ and run ./setup.sh.")
    cmd = [
        npx,
        "remotion",
        kind,
        "src/index.ts",
        "Explainer",
    ]
    flags = [f"--props={props}", f"--public-dir={public}", "--log=error"]
    browser = os.environ.get("REMOTION_BROWSER_EXECUTABLE")
    if browser:
        flags.append(f"--browser-executable={browser}")
    return cmd, flags


def video(props: Path, public: Path, out: Path, draft: bool, has_audio: bool) -> None:
    cmd, flags = _base("render", props, public)
    flags += [
        f"--concurrency={max(1, (os.cpu_count() or 2))}",
        "--codec=h264",
        "--pixel-format=yuv420p",
        "--image-format=jpeg",
        f"--jpeg-quality={80 if draft else 95}",
        f"--crf={26 if draft else 16}",
        "--x264-preset=" + ("veryfast" if draft else "slow"),
    ]
    if draft:
        flags.append("--scale=0.5")
    if has_audio:
        flags += ["--audio-codec=aac", "--audio-bitrate=320k"]
    else:
        flags.append("--muted")
    subprocess.run(cmd + [str(out)] + flags, cwd=ROOT, check=True)


def still(props: Path, public: Path, out: Path, frame: int) -> None:
    cmd, flags = _base("still", props, public)
    subprocess.run(cmd + [str(out), f"--frame={frame}", "--image-format=png"] + flags, cwd=ROOT, check=True)


def stills(props: Path, public: Path, out: Path) -> None:
    subprocess.run(["node", "scripts/stills.mjs", str(props), str(public), str(out)], cwd=ROOT, check=True)
