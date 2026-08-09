# ADR 0001 Companion: Functional Candidate v5 Change Disposition

- **Status:** Accepted decomposition snapshot
- **Assessment date:** 2026-08-09
- **Parent decision:** [ADR 0001: Hybrid Agent Runtime Architecture](0001-hybrid-agent-runtime-architecture.md)
- **Purpose:** Preserve exact provenance and define clean PR/implementation boundaries before v6 work

## Executive conclusion

The accepted v5 candidate is sufficiently preserved and understood to begin clean implementation work, but its composite worktrees are not suitable PR branches.

The candidate consists of:

1. exact existing PR content;
2. deliberate integrations of overlapping PRs;
3. small generally useful fixes discovered during acceptance;
4. a private candidate overlay;
5. monolithic runtime implementations superseded by ADR 0001;
6. deferred product work not implemented in v5.

v6 must be reconstructed from recorded clean bases and reviewed branches. The accepted v5 application remains the functional reference and rollback until a reconstructed candidate passes equivalent acceptance.

## Frozen assessment inputs

### Buzz

- Candidate base/HEAD: `5bf78671f45178f8de02ba18d3d321cbbf19cd1f`
- Current upstream main observed during assessment: `119a84897f225c1e3213a09cd149abb37dcb3abc`
- Candidate inventory: 59 changed tracked files, no untracked files
- Staged changes: 39 files
- Unstaged changes: 22 files
- Files with both staged and unstaged changes: 2
- `git diff --check HEAD`: clean

### codex-acp

- Candidate base/HEAD: `145ebba5d2030b4aa6d19cbb89d190b7b498d454`
- Candidate inventory: 15 status entries
- Tracked diff: 11 files, 437 insertions, 14 deletions
- Additional untracked implementation/tests: 4 files, 368 lines

### Review boundary

The assessment was read-only. It inspected staged, unstaged, and untracked state plus immutable PR heads. It did not edit, commit, push, build, install, launch, stop, or test the candidate.

## Live GitHub status at assessment time

| Work | State | Exact head | Disposition |
|---|---|---|---|
| `block/buzz#4913` shared-agent mentions | Merged | `fabbc9a4733f6f316c95874050db4f83da68ea8e` | Already upstream; do not replay |
| `block/buzz#4964` remote-owned agent ingestion/profile/tray | Open, mergeable | `a847f38886e9db0829470847ab593a39ddb87d33` | Preserve exact PR |
| `block/buzz#4983` DNS/signed delivery | Open draft, conflicting | `23565511392457b177ed47a25f8ed9bb6be8d652` | Reconcile or split; do not transplant composite ACP/config files |
| `block/buzz#5204` packaged mention boundary | Open, mergeable | `51cafdb0d55168aaffafc016bdf0acc585d780d6` | Preserve exact files; reconstruct editor overlap |
| `block/buzz#5386` persistent ACP system prompts | Open, mergeable | `1db51a13a60e09f0876c41e163fc082453c45456` | Rebase after ACP/delivery reconciliation |
| `agentclientprotocol/codex-acp#368` workspace/DNS behavior | Open, mergeable | `71d77db55a53c915cd9dce4247271f0c5d5f6054` | Refresh functional commits on current main plus #379 |
| `agentclientprotocol/codex-acp#379` persistent developer instructions | Open, mergeable | `1cf677dbe2801c4dbab678fc9e479a4a7fd87aa6` | Preserve and land first |
| `block/buzz#5342` restart session continuity | Open issue, not a PR | N/A | Deferred dedicated feature |

GitHub state is time-dependent. Exact heads above define this snapshot; re-query state before implementation or submission.

# Buzz disposition

## Exact existing PR: block/buzz#4964

All ten candidate working-tree blobs are byte-identical to the recorded PR head:

- `desktop/src/app/useTrayMenu.test.mjs`
- `desktop/src/app/useTrayMenu.ts`
- `desktop/src/features/agents/useAgentObserverIngestion.test.mjs`
- `desktop/src/features/agents/useAgentObserverIngestion.ts`
- `desktop/src/features/profile/hooks.ts`
- `desktop/src/features/profile/lib/identity.ts`
- `desktop/src/features/profile/ui/UserProfilePanel.tsx`
- `desktop/src/features/profile/ui/UserProfilePanelUtils.test.mjs`
- `desktop/src/features/profile/ui/UserProfilePanelUtils.ts`
- `desktop/tests/e2e/profile.spec.ts`

