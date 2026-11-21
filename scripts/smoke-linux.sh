#!/usr/bin/env bash
# Run in an isolated D-Bus session and X display after installing the package.
set -euo pipefail
test_home=$(mktemp -d)
export XDG_CACHE_HOME="$test_home/cache" XDG_CONFIG_HOME="$test_home/config"
export XDG_DATA_HOME="$test_home/data" LIBGL_ALWAYS_SOFTWARE=1
export XDG_STATE_HOME="$test_home/state"
theme_file="$XDG_STATE_HOME/omarchy/current/theme/colors.toml"
mkdir -p "$(dirname "$theme_file")"
set_palette() {
  printf 'mode = "%s"\nbackground = "%s"\nforeground = "%s"\naccent = "%s"\n' "$@" > "$theme_file"
}
set_palette dark '#202630' '#edf1f4' '#91cafa'
export WEBKIT_DISABLE_DMABUF_RENDERER=1
openbox >/dev/null 2>&1 &
wm_pid=$!
clippo >"$test_home/widget.log" 2>&1 &
app_pid=$!
trap 'kill "$app_pid" "$wm_pid" 2>/dev/null || true' EXIT
window=$(timeout 60 xdotool search --sync --onlyvisible --name '^ClippO$' | head -1) || { cat "$test_home/widget.log"; exit 1; }
xdotool windowactivate --sync "$window"
await_panel_color() {
  for attempt in {1..40}; do
    if import -silent -window root -depth 8 miff:- |
      convert - -format %c histogram:info:- | grep -i "$1" >/dev/null; then
      return 0
    fi
    sleep 0.25
  done
  echo "ClippO did not render the expected Omarchy background $1" >&2
  return 1
}
await_panel_color '#202630'
set_palette light '#f4ede2' '#352f29' '#805726'
await_panel_color '#f4ede2'
convert -size 641x359 gradient: -strip "$test_home/sample.png"
gs -q -dBATCH -dNOPAUSE -sDEVICE=pdfwrite -sOutputFile="$test_home/sample.pdf" \
  -c '/Helvetica findfont 18 scalefont setfont 20 50 moveto (ClippO PDF test) show showpage'
python3 - "$test_home/sample.wav" <<'PY'
import math, struct, sys, wave
with wave.open(sys.argv[1], "wb") as f:
    f.setparams((1, 2, 44100, 0, "NONE", ""))
    f.writeframes(b"".join(struct.pack("<h", round(16000 * math.sin(2 * math.pi * i / 100))) for i in range(44100)))
PY
for ext in png pdf wav; do
  suffix=$ext
  if [[ "$ext" == wav ]]; then suffix=mp3; fi
  printf 'file://%s/sample.%s\r\n' "$test_home" "$ext" | xclip -selection clipboard -t text/uri-list
  xdotool key ctrl+v
  for attempt in {1..80}; do
    result=$(find "$XDG_CACHE_HOME" -name "sample-clippo.$suffix" -type f -size +0c -print -quit)
    [[ -n "$result" ]] && break
    sleep 0.25
  done
  if [[ -z "$result" ]]; then cat "$test_home/widget.log"; exit 1; fi
  case "$suffix" in
    pdf) qpdf --check "$result";;
    png) identify "$result";;
    mp3) file --brief --mime-type "$result" | grep -Fx audio/mpeg;;
  esac
done
