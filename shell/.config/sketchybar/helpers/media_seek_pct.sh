#!/bin/bash
# Seek to N% of current track duration (0-100).
set -e
pct="${1:-0}"
if ! [[ "$pct" =~ ^[0-9]+$ ]]; then pct=0; fi
if [ "$pct" -gt 100 ]; then pct=100; fi
dur=$(nowplaying-cli get duration 2>/dev/null | head -1 | cut -d. -f1)
dur=${dur:-0}
if ! [[ "$dur" =~ ^[0-9]+$ ]] || [ "$dur" -le 0 ]; then
  exit 0
fi
seek=$(( dur * pct / 100 ))
nowplaying-cli seek "$seek" >/dev/null 2>&1 || true
