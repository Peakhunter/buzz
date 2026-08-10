# ADR 0001 implementation slice: thin v6 Codex runtime plan

Status: Candidate for isolated verification

Source identities:

- Canonical source base: `block/buzz@119a84897f225c1e3213a09cd149abb37dcb3abc`
- Accepted ADR anchor: `Peakhunter/buzz@5be4a45a7c0c8a60cee3eb4273e2a425b4122b43`

## Smallest real vertical slice

This candidate applies ADR 0001's execution-authority boundary to one runtime family: Codex on non-Windows desktop platforms.

For each Codex operation, Buzz resolves one runtime execution plan containing:

- the absolute `codex-acp` adapter path;
- the complete npm adapter package tree;
- the exact Node interpreter;
- the absolute Codex provider path;
- either the complete npm provider package tree or a content-hashed native Mach-O/ELF provider;
- SHA-256 identities, byte counts, source classes, package-tree inventories, platform, architecture, and a deterministic plan ID;
- a plan-owned executable environment.

The same plan contract is consumed by adapter version detection, readiness/login probes, account connection, visible terminal login, model discovery, and managed-agent spawn. Components and package inventories are revalidated immediately before execution. Runtime selection fails closed on drift, undeclared package symlinks, unsupported launchers, or unresolved components.

The plan removes executable redirection, Node injection, dynamic-loader injection, and ambient `PATH` overrides. It supplies absolute adapter/provider paths and a minimal PATH containing the verified Node directory plus fixed operating-system directories.

The runtime catalog exposes the plan source (`managed`, `verified_external`, or `bundled`) and a short immutable plan ID for UI acceptance.

## Deliberate boundaries

- Codex and `codex-acp` are not copied into or bundled with `Buzz.app`.
- Existing verified runtimes are reused in place. Runtime installation remains an explicit user action.
- Other runtime families retain their existing behavior and are deferred to later slices.
- Windows Codex retains its existing behavior and is deferred.
- Native providers are treated as self-contained; operating-system libraries remain in the platform trust boundary.
- Same-user mutation after the immediate pre-execution verification is outside this slice's threat model.

## Isolated candidate

The CI candidate is an ad-hoc-signed Apple Silicon app named `Buzz v6 Candidate` with:

- bundle identifier `xyz.block.buzz.app.dev.thinv6`;
- keyring service `buzz-desktop-candidate.thin-v6`;
- nest directory `~/.buzz-candidate-thin-v6`;
- deep-link scheme `buzz-v6-candidate`;
- updater endpoints disabled.

This keeps the installed/running v5 app, its keyring, nest, deep links, source, caches, and rollback material untouched.

## Gates

The gates remain independent and ordered:

1. ADR anchor and human review.
2. Source review of this implementation slice.
3. Public free-runner build.
4. Independent artifact download and hash verification.
5. Isolated Mac UI/product acceptance.
6. Signing review.
7. Installation approval.
8. Release approval.

This candidate authorizes only gates 1-4. It does not authorize production signing, notarization, installation, GitHub Releases, or replacement of v5.
