#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
cargo build --release --locked -p notsuperhuman
bundle=target/notsuperhuman.app
mkdir -p "$bundle/Contents/MacOS" "$bundle/Contents/Resources"
cp target/release/notsuperhuman "$bundle/Contents/MacOS/notsuperhuman"
cat > "$bundle/Contents/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleIdentifier</key><string>dev.nothq.notsuperhuman</string>
<key>CFBundleName</key><string>notsuperhuman</string>
<key>CFBundleDisplayName</key><string>notsuperhuman</string>
<key>CFBundleExecutable</key><string>notsuperhuman</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>CFBundleShortVersionString</key><string>0.1.0</string>
<key>CFBundleVersion</key><string>1</string>
<key>NSHighResolutionCapable</key><true/>
<key>NSPrincipalClass</key><string>NSApplication</string>
</dict></plist>
PLIST
codesign --force --deep --sign - "$bundle"
echo "Built $bundle"