**Action:** use/rebase #4964. Do not reconstruct these files from v5.

## Exact existing PR plus editor overlap: block/buzz#5204

Exact candidate blobs:

- `desktop/playwright.config.ts`
- `desktop/src/features/messages/lib/mentionBoundaryBeforeInput.test.mjs`
- `desktop/src/features/messages/lib/mentionBoundaryBeforeInput.ts`
- `desktop/src/features/messages/lib/mentionHighlightExtension.ts`
- `desktop/src/features/messages/lib/useRichTextEditor.mentionBoundary.test.mjs`
- `desktop/tests/e2e/mention-caret-geometry.spec.ts`

Evolved integration:

- `desktop/src/features/messages/lib/useRichTextEditor.ts`

The editor file combines #5204's `beforeinput` integration with later snapshot-paste, link-preview, exact-URL paste, and preview-projection behavior.

**Action:** preserve the six exact files and reconstruct only the mention-boundary hunk against current editor source. Never replace the current editor wholesale with the old candidate blob.

## Exact existing PR plus ACP overlap: block/buzz#5386

Exact candidate blob:

- `crates/buzz-acp/src/pool.rs`

Evolved integration:

- `crates/buzz-acp/src/acp.rs`

The ACP file combines persistent-prompt capability negotiation and wire tests with #4983 delivery behavior and later candidate integration.

**Action:** preserve #5386's wire contract and tests, but rebase/reconcile its ACP hunk after the delivery lane is resolved.

## Exact existing PR plus policy-sensitive overlap: block/buzz#4983

Exact candidate blobs:

- `Cargo.toml`
- `crates/buzz-acp/Cargo.toml`
- `crates/buzz-acp/src/delivery.rs`
- `crates/buzz-acp/src/lib.rs`
- `crates/buzz-acp/src/relay.rs`
- `crates/buzz-cli/Cargo.toml`
- `crates/buzz-cli/src/client.rs`
- `crates/buzz-cli/src/delivery_broker.rs`
- `crates/buzz-core/src/delivery_broker.rs`
- `crates/buzz-core/src/lib.rs`

Exact coherent PR hunks in non-identical whole files:

- `.env.example`
- `Cargo.lock`
- `crates/buzz-cli/src/lib.rs`

Evolved integrations:

- `crates/buzz-acp/src/acp.rs`
- `crates/buzz-acp/src/config.rs`

`config.rs` contains a real security-policy conflict: the candidate/default text tends toward `bypassPermissions`, while the PR head describes `dontAsk`. This is not formatting and must not be inherited accidentally.

**Action:** reconcile or split #4983 on a clean base. Explicitly choose and test permission behavior. Regenerate `Cargo.lock` from the selected base.

## Already upstream: block/buzz#4913

The PR head is patch-equivalent to observed upstream main. No #4913 patch should be carried from v5.

The candidate's changed eligibility files are a later follow-up, not a replay:

- `desktop/src/features/agents/lib/agentAutocompleteEligibility.ts`
- `desktop/src/features/agents/lib/agentAutocompleteEligibility.test.mjs`

## New generally useful Buzz PRs

### Remote channel-member mention eligibility

Files:

- `desktop/src/features/agents/lib/agentAutocompleteEligibility.ts`
- `desktop/src/features/agents/lib/agentAutocompleteEligibility.test.mjs`

Purpose: integrate merged #4913 policy with #4964 remote-owned channel-agent hydration.

Gate: prove membership comes from authoritative channel/profile state and cannot admit an arbitrary relay-directory agent. Test explicit exclusion and non-member paths through the production picker.

### Passphrase test correctness

File:

- `desktop/src-tauri/src/key_backup_tests.rs`

Purpose: use an unambiguous delimiter because the EFF word list contains `yo-yo`.

Disposition: independent test-only PR.

### Bounded managed-agent shutdown

File:

- `desktop/src-tauri/src/shutdown.rs`

Purpose: bound shutdown and perform fallback orphan/system-process sweeps.

Gate: prove the detached worker cannot race fallback sweeps, mutate state after timeout, or retain resources. Cover success, returned error, panic/disconnect, timeout, and restart.

## Private candidate overlay

Keep these only for final private candidate composition unless later generalized deliberately:

