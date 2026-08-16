#!/usr/bin/env python3
"""Contract checks for the Mac-local Peakhunter candidate builder."""

from __future__ import annotations

import json
import subprocess
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SCRIPT = ROOT / "scripts" / "build-peakhunter-candidate.sh"
OUTPUT_HELPER = ROOT / "scripts" / "peakhunter-candidate-output.sh"
CONFIG = ROOT / "desktop" / "src-tauri" / "tauri.peakhunter.conf.json"
PLIST = ROOT / "desktop" / "src-tauri" / "Info.peakhunter.plist"
BUILD_SOURCE = ROOT / "desktop" / "src-tauri" / "build.rs"
KEYRING_SOURCE = ROOT / "desktop" / "src-tauri" / "src" / "app_state_keyring.rs"
PACKAGE = ROOT / "desktop" / "package.json"


def require(text: str, needle: str) -> None:
    if needle not in text:
        raise AssertionError(f"missing contract fragment: {needle}")


def verify_exclusive_output_claim() -> None:
    with tempfile.TemporaryDirectory() as temporary:
        parent = Path(temporary)
        existing = parent / "existing"
        existing.mkdir()
        sentinel = existing / "sentinel"
        sentinel.write_text("owned elsewhere", encoding="utf-8")

        rejected = subprocess.run(
            [
                "bash",
                "-c",
                'source "$1"; OUTPUT_ROOT="$2"; claim_output_root',
                "bash",
                str(OUTPUT_HELPER),
                str(existing),
            ],
            check=False,
        )
        assert rejected.returncode != 0
        assert sentinel.read_text(encoding="utf-8") == "owned elsewhere"

        claimed = parent / "claimed"
        subprocess.run(
            [
                "bash",
                "-c",
                'source "$1"; OUTPUT_ROOT="$2"; claim_output_root; cleanup_output_root',
                "bash",
                str(OUTPUT_HELPER),
                str(claimed),
            ],
            check=True,
        )
        assert not claimed.exists()


def main() -> None:
    verify_exclusive_output_claim()
    source_version = json.loads(PACKAGE.read_text(encoding="utf-8"))["version"]
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
        "version_pattern": f"{source_version}-test.<source-sha12>",
    }

    script = SCRIPT.read_text(encoding="utf-8")
    build_source = BUILD_SOURCE.read_text(encoding="utf-8")
    keyring_source = KEYRING_SOURCE.read_text(encoding="utf-8")
    require(build_source, "BUZZ_BUILD_CANDIDATE_ID")
    require(build_source, "BUZZ_DESKTOP_BUILD_CANDIDATE_ID")
    require(keyring_source, 'format!("buzz-desktop-candidate.{candidate_id}")')
    require(script, 'KEYRING_SERVICE="buzz-desktop-candidate.$CANDIDATE_ID"')
    require(script, 'OUTPUT_ROOT="$REPO_ROOT/target/peakhunter-candidate/$SOURCE_SHA"')
    require(script, 'git status --porcelain --untracked-files=all')
    require(script, "':!desktop/src-tauri/target'")
    require(script, 'source "$REPO_ROOT/scripts/peakhunter-candidate-output.sh"')
    require(script, "claim_output_root")
    require(script, "cleanup_output_root")
    assert script.index("trap restore_versions EXIT") < script.index(
        "claim_output_root"
    )
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
    require(script, 'VERSION_PATHS_JSON="$VERSION_PATHS_JSON"')
    require(script, 'json.loads(os.environ["VERSION_PATHS_JSON"])')
    require(script, '"version_files_restamped"')
    require(script, '"bundle_manifest_sha256"')
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
