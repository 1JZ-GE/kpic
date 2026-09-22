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
cp "$crate_dir/target/release/imgsqueeze" "$widget_dir/contents/daemon/imgsqueeze"
chmod +x "$widget_dir/contents/daemon/imgsqueeze"
echo "installed: widget + daemon at $widget_dir/contents/daemon/imgsqueeze"