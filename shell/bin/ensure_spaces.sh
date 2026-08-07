#!/bin/bash
# Ensure 9 yabai spaces exist.
desired=9
if ! command -v yabai >/dev/null 2>&1; then
  exit 0
fi
if ! command -v jq >/dev/null 2>&1; then
  exit 0
fi
current=$(yabai -m query --spaces 2>/dev/null | jq length)
current=${current:-0}
if [ "$current" -lt "$desired" ]; then
  for ((i=current+1; i<=desired; i++)); do
    yabai -m space --create >/dev/null 2>&1 || true
  done
fi
