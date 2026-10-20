#!/bin/bash
# Build the release binary and assemble a locally runnable .app bundle.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

CONFIGURATION="release"
APP_NAME="SPACE-INVADERS"
APP_DIR="$ROOT/dist/$APP_NAME.app"

echo "→ Building release binary for $(uname -m)…"
cargo build --release --locked -p space-invaders-app

BINARY="$ROOT/target/$CONFIGURATION/SpaceInvaders"
if [[ ! -x "$BINARY" ]]; then
    echo "Build did not produce $BINARY" >&2
    exit 1
fi

rm -rf "$APP_DIR"
mkdir -p "$APP_DIR/Contents/MacOS" "$APP_DIR/Contents/Resources"

cp "$BINARY" "$APP_DIR/Contents/MacOS/SpaceInvaders"
cp "$ROOT/Resources/Info.plist" "$APP_DIR/Contents/Info.plist"
cp "$ROOT/Resources/AppIcon.icns" "$APP_DIR/Contents/Resources/AppIcon.icns"
printf 'APPL????' > "$APP_DIR/Contents/PkgInfo"

chmod +x "$APP_DIR/Contents/MacOS/SpaceInvaders"

# Ad-hoc signing is enough for a locally built app. It is not a Developer ID
# signature and the bundle is not notarized for general distribution.
codesign --force --deep --sign - "$APP_DIR" >/dev/null 2>&1 || true

echo "→ App bundle: $APP_DIR"
echo "  open \"$APP_DIR\""
