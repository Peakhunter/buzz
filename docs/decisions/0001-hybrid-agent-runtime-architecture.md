# ADR 0001: Hybrid Agent Runtime Architecture

- **Status:** Accepted; implementation not started
- **Decision date:** 2026-08-09
- **Scope:** Buzz desktop managed-agent runtimes and ACP adapters
- **Implementation disposition:** [Functional Candidate v5 change disposition](0001-hybrid-agent-runtime-change-disposition.md)

## Context

Buzz supports agent families with different runtime shapes: Buzz Agent ships with the app, Goose speaks ACP directly, while Claude Code and Codex combine provider CLIs with ACP adapters and, for Codex, native sibling helpers and resources.

Buzz already has a managed-runtime catalog and conditional installer. It can discover an existing provider CLI, install missing components, install or update an ACP adapter in a Buzz-managed npm prefix, verify availability, and restart eligible setup-mode agents. That is a useful foundation, but availability is not deterministic execution. Existing recipes may use moving installer endpoints or unpinned packages, while readiness, login, model discovery, spawn, and child execution can independently resolve commands. Saved raw commands and inherited executable-selection environment variables can bypass an approved runtime choice.

A functional macOS candidate proved that a hermetic runtime can work end to end and after a clean application restart. It also exposed the costs of treating every runtime as part of the desktop bundle: substantial application growth, native binaries dominating size, duplicate materialized files, and provider updates coupled to desktop releases.

The candidate also showed that one provider family can combine components with different provenance. An external Claude Code CLI can work with an ACP adapter already installed in Buzz's managed prefix. Discovery must therefore distinguish "not visible to this profile" from "not installed."

The product must support all intended installation modes rather than optimize for one package manager, provider, or machine.

## Decision

Buzz will use a **hybrid runtime architecture** with three supported sources:

1. **Bundled:** a known-good baseline shipped with the signed Buzz application.
2. **Buzz-managed:** independently versioned, signed, content-addressed runtime packages installed and updated by Buzz.
3. **Verified external:** an existing provider installation explicitly selected and verified by Buzz.

All three sources resolve to the same immutable execution contract. Downstream consumers must not reconstruct commands or silently discover replacements. Buzz will extend its existing managed-runtime framework rather than introduce a parallel package manager.

## Runtime execution authority

The sole authority for managed-agent execution will be an immutable object referred to here as `RuntimeExecutionPlan`. The final source name may differ, but the invariant is normative.

A plan identifies at least:

- plan identity and provider family;
- platform and architecture;
- component identities, roles, and per-component source;
- package and component versions;
- canonical absolute executable and resource paths;
- hashes and signing identities where supported;
- manifest provenance and compatibility generation;
- adapter, provider, login, and model-discovery invocations;
- sibling helper and resource locations;
- environment allowlist and plan-generated values;
- explicitly denied executable-control variables;
- update and rollback identity.

The same plan must be consumed by readiness, login, model discovery, managed spawn, turn execution, adapter child execution, sibling/helper execution, updates, and rollback. Discovery and installation may propose candidates; only plan resolution grants execution authority.

After approval, failure or drift must block execution and present a configuration state. Buzz must not silently substitute another command, PATH result, source, provider, or adapter.

## Provider families and component provenance

Buzz will model complete provider families rather than only primary executables. A family manifest may include an ACP adapter, provider CLI or app server, native sibling executables, utility binaries, package metadata, resources, and supported invocation roles.

Each component records its own provenance. A family may combine sources when its compatibility manifest permits it:

```text
Claude runtime family
├── Claude Code CLI: verified external
└── Claude ACP adapter: Buzz-managed
```

Verifying a primary executable while resolving helpers from PATH is insufficient.

## Runtime packages and storage

Provider-native binaries and helpers will be independently verifiable files. They must not be embedded in an adapter merely to be materialized elsewhere before use.

A bundled family may use a layout equivalent to:

```text
Buzz.app/Contents/Resources/Runtimes/
  <provider>/<package-id>/
    manifest
    adapter
    provider executable
    sibling helpers
    resources/
```

Buzz-managed packages use the same logical package and manifest shape in a Buzz-owned content-addressed store outside the application bundle. The adapter receives provider identity through a controlled plan interface; ambient or renderer-persisted environment must not redirect execution.

Managed runtime bytes should be stored in a profile-neutral content-addressed store. Profiles retain separate agent configuration, credentials, drafts, relay state, and plan approvals while sharing verified immutable objects.

