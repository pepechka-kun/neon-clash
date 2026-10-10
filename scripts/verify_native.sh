#!/bin/zsh
# Launch through macOS LaunchServices: a background terminal can leave Metal's
# drawable occluded, which the existing AIGE renderer correctly skips.
set -euo pipefail
GAME_ROOT="${0:A:h:h}"
cd "$GAME_ROOT"
if ioreg -l -n Root -d 1 | rg '"CGSSessionScreenIsLocked"\s*=\s*Yes' > /dev/null; then
    print -u2 "Нативная проверка недоступна: экран macOS заблокирован. Разблокируйте экран и повторите scripts/verify_native.sh."
    exit 3
fi
GAME_TARGET="${CARGO_TARGET_DIR:-$GAME_ROOT/target}"
export CARGO_TARGET_DIR="$GAME_TARGET"
cargo build --offline --release --example native_smoke
CHECK_BUNDLE="$GAME_ROOT/artifacts/Native Check.app"
mkdir -p "$CHECK_BUNDLE/Contents/MacOS"
cp "$GAME_TARGET/release/examples/native_smoke" "$CHECK_BUNDLE/Contents/MacOS/NativeCheck"
python3 - "$CHECK_BUNDLE" <<'PY'
import pathlib, plistlib, sys
root = pathlib.Path(sys.argv[1])
with open("scripts/Info.plist", "rb") as f:
    info = plistlib.load(f)
info.update(CFBundleExecutable="NativeCheck", CFBundleIdentifier="games.aige.neonclash.check", CFBundleName="Neon Clash Check")
with open(root / "Contents/Info.plist", "wb") as f:
    plistlib.dump(info, f)
PY
codesign --force --sign - "$CHECK_BUNDLE"
rm -f artifacts/native-report.json
open -W -n "$CHECK_BUNDLE" --stdout "$GAME_ROOT/artifacts/native-stdout.log" --stderr "$GAME_ROOT/artifacts/native-stderr.log"
python3 - <<'PY'
import json, pathlib
p = pathlib.Path("artifacts/native-report.json")
if not p.exists():
    raise SystemExit(pathlib.Path("artifacts/native-stderr.log").read_text())
r = json.loads(p.read_text())
assert r["frames_presented"] > 100 and r["render_errors"] == r["tick_errors"] == 0
print(json.dumps(r, indent=2))
PY
