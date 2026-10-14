#!/usr/bin/env bash
# One-time setup: Python env, Node deps, voice model.
set -euo pipefail
cd "$(dirname "$0")"

command -v ffmpeg >/dev/null || { echo "ffmpeg is required: brew install ffmpeg" >&2; exit 1; }
command -v node >/dev/null || { echo "Node.js 20+ is required: brew install node" >&2; exit 1; }

python3 -m venv .venv
.venv/bin/pip install --quiet --upgrade pip
.venv/bin/pip install --quiet -r requirements.txt
npm ci --no-audit --no-fund

mkdir -p models
base=https://github.com/thewh1teagle/kokoro-onnx/releases/download/model-files-v1.0
for f in kokoro-v1.0.onnx voices-v1.0.bin; do
  [[ -f models/$f ]] || curl -fL --progress-bar -o "models/$f" "$base/$f"
done
echo "Ready. Try: ./explain build specs/rope.json"
