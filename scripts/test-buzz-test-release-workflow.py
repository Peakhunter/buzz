#!/usr/bin/env python3
"""Dependency-free static contract for the Peakhunter Buzz test-release workflow."""

from __future__ import annotations

import re
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
WORKFLOW = ROOT / ".github" / "workflows" / "buzz-test-release.yml"
CONTRACT_WORKFLOW = ROOT / ".github" / "workflows" / "buzz-test-release-contract.yml"


def require(condition: bool, message: str) -> None:
    if not condition:
        raise SystemExit(message)


def between(text: str, start: str, end: str | None = None) -> str:
    require(start in text, f"missing section: {start.strip()}")
    value = text[text.index(start) :]
    if end is not None:
        require(end in value[len(start) :], f"missing section boundary: {end.strip()}")
        value = value[: value.index(end, len(start))]
    return value


def pinned_actions(text: str) -> None:
    for match in re.finditer(r"uses:\s*([^\s#]+)", text):
        action = match.group(1)
        require(re.fullmatch(r"[^@]+@[0-9a-f]{40}", action) is not None, f"action is not pinned: {action}")


def main() -> None:
    require(WORKFLOW.exists(), "Buzz test-release workflow is missing")
    require(CONTRACT_WORKFLOW.exists(), "path-scoped Buzz test-release contract workflow is missing")
    text = WORKFLOW.read_text(encoding="utf-8")
    contract_text = CONTRACT_WORKFLOW.read_text(encoding="utf-8")

    contract_trigger = between(contract_text, "on:\n", "permissions:\n")
    require("pull_request:" in contract_trigger, "contract workflow must be PR-only")
    require("workflow_dispatch:" not in contract_trigger and "push:" not in contract_trigger, "contract workflow must be PR-only")
    paths_match = re.search(r"(?m)^    paths:\n((?:      - .+\n)+)", contract_trigger)
    if paths_match is None:
        raise SystemExit("contract workflow must declare path filters")
    paths = {line.removeprefix("      - ") for line in paths_match.group(1).splitlines()}
    require(
        paths
        == {
            ".github/workflows/buzz-test-release.yml",
            ".github/workflows/buzz-test-release-contract.yml",
            "scripts/test-buzz-test-release-workflow.py",
        },
        "contract workflow must be scoped to its three infrastructure files",
    )
    require("permissions:\n  contents: read\n" in contract_text, "contract workflow must be read-only")
    require("jobs:\n  contract:\n" in contract_text, "contract workflow must have one job")
    require(contract_text.count("\n  contract:\n") == 1, "contract workflow must have one job")
    require("runs-on: ubuntu-latest" in contract_text, "contract must use a cheap Linux runner")
    require("timeout-minutes: 2" in contract_text, "contract must have a two-minute timeout")
    require("run: python3 scripts/test-buzz-test-release-workflow.py" in contract_text, "contract workflow must execute the static contract")

    trigger = between(text, "on:\n", "concurrency:\n")
    require("workflow_dispatch:" in trigger, "test releases must be manually dispatched")
    require("pull_request:" not in trigger and "push:" not in trigger, "test releases must be manual-only")
    require("source_sha:" in trigger and "required: true" in trigger, "source_sha must be required")

    default_permissions = between(text, "permissions:\n", "jobs:\n").strip()
    require(default_permissions == "permissions:\n  contents: read", "default permissions must be contents: read")
    require("group: buzz-test-release-${{ inputs.source_sha }}" in text, "concurrency must use source_sha")
    require("cancel-in-progress: false" in text, "release builds must not cancel one another")

    job_names = re.findall(r"(?m)^  ([a-z][a-z0-9_]*):\n    name:", between(text, "jobs:\n"))
    require(job_names == ["verify_source", "build", "publish"], "expected verify_source, build, and publish jobs")

    verify = between(text, "\n  verify_source:\n", "\n  build:\n")
    build = between(text, "\n  build:\n", "\n  publish:\n")
    publish = between(text, "\n  publish:\n")

    require("if: github.repository == 'Peakhunter/buzz'" in verify, "workflow must be restricted to Peakhunter/buzz")
    require("test releases must be dispatched from refs/heads/main" in verify, "publishing workflow authority must come from main")
    require(re.search(r"\^\[0-9a-f\]\{40\}\$", verify) is not None, "source_sha must be validated as a full lowercase SHA")
    require("requested source SHA does not match checked out commit" in verify, "checkout identity must be compared with source_sha")
    require("test release already exists; refusing an expensive rebuild" in verify, "existing releases must fail in cheap preflight")

    require("needs: verify_source" in build, "macOS build must wait for source verification")
    require("runs-on: macos-15" in build, "test release must use the documented ARM64 macOS label")
    require("permissions:\n      contents: read" in build, "build job must remain read-only")
    require("Verify Apple Silicon runner and toolchain" in build, "build must verify runner architecture")
    require("uname -m" in build and "aarch64-apple-darwin" in build, "build must fail closed unless host is ARM64")
    require("CFBundleExecutable" in build and "lipo -archs" in build, "packaged executable architecture must be verified")
    require(build.index("lipo -archs") < build.index("ditto -c -k"), "architecture verification must precede packaging")

    require("needs: build" in publish, "publication must wait for the successful build")
    require("runs-on: ubuntu-latest" in publish, "publication should use a cheap Linux runner")
    require("permissions:\n      actions: read\n      contents: write" in publish, "only publish may write repository contents")

    required_fragments = (
        "Restore pnpm store cache",
        "Save pnpm store cache",
        "Restore Cargo downloads cache",
        "Save Cargo downloads cache",
        "Restore compiler outputs cache",
        "Save compiler outputs cache",
        "cache-matched-key",
        "${{ needs.verify_source.outputs.source_sha }}",
        "restore-keys:",
        "Build unsigned Tauri app",
        "cd desktop\n          node scripts/set-version-from-tag.mjs",
        '"productName": "Buzz Test Release"',
        '"identifier": "xyz.block.buzz.app.dev.test-release"',
        '"schemes": ["buzz-test-release"]',
        "BUZZ_BUILD_CANDIDATE_ID: test-release",
        '"endpoints": []',
        "createUpdaterArtifacts\": false",
        'APP="desktop/src-tauri/target/release/bundle/macos/Buzz Test Release.app"',
        "codesign --force --deep --sign -",
        "codesign --verify --deep --strict",
        '"signed": True',
        '"signing": "ad_hoc"',
        '"candidate_id": "test-release"',
        '"keyring_service": "buzz-desktop-candidate.test-release"',
        '"state_root": "~/.buzz-candidate-test-release"',
        '"updater_enabled": False',
        "manifest.json",
        "SHA256SUMS.txt",
        "unexpected handoff files",
        "manifest identity does not match trusted build outputs",
        "unexpected checksum entries",
        "actions/upload-artifact@",
        "actions/download-artifact@",
        "gh release create",
        "--method POST \"repos/$GITHUB_REPOSITORY/git/refs\"",
        "-f ref=\"refs/tags/$TAG\"",
        "--draft",
        "--prerelease",
        "--target \"$SOURCE_SHA\"",
        "gh release view",
        "immutable test release already exists",
        "git/tags/$OBJECT_SHA",
        "test release tag resolves to wrong commit",
        "draft=false",
        "names != expected",
        "Buzz_test_",
        "macos-arm64_adhoc.zip",
    )
    for fragment in required_fragments:
        require(fragment in text, f"workflow is missing required contract fragment: {fragment}")

    require("retention-days: 1" in build, "internal handoff artifact must be short-lived")
    require('"signed": True' in build, "manifest generator must disclose ad-hoc signed status")
    require("$GITHUB_STEP_SUMMARY" in text, "cache and release telemetry must be summarized")
    require("id: cache_keys" in build, "cache keys must be frozen before build-time lockfile changes")
    require(
        text.count("steps.cache_keys.outputs.pnpm_key") == 2
        and text.count("steps.cache_keys.outputs.cargo_key") == 2
        and text.count("steps.cache_keys.outputs.compiler_key") == 2,
        "restore and save must reuse the same frozen cache keys",
    )
    require(
        "            rust-compiler-${{ runner.os }}-arm64-\n" not in build,
        "compiler restore must not cross frozen toolchain and lockfile scope",
    )
    require(
        "runner.arch" in build
        and "macos10.15" in build
        and text.count("steps.cache_keys.outputs.mesh_scope") == 2,
        "mesh native cache must freeze OS, architecture, deployment target, toolchain, and dependency scope",
    )

    build_step = build.index("Build unsigned Tauri app")
    for save_name in ("Save pnpm store cache", "Save Cargo downloads cache", "Save compiler outputs cache"):
        require(build.index(save_name) > build_step, "caches may save only after a successful build")
    require(build.count("continue-on-error: true") == 4, "cache service failures must not block a valid release")
    require(build.count("if: success() && steps.") == 4, "cache saves must remain success-only")

    publish_order = (
        publish.index("Download verified build handoff"),
        publish.index("Verify release assets"),
        publish.index("Create draft and publish prerelease"),
        publish.index("gh release create"),
        publish.index("--method POST \"repos/$GITHUB_REPOSITORY/git/refs\""),
        publish.index("RELEASE_JSON="),
        publish.index("git/ref/tags/$TAG", publish.index("RELEASE_JSON=")),
        publish.index("draft=false"),
    )
    require(list(publish_order) == sorted(publish_order), "publication boundary steps are out of order")

    pinned_actions(text)
    pinned_actions(contract_text)
    print("Buzz test-release workflow contract passed")


if __name__ == "__main__":
    main()
