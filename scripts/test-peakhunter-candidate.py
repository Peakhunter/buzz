#!/usr/bin/env python3
"""Contract checks for the Mac-local Peakhunter candidate builder."""

from __future__ import annotations

import json
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SCRIPT = ROOT / "scripts" / "build-peakhunter-candidate.sh"
CONFIG = ROOT / "desktop" / "src-tauri" / "tauri.peakhunter.conf.json"
PLIST = ROOT / "desktop" / "src-tauri" / "Info.peakhunter.plist"


def require(text: str, needle: str) -> None:
    if needle not in text:
        raise AssertionError(f"missing contract fragment: {needle}")


def main() -> None:
    config = json.loads(CONFIG.read_text(encoding="utf-8"))
    assert config["productName"] == "Buzz Peakhunter"
    assert config["identifier"] == "xyz.block.buzz.app.dev.peakhunter"
    assert config["plugins"]["deep-link"]["desktop"]["schemes"] == [
        "buzz-peakhunter"
    ]
    assert config["plugins"]["updater"]["endpoints"] == []
    assert config["bundle"]["createUpdaterArtifacts"] is False
    assert config["bundle"]["macOS"]["infoPlist"] == "Info.peakhunter.plist"
    assert config["bundle"]["macOS"]["entitlements"] == "Entitlements.plist"

    plist = PLIST.read_text(encoding="utf-8")
    require(plist, "<string>Buzz Peakhunter</string>")

    contract = json.loads(
        subprocess.check_output([str(SCRIPT), "--print-contract"], text=True)
    )
    assert contract == {
        "bundle_id": "xyz.block.buzz.app.dev.peakhunter",
        "candidate_id": "peakhunter",
        "keyring_service": "buzz-desktop-candidate.peakhunter",
        "product_name": "Buzz Peakhunter",
        "url_scheme": "buzz-peakhunter",
        "version_pattern": "0.5.11-test.<source-sha12>",
    }

    script = SCRIPT.read_text(encoding="utf-8")
    require(script, 'OUTPUT_ROOT="$REPO_ROOT/target/peakhunter-candidate/$SOURCE_SHA"')
    require(script, 'git status --porcelain --untracked-files=all')
    require(script, "':!desktop/src-tauri/target'")
    require(script, 'if [[ "$BUILD_COMPLETE" != true ]]; then')
    require(script, 'rm -rf "$OUTPUT_ROOT"')
    assert script.index("trap restore_versions EXIT") < script.index('mkdir -p "$OUTPUT_ROOT"')
    if "/Applications" in script:
        raise AssertionError("candidate builder must never write to /Applications")
    require(script, 'export BUZZ_BUILD_CANDIDATE_ID="$CANDIDATE_ID"')
    require(script, ': "${BUZZ_RELAY_URL:?')
    if "buzz.peakhunter.com" in script:
        raise AssertionError("candidate builder must not commit a default relay endpoint")
    require(script, 'codesign --force --sign "$SIGNING_IDENTITY"')
    require(script, '--entitlements "$ENTITLEMENTS" "$APP"')
    require(script, 'find "$APP/Contents/MacOS" -depth')
    require(script, 'ARCHITECTURE="$(lipo -archs "$EXECUTABLE")"')
    require(script, "codesign --verify --deep --strict")
    require(script, 'PRODUCT_NAME="$PRODUCT_NAME" BUNDLE_ID="$BUNDLE_ID"')
    require(script, 'KEYRING_SERVICE="$KEYRING_SERVICE"')
    require(script, '"version_files_restamped"')
    if '"source_dirty": False' in script:
        raise AssertionError("manifest must not hardcode source_dirty")
    if '"strict_verification": True' in script:
        raise AssertionError("manifest must not hardcode strict verification")
    if '"pending_disposable_launch"' in script:
        raise AssertionError("manifest must not claim pending notification verification")
    require(script, "manifest.json")
    require(script, "SHA256SUMS.txt")

    print("Peakhunter candidate packaging contract passed")


if __name__ == "__main__":
    main()
