use std::path::Path;

use super::{resolve_workspace_command_with_context, WorkspaceCommandResolution};

#[cfg(unix)]
fn write_executable(path: &Path) {
    use std::os::unix::fs::PermissionsExt;

    std::fs::create_dir_all(path.parent().expect("executable parent"))
        .expect("create executable parent");
    std::fs::write(path, "#!/bin/sh\nexit 0\n").expect("write executable");
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))
        .expect("chmod executable");
}

#[cfg(unix)]
#[test]
fn packaged_sidecar_wins_over_populated_development_targets() {
    let root = std::env::temp_dir().join(format!(
        "buzz-packaged-sidecar-resolution-{}",
        uuid::Uuid::new_v4()
    ));
    let current_dir = root.join("source-worktree");
    let current_exe = root.join("Buzz Test Release.app/Contents/MacOS/buzz-desktop");
    let bundled = root.join("Buzz Test Release.app/Contents/MacOS/buzz-acp");
    write_executable(&current_dir.join("target/debug/buzz-acp"));
    write_executable(&current_dir.join("target/release/buzz-acp"));
    write_executable(&root.join("target/debug/buzz-acp"));
    write_executable(&root.join("target/release/buzz-acp"));
    write_executable(&bundled);

    let resolved = resolve_workspace_command_with_context(
        "buzz-acp",
        &root,
        Some(&current_dir),
        Some(&current_exe),
    );

    assert_eq!(resolved, WorkspaceCommandResolution::Resolved(bundled));
    let _ = std::fs::remove_dir_all(root);
}

#[cfg(unix)]
#[test]
fn packaged_sidecar_missing_does_not_fall_back_to_development_targets() {
    let root = std::env::temp_dir().join(format!(
        "buzz-packaged-sidecar-missing-{}",
        uuid::Uuid::new_v4()
    ));
    let current_dir = root.join("source-worktree");
    let current_exe = root.join("Buzz Test Release.app/Contents/MacOS/buzz-desktop");
    write_executable(&current_dir.join("target/debug/buzz-acp"));
    write_executable(&root.join("target/release/buzz-acp"));

    let resolved = resolve_workspace_command_with_context(
        "buzz-acp",
        &root,
        Some(&current_dir),
        Some(&current_exe),
    );

    assert_eq!(resolved, WorkspaceCommandResolution::PackagedSidecarMissing);
    let _ = std::fs::remove_dir_all(root);
}

#[cfg(unix)]
#[test]
fn packaged_sidecar_ignores_explicit_external_sidecar_path() {
    let root = std::env::temp_dir().join(format!(
        "buzz-packaged-explicit-sidecar-{}",
        uuid::Uuid::new_v4()
    ));
    let current_exe = root.join("Buzz Test Release.app/Contents/MacOS/buzz-desktop");
    let bundled = root.join("Buzz Test Release.app/Contents/MacOS/buzz-acp");
    let external = root.join("source-worktree/target/debug/buzz-acp");
    write_executable(&bundled);
    write_executable(&external);

    let resolved = resolve_workspace_command_with_context(
        external.to_str().expect("utf8 external path"),
        &root,
        None,
        Some(&current_exe),
    );

    assert_eq!(resolved, WorkspaceCommandResolution::Resolved(bundled));
    let _ = std::fs::remove_dir_all(root);
}

#[cfg(unix)]
#[test]
fn packaged_execution_does_not_search_worktree_for_bare_custom_commands() {
    let root = std::env::temp_dir().join(format!(
        "buzz-packaged-custom-command-{}",
        uuid::Uuid::new_v4()
    ));
    let current_dir = root.join("source-worktree");
    let current_exe = root.join("Buzz Test Release.app/Contents/MacOS/buzz-desktop");
    write_executable(&current_dir.join("target/debug/custom-acp"));

    let resolved = resolve_workspace_command_with_context(
        "custom-acp",
        &root,
        Some(&current_dir),
        Some(&current_exe),
    );

    assert_eq!(resolved, WorkspaceCommandResolution::NotFound);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn packaged_sidecar_names_match_tauri_external_bins() {
    let config: serde_json::Value =
        serde_json::from_str(include_str!("../../../../tauri.conf.json"))
            .expect("parse tauri.conf.json");
    let mut configured = config["bundle"]["externalBin"]
        .as_array()
        .expect("bundle.externalBin array")
        .iter()
        .map(|entry| {
            super::super::normalize_command_identity(entry.as_str().expect("externalBin string"))
        })
        .collect::<Vec<_>>();
    let mut guarded = super::PACKAGED_SIDECAR_COMMANDS
        .iter()
        .map(|command| command.to_string())
        .collect::<Vec<_>>();
    configured.sort();
    guarded.sort();

    assert_eq!(guarded, configured);
}

#[cfg(unix)]
#[test]
fn development_execution_allows_release_sidecar_fallback() {
    let root = std::env::temp_dir().join(format!(
        "buzz-development-sidecar-resolution-{}",
        uuid::Uuid::new_v4()
    ));
    let current_dir = root.join("source-worktree");
    let current_exe = current_dir.join("target/debug/buzz-desktop");
    let release_sidecar = root.join("target/release/buzz-acp");
    write_executable(&release_sidecar);

    let resolved = resolve_workspace_command_with_context(
        "buzz-acp",
        &root,
        Some(&current_dir),
        Some(&current_exe),
    );

    assert_eq!(
        resolved,
        WorkspaceCommandResolution::Resolved(release_sidecar)
    );
    let _ = std::fs::remove_dir_all(root);
}
