#!/bin/bash
# Seek to N% of track. Yandex Music often ignores MediaRemote seek —
# still works for Spotify / Apple Music.
pct="${1:-0}"
pct="${pct%%.*}"
case "$pct" in
  ''|*[!0-9]*) pct=0 ;;
esac
[ "$pct" -gt 100 ] && pct=100
[ "$pct" -lt 0 ] && pct=0

dur=$(nowplaying-cli get duration 2>/dev/null | head -1)
dur="${dur%%.*}"
case "$dur" in
  ''|*[!0-9]*) exit 0 ;;
esac
[ "$dur" -le 0 ] && exit 0

seek=$(( dur * pct / 100 ))
nowplaying-cli seek "$seek" >/dev/null 2>&1 \
  || nowplaying-cli seek "${seek}.0" >/dev/null 2>&1 \
  || true

sketchybar --trigger media_change >/dev/null 2>&1 || true
