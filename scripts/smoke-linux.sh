#!/usr/bin/env bash
# Run in an isolated D-Bus session and X display after installing the package.
set -euo pipefail
test_home=$(mktemp -d)
export XDG_CACHE_HOME="$test_home/cache" XDG_CONFIG_HOME="$test_home/config"
export XDG_DATA_HOME="$test_home/data" LIBGL_ALWAYS_SOFTWARE=1
export WEBKIT_DISABLE_DMABUF_RENDERER=1
openbox >/dev/null 2>&1 &
wm_pid=$!
clippo >"$test_home/widget.log" 2>&1 &
app_pid=$!
trap 'kill "$app_pid" "$wm_pid" 2>/dev/null || true' EXIT
window=$(timeout 60 xdotool search --sync --onlyvisible --name '^ClippO$' | head -1) || { cat "$test_home/widget.log"; exit 1; }
xdotool windowactivate --sync "$window"
convert -size 641x359 gradient: -strip "$test_home/sample.png"
gs -q -dBATCH -dNOPAUSE -sDEVICE=pdfwrite -sOutputFile="$test_home/sample.pdf" \
  -c '/Helvetica findfont 18 scalefont setfont 20 50 moveto (ClippO PDF test) show showpage'
for ext in png pdf; do
  printf 'file://%s/sample.%s\r\n' "$test_home" "$ext" | xclip -selection clipboard -t text/uri-list
  xdotool key ctrl+v
  for attempt in {1..80}; do
    result=$(find "$XDG_CACHE_HOME" -name "sample-clippo.$ext" -type f -size +0c -print -quit)
    [[ -n "$result" ]] && break
    sleep 0.25
  done
  if [[ -z "$result" ]]; then cat "$test_home/widget.log"; exit 1; fi
  if [[ "$ext" == pdf ]]; then qpdf --check "$result"; else identify "$result"; fi
done
