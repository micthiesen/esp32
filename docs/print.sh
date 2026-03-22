#!/bin/bash
# Generate printable PDF from the wiring checklist
# Usage: bash docs/print.sh [--open]

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
INPUT="$SCRIPT_DIR/wiring-checklist.md"
CSS="$SCRIPT_DIR/print.css"
OUTPUT="$SCRIPT_DIR/wiring-checklist.pdf"

npx -y md-to-pdf "$INPUT" --stylesheet "$CSS"

echo "Generated $OUTPUT"

if [ "$1" = "--open" ]; then
  open "$OUTPUT"
fi
