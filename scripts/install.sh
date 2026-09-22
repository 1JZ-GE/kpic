#!/usr/bin/env bash
set -euo pipefail
# installs the kpic widget with its session dbus daemon bundled inside
crate_dir="$(cd "$(dirname "$0")/.." && pwd)"
data_dir="${XDG_DATA_HOME:-${HOME}/.local/share}"
widget_dir="$data_dir/plasma/plasmoids/org.kpic.compressor"
cargo build --release --manifest-path "$crate_dir/Cargo.toml"
# install or upgrade the widget package
kpackagetool6 -t Plasma/Applet -i "$crate_dir/plasmoid/package" 2>/dev/null \
  || kpackagetool6 -t Plasma/Applet -u "$crate_dir/plasmoid/package"
# ship the compiled daemon inside the widget dir; the widget spawns it on load
mkdir -p "$widget_dir/contents/daemon"
cp "$crate_dir/target/release/kpic" "$widget_dir/contents/daemon/kpic"
chmod +x "$widget_dir/contents/daemon/kpic"
# hardware msaa for the whole shell (smoother widgets, incl. this one);
# picked up on next login, applies to all plasmoids
env_dir="${XDG_CONFIG_HOME:-${HOME}/.config}/plasma-workspace/env"
mkdir -p "$env_dir"
if ! grep -q "QSG_ANTIALIASING_MSAA" "$env_dir/010-kpic-aa.sh" 2>/dev/null; then
  echo 'export QSG_ANTIALIASING_MSAA=8' >> "$env_dir/010-kpic-aa.sh"
fi
echo "installed: widget + daemon at $widget_dir/contents/daemon/kpic"