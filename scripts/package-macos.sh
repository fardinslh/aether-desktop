#!/bin/bash
set -euo pipefail
repo="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repo"
stage="$repo/src-tauri/target/macos-package"
app="$repo/src-tauri/target/release/bundle/macos/Aether Desktop.app"
out="${1:-$repo/src-tauri/target/release/bundle/macos}"
package_version="$(python3 -c 'import json; print(json.load(open("src-tauri/tauri.conf.json"))["version"])')"
package_path="$out/Aether-Desktop-$package_version-arm64.pkg"
[ "$(uname -m)" = arm64 ] || { echo 'This package targets Apple Silicon'; exit 1; }
[ -d "$app" ]
mkdir -p "$stage/root/Applications" "$stage/root/Library/PrivilegedHelperTools" "$stage/root/Library/LaunchDaemons" "$stage/root/Library/Application Support/AetherDesktop/router" "$out"
codesign --force --sign - "$app/Contents/MacOS/aether-router-helper"
codesign --force --sign - "$app"
codesign --verify --strict "$app"
cp -R "$app" "$stage/root/Applications/"
cp src-tauri/target/release/aether-router-helper "$stage/root/Library/PrivilegedHelperTools/com.aether.desktop.router"
cp src-tauri/resources/runtime/sing-box "$stage/root/Library/Application Support/AetherDesktop/router/sing-box"
cp src-tauri/macos/com.aether.desktop.router.plist "$stage/root/Library/LaunchDaemons/"
chmod 755 src-tauri/macos/pkg-scripts/postinstall
pkgbuild --component-plist src-tauri/macos/components.plist --root "$stage/root" --scripts src-tauri/macos/pkg-scripts --identifier com.aether.desktop.local --version "$package_version" --install-location / "$package_path"

[ -s "$package_path" ]
xar -tf "$package_path" >/dev/null
