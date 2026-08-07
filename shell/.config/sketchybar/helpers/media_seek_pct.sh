#!/bin/bash
# Seek to N% of current track (sketchybar passes $PERCENTAGE).
pct="${1:-0}"
pct="${pct%%.*}"
case "$pct" in
  ''|*[!0-9]*) pct=0 ;;
esac
if [ "$pct" -gt 100 ]; then pct=100; fi
if [ "$pct" -lt 0 ]; then pct=0; fi

dur=$(nowplaying-cli get duration 2>/dev/null | head -1)
dur="${dur%%.*}"
case "$dur" in
  ''|*[!0-9]*) exit 0 ;;
esac
if [ "$dur" -le 0 ]; then exit 0; fi

seek=$(( dur * pct / 100 ))
nowplaying-cli seek "$seek" >/dev/null 2>&1 || true
