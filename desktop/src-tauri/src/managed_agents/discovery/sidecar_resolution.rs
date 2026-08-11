use std::path::{Path, PathBuf};

use super::{
    command_looks_like_path, executable_basename, is_executable_file, normalize_command_identity,
    workspace_root_dir,
};

const PACKAGED_SIDECAR_COMMANDS: &[&str] = &[
    "buzz-acp",
    "buzz-agent",
    "buzz-backend-kubernetes",
    "buzz-dev-mcp",
    "git-credential-nostr",
    "buzz",
];

#[derive(Debug, Eq, PartialEq)]
pub(super) enum WorkspaceCommandResolution {
    Resolved(PathBuf),
    PackagedSidecarMissing,
    NotFound,
}

fn profile_target_dirs(root: &Path) -> [PathBuf; 2] {
    if cfg!(debug_assertions) {
        // `just dev` builds fresh debug sidecars; never prefer stale release output.
        [root.join("target/debug"), root.join("target/release")]
    } else {
        [root.join("target/release"), root.join("target/debug")]
    }
}

fn is_packaged_sidecar_command(command: &str) -> bool {
    let identity = normalize_command_identity(command);
    PACKAGED_SIDECAR_COMMANDS.contains(&identity.as_str())
}

/// Return the executable directory only when `current_exe` is structurally
/// inside a macOS application bundle. The existence of a source checkout is
/// deliberately irrelevant: packaged execution is identified by the running
/// executable, not by ambient build-machine paths.
fn macos_app_bundle_executable_dir(current_exe: &Path) -> Option<&Path> {
    let macos_dir = current_exe.parent()?;
    let contents_dir = macos_dir.parent()?;
    let app_dir = contents_dir.parent()?;
    (macos_dir.file_name().is_some_and(|name| name == "MacOS")
        && contents_dir
            .file_name()
            .is_some_and(|name| name == "Contents")
        && app_dir
            .extension()
            .is_some_and(|extension| extension == "app"))
    .then_some(macos_dir)
}

pub(super) fn resolve_packaged_sidecar_command(
    command: &str,
    current_exe: &Path,
) -> Option<WorkspaceCommandResolution> {
    let bundle_dir = macos_app_bundle_executable_dir(current_exe)?;
    if !is_packaged_sidecar_command(command) {
        return None;
    }

    let file_name = executable_basename(&normalize_command_identity(command));
    let candidate = bundle_dir.join(file_name);
    Some(if is_executable_file(&candidate) {
        WorkspaceCommandResolution::Resolved(candidate)
    } else {
        WorkspaceCommandResolution::PackagedSidecarMissing
    })
}

fn command_search_dirs_with_context(
    workspace_root: &Path,
    current_dir: Option<&Path>,
    current_exe: Option<&Path>,
) -> Vec<PathBuf> {
    let mut dirs = profile_target_dirs(workspace_root).to_vec();
    if let Some(current_dir) = current_dir {
        dirs.extend(profile_target_dirs(current_dir));
    }
    dirs.extend(current_exe.and_then(|path| path.parent().map(Path::to_path_buf)));
    dirs.into_iter().fold(Vec::new(), |mut unique, dir| {
        if !unique.contains(&dir) {
            unique.push(dir);
        }
        unique
    })
}

fn resolve_workspace_command_with_context(
    command: &str,
    workspace_root: &Path,
    current_dir: Option<&Path>,
    current_exe: Option<&Path>,
) -> WorkspaceCommandResolution {
    if let Some(resolution) =
        current_exe.and_then(|current_exe| resolve_packaged_sidecar_command(command, current_exe))
    {
        return resolution;
    }

    if current_exe
        .and_then(macos_app_bundle_executable_dir)
        .is_some()
    {
        // Packaged apps must not infer development execution from a source
        // worktree that happens to exist on the machine. Explicit custom paths
        // remain valid; bare non-sidecar commands continue to managed/PATH
        // resolution outside this workspace-specific resolver.
        if command_looks_like_path(command) {
            let path = PathBuf::from(command);
            return if is_executable_file(&path) {
                WorkspaceCommandResolution::Resolved(path)
            } else {
                WorkspaceCommandResolution::NotFound
            };
        }
        return WorkspaceCommandResolution::NotFound;
    }

    if command_looks_like_path(command) {
        let path = PathBuf::from(command);
        return if is_executable_file(&path) {
            WorkspaceCommandResolution::Resolved(path)
        } else {
            WorkspaceCommandResolution::NotFound
        };
    }

    let file_name = executable_basename(command);
    command_search_dirs_with_context(workspace_root, current_dir, current_exe)
        .into_iter()
        .map(|dir| dir.join(&file_name))
        .find(|candidate| is_executable_file(candidate))
        .map(WorkspaceCommandResolution::Resolved)
        .unwrap_or(WorkspaceCommandResolution::NotFound)
}

pub(super) fn resolve_workspace_command_resolution(command: &str) -> WorkspaceCommandResolution {
    let current_dir = std::env::current_dir().ok();
    let current_exe = std::env::current_exe().ok();
    resolve_workspace_command_with_context(
        command,
        &workspace_root_dir(),
        current_dir.as_deref(),
        current_exe.as_deref(),
    )
}

#[cfg(test)]
mod tests;
