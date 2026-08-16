#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$REPO_ROOT/scripts/peakhunter-candidate-output.sh"
PRODUCT_NAME="Buzz Peakhunter"
CANDIDATE_ID="peakhunter"
BUNDLE_ID="xyz.block.buzz.app.dev.peakhunter"
URL_SCHEME="buzz-peakhunter"
KEYRING_SERVICE="buzz-desktop-candidate.$CANDIDATE_ID"
CONFIG="src-tauri/tauri.peakhunter.conf.json"
SOURCE_VERSION="$(node -p "require('$REPO_ROOT/desktop/package.json').version")"

if [[ "${1:-}" == "--print-contract" ]]; then
  printf '{"bundle_id":"%s","candidate_id":"%s","keyring_service":"%s","product_name":"%s","url_scheme":"%s","version_pattern":"%s-test.<source-sha12>"}\n' \
    "$BUNDLE_ID" "$CANDIDATE_ID" "$KEYRING_SERVICE" "$PRODUCT_NAME" "$URL_SCHEME" "$SOURCE_VERSION"
  exit 0
fi
if [[ $# -ne 0 ]]; then
  echo "usage: $0 [--print-contract]" >&2
  exit 2
fi

cd "$REPO_ROOT"
WORKTREE_STATUS="$(git status --porcelain --untracked-files=all -- ':!target' ':!desktop/src-tauri/target')"
if [[ -n "$WORKTREE_STATUS" ]]; then
  echo "candidate build requires a clean worktree, including untracked files" >&2
  printf '%s\n' "$WORKTREE_STATUS" >&2
  exit 1
fi
: "${BUZZ_RELAY_URL:?set BUZZ_RELAY_URL explicitly for the candidate build}"
SOURCE_SHA="$(git rev-parse HEAD)"
SHORT_SHA="${SOURCE_SHA:0:12}"
VERSION="$SOURCE_VERSION-test.$SHORT_SHA"
OUTPUT_ROOT="$REPO_ROOT/target/peakhunter-candidate/$SOURCE_SHA"
UNSIGNED_APP="$REPO_ROOT/desktop/src-tauri/target/release/bundle/macos/$PRODUCT_NAME.app"
APP="$OUTPUT_ROOT/$PRODUCT_NAME.app"
ZIP="$OUTPUT_ROOT/Buzz_Peakhunter_${SHORT_SHA}_macos-arm64_apple-development.zip"
MANIFEST="$OUTPUT_ROOT/manifest.json"
SUMS="$OUTPUT_ROOT/SHA256SUMS.txt"
BUNDLE_MANIFEST="$OUTPUT_ROOT/Buzz_Peakhunter.app.SHA256MANIFEST.txt"
ENTITLEMENTS="$REPO_ROOT/desktop/src-tauri/Entitlements.plist"

SIGNING_IDENTITIES=()
while IFS= read -r identity; do
  [[ -n "$identity" ]] && SIGNING_IDENTITIES+=("$identity")
done < <(
  security find-identity -v -p codesigning 2>/dev/null |
    sed -n 's/^[[:space:]]*[0-9][0-9]*) \([0-9A-F]\{40\}\) "Apple Development:.*$/\1/p'
)
if [[ ${#SIGNING_IDENTITIES[@]} -ne 1 ]]; then
  echo "expected exactly one valid Apple Development signing identity; found ${#SIGNING_IDENTITIES[@]}" >&2
  exit 1
fi
SIGNING_IDENTITY="${SIGNING_IDENTITIES[0]}"

VERSION_PATHS=(
  desktop/package.json
  desktop/src-tauri/tauri.conf.json
  desktop/src-tauri/Cargo.toml
  desktop/src-tauri/Cargo.lock
)
VERSION_PATHS_JSON="$(printf '%s\n' "${VERSION_PATHS[@]}" | python3 -c 'import json, sys; print(json.dumps([line.rstrip("\n") for line in sys.stdin]))')"
restore_versions() {
  git restore --source=HEAD -- "${VERSION_PATHS[@]}" || true
  cleanup_output_root
}
trap restore_versions EXIT
claim_output_root

(
  cd desktop
  node scripts/set-version-from-tag.mjs "$VERSION"
)

export BUZZ_BUILD_CANDIDATE_ID="$CANDIDATE_ID"
MESH_LLM_NATIVE_RUNTIME_CACHE_DIR="$(./scripts/ensure-mesh-native-runtime.sh)"
export MESH_LLM_NATIVE_RUNTIME_CACHE_DIR

cargo build --release \
  -p buzz-acp -p buzz-agent -p buzz-backend-kubernetes \
  -p buzz-dev-mcp -p git-credential-nostr -p buzz-cli
./scripts/bundle-sidecars.sh
pnpm install --frozen-lockfile
(
  cd desktop
  pnpm tauri build --no-sign --features mesh-llm --config "$CONFIG"
)

[[ -d "$UNSIGNED_APP" ]] || { echo "missing unsigned app: $UNSIGNED_APP" >&2; exit 1; }
ditto "$UNSIGNED_APP" "$APP"

while IFS= read -r -d '' nested; do
  codesign --force --sign "$SIGNING_IDENTITY" --options runtime --timestamp=none "$nested"
done < <(find "$APP/Contents/MacOS" -depth -type f -perm -111 -print0)
codesign --force --sign "$SIGNING_IDENTITY" --options runtime --timestamp=none --entitlements "$ENTITLEMENTS" "$APP"
codesign --verify --deep --strict --verbose=2 "$APP"

ACTUAL_ID="$(/usr/libexec/PlistBuddy -c 'Print :CFBundleIdentifier' "$APP/Contents/Info.plist")"
ACTUAL_NAME="$(/usr/libexec/PlistBuddy -c 'Print :CFBundleDisplayName' "$APP/Contents/Info.plist")"
ACTUAL_SHORT_VERSION="$(/usr/libexec/PlistBuddy -c 'Print :CFBundleShortVersionString' "$APP/Contents/Info.plist")"
ACTUAL_BUILD_VERSION="$(/usr/libexec/PlistBuddy -c 'Print :CFBundleVersion' "$APP/Contents/Info.plist")"
[[ "$ACTUAL_ID" == "$BUNDLE_ID" ]] || { echo "bundle identifier mismatch: expected $BUNDLE_ID, got $ACTUAL_ID" >&2; exit 1; }
[[ "$ACTUAL_NAME" == "$PRODUCT_NAME" ]] || { echo "display name mismatch: expected $PRODUCT_NAME, got $ACTUAL_NAME" >&2; exit 1; }
[[ "$ACTUAL_SHORT_VERSION" == "$VERSION" ]] || { echo "short version mismatch: expected $VERSION, got $ACTUAL_SHORT_VERSION" >&2; exit 1; }
[[ "$ACTUAL_BUILD_VERSION" == "$VERSION" ]] || { echo "build version mismatch: expected $VERSION, got $ACTUAL_BUILD_VERSION" >&2; exit 1; }

TEAM_ID="$(codesign -dvv "$APP" 2>&1 | sed -n 's/^TeamIdentifier=//p')"
[[ "$TEAM_ID" =~ ^[A-Z0-9]{10}$ ]] || { echo "invalid or missing signing Team ID: $TEAM_ID" >&2; exit 1; }
SIGNATURE_FLAGS="$(codesign -dvv "$APP" 2>&1 | sed -n 's/^CodeDirectory .* flags=\([^ ]*\).*/\1/p')"
[[ "$SIGNATURE_FLAGS" == *runtime* ]] || { echo "outer app signature is missing hardened-runtime flag: $SIGNATURE_FLAGS" >&2; exit 1; }
EXECUTABLE_NAME="$(/usr/libexec/PlistBuddy -c 'Print :CFBundleExecutable' "$APP/Contents/Info.plist")"
EXECUTABLE="$APP/Contents/MacOS/$EXECUTABLE_NAME"
ARCHITECTURE="$(lipo -archs "$EXECUTABLE")"
[[ "$ARCHITECTURE" == "arm64" ]] || { echo "candidate architecture mismatch: expected arm64, got $ARCHITECTURE" >&2; exit 1; }
EXECUTABLE_SHA256="$(shasum -a 256 "$EXECUTABLE" | cut -d' ' -f1)"
ENTITLEMENTS_SHA256="$(shasum -a 256 "$ENTITLEMENTS" | cut -d' ' -f1)"
NESTED_SIGNED_COUNT="$(find "$APP/Contents/MacOS" -type f -perm -111 | wc -l | tr -d ' ')"

(
  cd "$APP"
  find . -type f -print0 | LC_ALL=C sort -z | while IFS= read -r -d '' file; do
    shasum -a 256 "$file"
  done
) > "$BUNDLE_MANIFEST"
BUNDLE_MANIFEST_SHA256="$(shasum -a 256 "$BUNDLE_MANIFEST" | cut -d' ' -f1)"

xcrun stapler validate "$APP" >/dev/null 2>&1 && NOTARIZATION="stapled" || NOTARIZATION="not_stapled"
ditto -c -k --sequesterRsrc --keepParent "$APP" "$ZIP"
ZIP_SHA256="$(shasum -a 256 "$ZIP" | cut -d' ' -f1)"

SOURCE_SHA="$SOURCE_SHA" VERSION="$VERSION" OUTPUT_ROOT="$OUTPUT_ROOT" ZIP="$ZIP" \
ZIP_SHA256="$ZIP_SHA256" EXECUTABLE_SHA256="$EXECUTABLE_SHA256" \
ENTITLEMENTS_SHA256="$ENTITLEMENTS_SHA256" TEAM_ID="$TEAM_ID" \
NESTED_SIGNED_COUNT="$NESTED_SIGNED_COUNT" NOTARIZATION="$NOTARIZATION" \
PRODUCT_NAME="$PRODUCT_NAME" BUNDLE_ID="$BUNDLE_ID" URL_SCHEME="$URL_SCHEME" \
CANDIDATE_ID="$CANDIDATE_ID" KEYRING_SERVICE="$KEYRING_SERVICE" \
ARCHITECTURE="$ARCHITECTURE" VERSION_PATHS_JSON="$VERSION_PATHS_JSON" \
BUNDLE_MANIFEST="$BUNDLE_MANIFEST" BUNDLE_MANIFEST_SHA256="$BUNDLE_MANIFEST_SHA256" \
python3 - <<'PY'
import json
import os
from pathlib import Path

root = Path(os.environ["OUTPUT_ROOT"])
manifest = {
    "schema_version": 1,
    "source_sha": os.environ["SOURCE_SHA"],
    "source_tree_at_start": "clean",
    "version_files_restamped": json.loads(os.environ["VERSION_PATHS_JSON"]),
    "product_name": os.environ["PRODUCT_NAME"],
    "bundle_id": os.environ["BUNDLE_ID"],
    "url_scheme": os.environ["URL_SCHEME"],
    "candidate_id": os.environ["CANDIDATE_ID"],
    "keyring_service": os.environ["KEYRING_SERVICE"],
    "version": os.environ["VERSION"],
    "architecture": os.environ["ARCHITECTURE"],
    "app_path": str(root / f'{os.environ["PRODUCT_NAME"]}.app'),
    "archive_path": os.environ["ZIP"],
    "archive_sha256": os.environ["ZIP_SHA256"],
    "executable_sha256": os.environ["EXECUTABLE_SHA256"],
    "bundle_manifest_path": os.environ["BUNDLE_MANIFEST"],
    "bundle_manifest_sha256": os.environ["BUNDLE_MANIFEST_SHA256"],
    "signing": {
        "kind": "Apple Development",
        "team_id": os.environ["TEAM_ID"],
        "hardened_runtime_verification": "passed",
        "nested_executable_count": int(os.environ["NESTED_SIGNED_COUNT"]),
        "strict_verification": "passed",
        "entitlements_sha256": os.environ["ENTITLEMENTS_SHA256"],
        "notarization": os.environ["NOTARIZATION"],
    },
}
(root / "manifest.json").write_text(
    json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8"
)
PY

(
  cd "$OUTPUT_ROOT"
  shasum -a 256 "$(basename "$ZIP")" "$(basename "$BUNDLE_MANIFEST")" manifest.json > SHA256SUMS.txt
)
BUILD_COMPLETE=true

echo "Candidate: $APP"
echo "Manifest:  $MANIFEST"
echo "Checksums: $SUMS"
echo "Version:   $VERSION"
echo "Team ID:   $TEAM_ID"
