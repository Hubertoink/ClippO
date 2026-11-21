#!/usr/bin/env bash
set -euo pipefail
source_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
if [[ ! -f "$source_dir/manifest.json" && -f /usr/share/clippo/omarchy/manifest.json ]]; then
  source_dir=/usr/share/clippo/omarchy
fi
plugin_id=hubertoink.clippo
config_dir="${XDG_CONFIG_HOME:-$HOME/.config}/omarchy"
destination="$config_dir/plugins/$plugin_id"

omarchy plugin validate "$source_dir"
mkdir -p "$config_dir/backups" "$destination/i18n"
if [[ -f "$config_dir/shell.json" ]]; then
  cp -p "$config_dir/shell.json" "$config_dir/backups/shell.clippo.$(date +%Y%m%d-%H%M%S).json"
fi
if [[ -f "$destination/manifest.json" ]]; then
  tar -czf "$config_dir/backups/clippo.$(date +%Y%m%d-%H%M%S).tar.gz" -C "$destination" .
fi
install -m 644 "$source_dir/manifest.json" "$destination/manifest.json"
install -m 644 "$source_dir/Main.qml" "$destination/Main.qml"
install -m 644 "$source_dir/i18n/en.json" "$destination/i18n/en.json"
install -m 644 "$source_dir/i18n/de.json" "$destination/i18n/de.json"
omarchy-shell shell rescanPlugins
if ! jq -e --arg id "$plugin_id" '[.bar.layout[][]? | select(.id == $id)] | length > 0' \
  "$config_dir/shell.json" >/dev/null; then
  omarchy plugin enable "$plugin_id" --section right
  omarchy bar move "$plugin_id" --section right --before omarchy.bluetooth
fi
echo "ClippO installiert. Klicke auf das ClippO-Symbol in der Leiste oder ziehe Dateien darauf."
