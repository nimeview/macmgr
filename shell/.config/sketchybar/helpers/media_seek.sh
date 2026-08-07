#!/bin/bash
# Relative seek for nowplaying-cli (absolute seek API).
set -e
delta="${1:-15}"
cur=$(nowplaying-cli get elapsedTime 2>/dev/null | head -1 | cut -d. -f1)
cur=${cur:-0}
if ! [[ "$cur" =~ ^[0-9]+$ ]]; then cur=0; fi
next=$((cur + delta))
if [ "$next" -lt 0 ]; then next=0; fi
nowplaying-cli seek "$next" >/dev/null 2>&1 || true