A profile must not directly execute mutable files from another profile's legacy support directory. It may discover an existing package, verify or import it into the shared store, and approve a profile-specific plan referencing that object.

## Verified external installations

External installations remain mutable outside Buzz's control. Buzz will support two explicit policies where platform and licensing constraints permit.

### Verified in place

- Verify the complete family when approving the plan.
- Revalidate relevant artifacts before each process spawn.
- Refuse execution after drift until the new identity is explicitly approved.
- Never fall back to another discovered installation.

### Verified snapshot

- Discover and verify the external family.
- Import or safely link approved artifacts into the content-addressed store.
- Execute the immutable snapshot.
- Retain original external provenance in the plan.

Verified snapshot is the preferred deterministic default where practical; verified in place is the space-saving option.

## Legacy managed-runtime adoption

Existing Buzz installations may already contain managed adapters in the legacy private npm prefix. The new runtime manager will:

1. discover supported legacy packages;
2. distinguish installation from profile visibility;
3. identify package name, version, executable target, hashes, and provenance;
4. verify and adopt compatible packages into the new store;
5. create a profile-specific approved plan;
6. avoid presenting an existing compatible adapter as a required new installation.

Legacy directories are migration sources, not permanent ambient execution paths.

## Signing, updates, and rollback

Buzz-managed runtime updates will be independently releasable from desktop releases while retaining an explicit trust chain. The design must provide:

- a runtime metadata signing authority distinct from transport security;
- signed manifests with complete artifact hashes and compatibility constraints;
- content-addressed packages;
- freshness and rollback protection;
- staged download and full verification before activation;
- atomic active-plan switching;
- preservation of the previous known-good plan;
- post-activation health verification;
- rollback only to a previously approved plan;
- visible source, version, provenance, signing identity, and rollback state.

Desktop and package metadata express compatibility explicitly. An unsigned manifest delivered over HTTPS is insufficient.

Initial policy should stage compatible updates and request approval before activation. Automatic activation may be added later only with clear policy, health verification, and bounded rollback.

## Executable-control boundaries

Saved agent configuration may refer to a provider selection or approved plan ID. It must not regain authority through an arbitrary raw-command fallback. Custom commands, if supported, require a separate explicit trust flow producing a validated plan.

Spawn environments will be constructed from a narrow inherited allowlist, plan-generated values, and user configuration that cannot redirect executable identity. Executable-control variables such as provider path overrides must be stripped unless generated by the approved plan. Prefer typed adapter configuration over ambient executable selection.

Authority must remain intact across:

```text
agent selection
  -> readiness
  -> authentication probe
  -> model discovery
  -> harness
  -> ACP adapter
  -> provider CLI or app server
  -> sibling helpers
```

## Executable and behavioral determinism

This architecture can provide local executable determinism for adapter/provider code, sibling helpers, launch arguments, environment projection, resources, compatibility metadata, and update/rollback state.

It cannot guarantee remote-model behavioral determinism. Providers may change server-side models, aliases, routing, safety layers, account entitlements, prompts, and experiments independently of the local runtime.

Product language must distinguish:

- **Runtime verified:** local execution identity is known and approved.
- **Remote model identity:** the exact provider/model label reported at runtime.

A verified local runtime does not imply identical remote-model behavior.

## User experience

Runtime settings will show provider family, component-level source, version, verification state, provenance and signing identity where available, active plan, update owner/policy, drift or incompatibility, rollback, and explicit adoption/source-selection actions.

Each agent is pinned to a specific approved plan. Implicit fallback is prohibited. Explicit user-configured fallback among previously approved plans may be considered separately.

## Release cadence and trust boundaries

The architecture separates:

1. **Buzz desktop releases:** UI, execution-plan contract, installer framework, and compatibility policy.
2. **Buzz-managed runtime releases:** adapters, provider CLIs, siblings, and resources, signed and rollback-capable independently.
3. **External provider updates:** controlled externally and never adopted merely because PATH changed.

The bundled baseline should provide reliable first-run and recovery behavior without forcing every provider family into every installation. Exact baseline contents remain a platform, licensing, and product decision.

## Alternatives considered

### Monolithic bundled runtimes

Strongest out-of-box reproducibility and offline behavior, but rejected as the sole architecture because it increases application size, duplicates unused runtimes, couples provider updates to desktop releases, and encourages embedded/materialized duplication.

### Thin verified external runtimes