- `desktop/scripts/verify-private-candidate-macos.sh`
- `desktop/src-tauri/Entitlements.private-candidate.plist`
- `desktop/src-tauri/Info.private-candidate.plist`
- `desktop/src-tauri/tauri.private-candidate.conf.json`
- `desktop/src-tauri/build.rs` private marker hunk
- `desktop/src-tauri/src/app_state.rs` visibility support
- `desktop/src-tauri/src/app_state_keyring.rs` private identity/keyring/filesystem portions
- `desktop/src-tauri/src/lib.rs` private startup hardening
- `desktop/src-tauri/src/managed_agents/managed_node_paths.rs` private namespace
- `desktop/src-tauri/src/managed_agents/nest.rs`
- `desktop/src-tauri/src/managed_agents/nest/tests.rs`
- `desktop/src-tauri/src/migration.rs`
- `desktop/src-tauri/src/migration_tests.rs`

Do not submit private names or machine-specific assumptions as product architecture.

## Buzz runtime implementation superseded by ADR 0001

The following candidate changes enforce a one-off monolithic bundled-Codex policy rather than the accepted hybrid architecture:

- `desktop/src-tauri/src/app_state_keyring.rs` executable resolver portions
- `desktop/src-tauri/src/commands/agent_discovery.rs`
- `desktop/src-tauri/src/managed_agents/discovery.rs`
- `desktop/src-tauri/src/managed_agents/discovery/runtime_metadata.rs`
- `desktop/src-tauri/src/managed_agents/discovery/tests.rs`
- `desktop/src-tauri/src/managed_agents/discovery/tests/managed_path_resolution.rs`
- `desktop/src-tauri/src/managed_agents/readiness.rs`
- `desktop/src-tauri/src/managed_agents/readiness/cli_login.rs`
- `desktop/src-tauri/src/managed_agents/runtime.rs`
- `desktop/src-tauri/src/managed_agents/runtime/tests.rs`

**Action:** do not upstream these implementations. Preserve their invariants in the replacement:

- missing or non-executable approved runtime fails closed;
- PATH, raw-command, and mutable managed substitutes cannot override approval;
- discovery, readiness, authentication, model discovery, and spawn identify the same runtime;
- private mode cannot silently install/reinstall mutable packages;
- ordinary/custom behavior remains compatible where policy permits;
- adapters and provider CLIs are distinct provider-family components;
- diagnostics identify selected source and remediation truthfully.

# codex-acp disposition

## Existing PR #379: preserve first

Exact #379 content is present coherently within the composite:

- `src/CodexAcpClient.ts`: system-prompt parsing, validation, and propagation
- `src/CodexAcpServer.ts`: metadata validation and `persistentSystemPrompt` capability
- `src/__tests__/CodexACPAgent/initialize.test.ts`: persistent prompt expectation
- `src/__tests__/CodexACPAgent/persistent-system-prompt.test.ts`: exact PR blob

**Action:** retain #379 as the first codex-acp dependency.

## Existing PR #368: refresh functional commits after #379

Evolved overlap is present in:

- `src/CodexAcpClient.ts`
- `src/CodexAcpServer.ts`
- `src/__tests__/CodexACPAgent/CodexAcpClient.test.ts`
- `src/__tests__/CodexACPAgent/initialize.test.ts`
- `src/__tests__/acp-test-utils.ts`

Preserve these invariants:

- network is enabled only for explicit boolean `true`;
- writable roots must be absolute strings;
- roots are deduplicated while additional directories remain;
- adding roots does not implicitly enable network;
- read-only and full-access modes remain unchanged;
- capability metadata advertises the exact supported shape.

**Action:** reconstruct #368's functional commits on current main plus #379. Do not use its old-base merge range as the feature boundary.

## New independent codex-acp PR: login status

Files:

- `src/login.ts`
- `src/__tests__/login.test.ts`

Behavior:

- noninteractive `login status` reports deterministic authenticated/logged-out success;
- status never initiates browser login;
- absent `CODEX_PATH` uses the bundled/approved runtime path rather than ambient `codex`;
- explicit selection remains testable until plan authority replaces it;
- connections and children are cleaned up on success and failure;
- interactive login remains unchanged.

This PR does not depend on native asset embedding; current base can resolve packaged JavaScript Codex through its ordinary connection path.

## codex-acp implementation superseded by ADR 0001

Do not submit the current embedded/materializing implementation:

- `src/CodexCli.ts`
- `src/CodexJsonRpcConnection.ts`
- `src/CodexExecutable.ts`
- `src/__tests__/compiled-bundle.test.ts`

Reasons:

- native assets are embedded only to be materialized elsewhere;
- cached files are reused based only on size rather than hash/signature;
- the helper target is unversioned;
- `CODEX_HOME` controls the materialization location;
- launch roles do not consume one immutable approved plan.

Preserve these replacement invariants:

- no PATH/raw-command fallback after approval;
- complete provider-family verification, including helper siblings;
- immutable package identity with hashes/signatures;
- readiness, login, model discovery, adapter spawn, provider spawn, and helpers consume the same plan;
- restart, state isolation, and rollback remain testable.

## Private codex-acp packaging overlay

Retain only for reproducing the accepted v5 artifact:

- `package.json` Darwin ARM64 entrypoint hunk
- `src/index.darwin-arm64.ts`
- `tsconfig.json` entrypoint exclusion

These are not the future product packaging contract.

# Deferred work

The following are intentionally absent from v5 and require clean implementation:

- immutable runtime-plan data contract and sole execution authority;
- removal of downstream raw-command and executable-environment bypasses;
- provider-family manifests and complete-family verification;
- profile-neutral content-addressed runtime store;
- bundled, Buzz-managed, verified-in-place, and verified-snapshot resolvers;
- adoption of legacy Buzz-managed packages;
- independently signed metadata, freshness, staged activation, and rollback;
- source/provenance/update/rollback UX;
- restart session continuity tracked by `block/buzz#5342`.

# Ordered implementation and PR stack

## Lane A: preserve accepted non-runtime behavior

1. Rebase/preserve `block/buzz#4964`.
2. Add the remote mention-eligibility follow-up.
3. Rebase `block/buzz#5204`, reconstructing only the editor overlap.
4. Reconcile or split `block/buzz#4983`; settle permission policy first.
5. Rebase `block/buzz#5386` onto the reconciled ACP source.
6. Submit passphrase test correctness independently.
7. Submit bounded shutdown only after lifecycle race tests pass.

## Lane B: codex-acp prerequisites

1. Land/preserve `agentclientprotocol/codex-acp#379`.
2. Refresh #368's functional commits on main plus #379.
3. Extract the independent login-status PR.
4. Do not upstream the embedded executable/materialization composite.

## Lane C: hybrid runtime architecture

1. Add the immutable runtime-plan contract and make it the sole authority.
2. Remove raw-command and executable-environment bypasses.
3. Add provider-family manifests and complete-family verification.
4. Add the content-addressed shared runtime store.
5. Add bundled, Buzz-managed, verified-in-place, and verified-snapshot resolvers.
6. Add legacy managed-runtime adoption.
7. Add signed staging, activation, update, freshness, and rollback.
8. Add provenance, source-selection, update, drift, and rollback UX.

## Lane D: candidate composition

1. Assemble exact reviewed PR heads from a recorded clean base.
2. Add only the minimal private overlay.
3. Build only after the restricted-runtime journey is red-capable and green on the reconstructed source.
4. Sign, verify, install separately, and run one Buzz application at a time.
5. Require equivalent functional, restart, runtime-path, state-isolation, signed-delivery, and rollback acceptance before replacing v5.

# Hard gates before v6 build

1. **Immutable provenance:** every product change comes from a reviewed clean branch, never the 59-file composite.
2. **Permission policy:** explicitly resolve `dontAsk` versus `bypassPermissions`; candidate defaults are not authority.
3. **Sole runtime authority:** every consumer receives the same immutable plan; no downstream rediscovery.
4. **Fail closed:** drift, missing components, hash/signature failure, denied executable-control variables, or incompatibility blocks execution with actionable state.
5. **Complete family:** adapter, provider CLI, siblings, helpers, and resources have component-level provenance and compatibility.
6. **Cross-repository ownership:** codex-acp owns generic Codex execution behavior; Buzz owns selection, installation, approval, and UX.
7. **Shared-interface regressions:** ordinary Buzz, custom commands, all providers, login, model discovery, restart, and installation paths remain covered.
8. **Lifecycle safety:** bounded shutdown cannot leave detached mutation or restart races.
9. **Restricted journey:** with external provider commands unavailable, readiness through user-visible response must prove the exact approved runtime path before packaging.
10. **Private overlay last:** isolated identity, keyring, data, nest, URL handling, permissions, executable allowlist, signing, and rollback are verified only after product branches are fixed.

# Readiness decision

The repository state is tidy enough to begin clean branch implementation under this matrix.

It is not yet appropriate to build v6. The first implementation work should establish clean branches and tests for the chosen lane; candidate packaging remains last.