Small and able to reuse provider update channels, but rejected as the sole architecture because layouts vary, complete sibling families may be missing, external paths drift, and first-run/offline behavior is weaker.

### Hybrid

Accepted because it preserves a known-good baseline, supports independently updated deterministic packages, reuses verified external installations, and serves different product/user needs without weakening execution authority.

## Consequences

Positive consequences:

- runtime source becomes explicit and auditable;
- all execution stages share one authority;
- provider updates can be decoupled from desktop releases;
- verified external installations remain supported;
- runtime objects can be shared without sharing profile state;
- rollback becomes first-class;
- native duplication can be removed.

Costs and risks:

- manifest, signing, compatibility, and update infrastructure are required;
- external in-place verification has performance and race considerations;
- legacy-prefix migration must be carefully tested;
- provider-family compatibility becomes a maintained contract;
- UI must explain source and trust clearly.

## Functional candidate disposition and PR strategy

The accepted functional candidate is an integration reference and rollback artifact, not a suitable pull-request branch. Its worktrees combine existing PR implementations, later integration repairs, generally useful fixes, and private-candidate packaging.

Before implementation, create a change-disposition matrix classifying each coherent change as:

- represented by an existing PR;
- already upstream or superseded;
- new general-purpose PR;
- private build overlay;
- superseded by this architecture while retaining its tests/invariant;
- explicitly deferred.

Do not submit the composite candidate diff as one pull request. Reconstruct product work in clean sibling worktrees from a recorded upstream base.

Expected upstreamable sequence:

1. reconcile existing ingestion, mention-boundary, persistent-prompt, DNS/delivery, and codex-acp work;
2. extract small independent lifecycle and UI correctness repairs;
3. establish immutable runtime-plan authority;
4. remove raw-command and environment executable bypasses;
5. add provider-family manifests and complete-family verification;
6. add the content-addressed managed runtime store;
7. add bundled, managed, and verified-external resolvers;
8. add legacy managed-runtime adoption;
9. add signed update, activation, and rollback behavior;
10. add source, provenance, update, and rollback UX.

Repository-specific work remains separate. Generic Codex executable resolution and compiled-bundle behavior belong in `agentclientprotocol/codex-acp`; Buzz owns plan resolution, installation, profile approval, and UX.

A future candidate should be assembled as:

```text
recorded clean base
  + exact reviewed PR heads
  + new reviewed PR branches
  + minimal private packaging overlay
  = reproducible candidate
```

The private overlay may retain alternate bundle identity, candidate verification, isolated state, and rollback tooling until equivalent general support exists. Private names and machine-specific assumptions do not belong in upstream PRs.

The accepted candidate remains unchanged until a reconstructed candidate passes equivalent functional, restart, runtime-path, state-isolation, and rollback acceptance.

## Validation requirements

Implementation must test at least:

- bundled, managed, verified-in-place, and verified-snapshot sources;
- mixed-provenance provider families;
- complete sibling/helper verification;
- readiness, login, model discovery, spawn, and turns using one plan;
- missing, incompatible, changed, and replaced artifacts;
- no PATH or raw-command fallback after approval;
- denial of persisted executable-control overrides;
- legacy adapter discovery and adoption;
- update signature, compatibility, atomic activation, and rollback;
- restart with the same approved plan;
- profile isolation with shared immutable objects;
- unchanged response and delivery semantics;
- separation of execution success from response-delivery success.

## Deferred decisions

The following remain separate product decisions:

- exact bundled baseline per platform;
- automatic versus approval-gated update activation;
- whether ordered fallback among pre-approved plans is needed;
- runtime-object and rollback retention limits;
- signing-key hierarchy and metadata protocol details;
- provider licensing constraints on snapshots;
- whether private-profile support becomes an official feature.

These may refine implementation but do not alter the accepted hybrid architecture or immutable-plan authority.

## Related work

Relevant existing or historical work includes:

- `block/buzz#4913` — shared/external-agent mention support already upstream;
- `block/buzz#4964` — remote-owned agent ingestion, profile, Activity, and tray identity;
- `block/buzz#4983` — DNS/signed-delivery draft requiring decomposition and reconciliation;
- `block/buzz#5204` — packaged WKWebView mention-boundary behavior;
- `block/buzz#5386` — persistent ACP system-prompt negotiation;
- `agentclientprotocol/codex-acp#368` — DNS/workspace behavior;
- `agentclientprotocol/codex-acp#379` — persistent developer instructions.

Exact PR state and overlap must be re-verified against the selected implementation base before code is ported.
