//! Immutable execution plans for the first managed-agent runtime family.
//!
//! This is the first thin slice of ADR 0001. It resolves the existing runtime
//! catalog into content-identified absolute component paths, denies ambient
//! executable-selection overrides, and revalidates every component before a
//! child process is spawned. Managed packages, snapshots, persistence, signing,
//! update activation, and rollback are deliberately deferred.

use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs::File,
    io::{BufRead, BufReader, Read},
    path::{Path, PathBuf},
    process::Command,
};

use super::{known_acp_runtime, resolve_command};

/// Environment variables that may redirect a known provider executable.
pub(crate) const DENIED_EXECUTABLE_ENV: &[&str] = &[
    "CLAUDE_CODE_EXECUTABLE",
    "CODEX_PATH",
    "DYLD_FALLBACK_FRAMEWORK_PATH",
    "DYLD_FALLBACK_LIBRARY_PATH",
    "DYLD_FORCE_FLAT_NAMESPACE",
    "DYLD_FRAMEWORK_PATH",
    "DYLD_IMAGE_SUFFIX",
    "DYLD_INSERT_LIBRARIES",
    "DYLD_LIBRARY_PATH",
    "DYLD_ROOT_PATH",
    "DYLD_VERSIONED_FRAMEWORK_PATH",
    "DYLD_VERSIONED_LIBRARY_PATH",
    "LD_AUDIT",
    "LD_DEBUG",
    "LD_DEBUG_OUTPUT",
    "LD_LIBRARY_PATH",
    "LD_PRELOAD",
    "LD_PROFILE",
    "NODE_OPTIONS",
    "NODE_PATH",
    "PATH",
];

/// Runtime bytes are shipped with Buzz, held in Buzz's managed prefix, or
/// explicitly reused in place.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RuntimePlanSource {
    Bundled,
    Managed,
    VerifiedExternal,
}

/// Role played by one content-identified runtime component.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RuntimeComponentRole {
    Harness,
    ProviderCli,
    Interpreter,
    RuntimeDependency,
}

/// One immutable component in a runtime execution plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RuntimePlanComponent {
    pub role: RuntimeComponentRole,
    pub source: RuntimePlanSource,
    pub path: PathBuf,
    pub sha256: String,
    pub bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RuntimePackageInventory {
    pub root: PathBuf,
    pub tree_sha256: String,
    pub files: usize,
}

/// The sole executable identity consumed by a known runtime operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RuntimeExecutionPlan {
    pub id: String,
    pub provider_family: String,
    pub platform: String,
    pub architecture: String,
    pub source: RuntimePlanSource,
    pub components: Vec<RuntimePlanComponent>,
    pub packages: Vec<RuntimePackageInventory>,
    pub generated_env: BTreeMap<String, String>,
    pub denied_env: Vec<String>,
}

impl RuntimeExecutionPlan {
    /// Stable wire label for the plan's selected source.
    pub(crate) fn source_label(&self) -> &'static str {
        match self.source {
            RuntimePlanSource::Bundled => "bundled",
            RuntimePlanSource::Managed => "managed",
            RuntimePlanSource::VerifiedExternal => "verified_external",
        }
    }

    /// Return the planned harness/adapter executable.
    pub(crate) fn harness_path(&self) -> Result<&Path, String> {
        self.components
            .iter()
            .find(|component| component.role == RuntimeComponentRole::Harness)
            .map(|component| component.path.as_path())
            .ok_or_else(|| format!("runtime plan {} has no harness component", self.id))
    }

    /// Return the planned provider CLI, when the family has a separate one.
    pub(crate) fn provider_cli_path(&self) -> Option<&Path> {
        self.components
            .iter()
            .find(|component| component.role == RuntimeComponentRole::ProviderCli)
            .map(|component| component.path.as_path())
    }

    pub(crate) fn generated_environment(&self, key: &str) -> Option<&str> {
        self.generated_env.get(key).map(String::as_str)
    }

    pub(crate) fn generated_environment_entries(&self) -> impl Iterator<Item = (&str, &str)> {
        self.generated_env
            .iter()
            .map(|(key, value)| (key.as_str(), value.as_str()))
    }

    pub(crate) fn denied_environment(&self) -> impl Iterator<Item = &str> {
        self.denied_env.iter().map(String::as_str)
    }

    /// Re-hash every component immediately before execution and refuse drift.
    pub(crate) fn verify(&self) -> Result<(), String> {
        for component in &self.components {
            let current = component_identity(component.role, component.source, &component.path)?;
            if current.sha256 != component.sha256 || current.bytes != component.bytes {
                return Err(format!(
                    "runtime plan {} drifted: {} no longer matches approved SHA-256 {}",
                    self.id,
                    component.path.display(),
                    component.sha256
                ));
            }
        }
        for package in &self.packages {
            let current = package_inventory(&package.root)?;
            if current.tree_sha256 != package.tree_sha256 || current.files != package.files {
                return Err(format!(
                    "runtime plan {} drifted: package tree {} no longer matches SHA-256 {}",
                    self.id,
                    package.root.display(),
                    package.tree_sha256
                ));
            }
        }
        Ok(())
    }

    /// Remove every executable override and then project plan-owned values.
    pub(crate) fn apply_environment(&self, command: &mut Command) {
        for key in &self.denied_env {
            command.env_remove(key);
        }
        for (key, value) in &self.generated_env {
            command.env(key, value);
        }
    }
}

/// Resolve Codex into an immutable plan. Other known runtimes, plus custom and
/// preset harnesses, remain on the legacy path until they gain complete plans.
pub(crate) fn resolve_runtime_execution_plan(
    effective_command: &str,
) -> Result<Option<RuntimeExecutionPlan>, String> {
    let Some(runtime) = known_acp_runtime(effective_command) else {
        return Ok(None);
    };
    // The first candidate proves the plan boundary for one external family.
    // Other built-ins stay on their existing path until their complete runtime
    // dependency closure can be represented without pretending it is verified.
    if runtime.id != "codex" || cfg!(windows) {
        return Ok(None);
    }

    let harness_path = resolve_command(effective_command).ok_or_else(|| {
        format!(
            "cannot resolve {} runtime harness `{effective_command}`",
            runtime.label
        )
    })?;
    let source = component_source(&harness_path, runtime.id == "buzz-agent");
    let mut generated_env = BTreeMap::new();
    let mut packages = Vec::new();
    let harness = component_identity(RuntimeComponentRole::Harness, source, &harness_path)?;
    let mut components = vec![harness];
    append_node_runtime_closure(
        &harness_path,
        source,
        &mut components,
        &mut packages,
        &mut generated_env,
    )?;

    if let Some(provider_command) = runtime.underlying_cli {
        let provider_path = resolve_command(provider_command).ok_or_else(|| {
            format!(
                "cannot resolve {} provider CLI `{provider_command}`",
                runtime.label
            )
        })?;
        let provider = component_identity(
            RuntimeComponentRole::ProviderCli,
            RuntimePlanSource::VerifiedExternal,
            &provider_path,
        )?;
        if provider.path != components[0].path {
            components.push(provider.clone());
            append_provider_runtime_closure(
                &provider.path,
                RuntimePlanSource::VerifiedExternal,
                &mut components,
                &mut packages,
                &mut generated_env,
            )?;
            match runtime.id {
                "claude" => {
                    generated_env.insert(
                        "CLAUDE_CODE_EXECUTABLE".to_string(),
                        provider.path.display().to_string(),
                    );
                }
                "codex" => {
                    generated_env.insert(
                        "CODEX_PATH".to_string(),
                        provider.path.display().to_string(),
                    );
                }
                _ => {}
            }
        }
    }

    let id = plan_identity(runtime.id, source, &components, &packages, &generated_env);
    Ok(Some(RuntimeExecutionPlan {
        id,
        provider_family: runtime.id.to_string(),
        platform: std::env::consts::OS.to_string(),
        architecture: std::env::consts::ARCH.to_string(),
        source,
        components,
        packages,
        generated_env,
        denied_env: DENIED_EXECUTABLE_ENV
            .iter()
            .map(|key| (*key).to_string())
            .collect(),
    }))
}

fn append_provider_runtime_closure(
    provider: &Path,
    source: RuntimePlanSource,
    components: &mut Vec<RuntimePlanComponent>,
    packages: &mut Vec<RuntimePackageInventory>,
    generated_env: &mut BTreeMap<String, String>,
) -> Result<(), String> {
    let mut file = File::open(provider)
        .map_err(|error| format!("failed to inspect {}: {error}", provider.display()))?;
    let mut prefix = [0_u8; 4];
    file.read_exact(&mut prefix)
        .map_err(|error| format!("failed to inspect {}: {error}", provider.display()))?;
    if prefix.starts_with(b"#!") {
        return append_node_runtime_closure(provider, source, components, packages, generated_env);
    }
    const NATIVE_MAGICS: [[u8; 4]; 9] = [
        *b"\x7fELF",
        [0xfe, 0xed, 0xfa, 0xce],
        [0xfe, 0xed, 0xfa, 0xcf],
        [0xce, 0xfa, 0xed, 0xfe],
        [0xcf, 0xfa, 0xed, 0xfe],
        [0xca, 0xfe, 0xba, 0xbe],
        [0xbe, 0xba, 0xfe, 0xca],
        [0xca, 0xfe, 0xba, 0xbf],
        [0xbf, 0xba, 0xfe, 0xca],
    ];
    if NATIVE_MAGICS.contains(&prefix) {
        // The provider executable itself is already a plan component. Dynamic
        // loader injection variables are removed by apply_environment(); OS
        // system libraries remain part of the platform trust boundary.
        return Ok(());
    }
    Err(format!(
        "Codex provider {} is neither a Node package launcher nor a supported native executable",
        provider.display()
    ))
}

fn append_node_runtime_closure(
    launcher: &Path,
    source: RuntimePlanSource,
    components: &mut Vec<RuntimePlanComponent>,
    packages: &mut Vec<RuntimePackageInventory>,
    generated_env: &mut BTreeMap<String, String>,
) -> Result<(), String> {
    let file = File::open(launcher).map_err(|error| {
        format!(
            "failed to inspect Codex adapter launcher {}: {error}",
            launcher.display()
        )
    })?;
    let mut first_line = String::new();
    BufReader::new(file)
        .read_line(&mut first_line)
        .map_err(|error| format!("failed to read Codex adapter shebang: {error}"))?;
    let shebang = first_line
        .strip_prefix("#!")
        .ok_or_else(|| {
            "Codex adapter is not a shebang launcher; refusing an incomplete plan".to_string()
        })?
        .trim();
    let words: Vec<&str> = shebang.split_whitespace().collect();
    let (interpreter_launcher, interpreter_name) = match words.as_slice() {
        [env, name, ..] if *env == "/usr/bin/env" => (Some(PathBuf::from(env)), *name),
        [interpreter, ..] => (None, *interpreter),
        [] => return Err("Codex adapter has an empty shebang".to_string()),
    };
    if Path::new(interpreter_name)
        .file_name()
        .and_then(|name| name.to_str())
        != Some("node")
    {
        return Err(format!(
            "Codex adapter interpreter `{interpreter_name}` is not the supported Node runtime"
        ));
    }

    if let Some(env_path) = interpreter_launcher {
        push_unique_component(
            components,
            component_identity(
                RuntimeComponentRole::Interpreter,
                RuntimePlanSource::VerifiedExternal,
                &env_path,
            )?,
        );
    }
    let node_path = resolve_command(interpreter_name)
        .ok_or_else(|| "cannot resolve the Node interpreter for Codex adapter".to_string())?;
    push_unique_component(
        components,
        component_identity(
            RuntimeComponentRole::Interpreter,
            component_source(&node_path, false),
            &node_path,
        )?,
    );
    generated_env.insert("PATH".to_string(), planned_path(&node_path)?);

    let canonical_launcher = launcher.canonicalize().map_err(|error| {
        format!(
            "failed to canonicalize Codex adapter launcher {}: {error}",
            launcher.display()
        )
    })?;
    let package_root = canonical_launcher
        .parent()
        .into_iter()
        .flat_map(Path::ancestors)
        .take(10)
        .find(|directory| directory.join("package.json").is_file())
        .ok_or_else(|| {
            format!(
                "cannot identify the npm package containing Codex adapter {}",
                canonical_launcher.display()
            )
        })?;
    let mut package_files = Vec::new();
    collect_package_files(package_root, &mut package_files, 20_000)?;
    package_files.sort();
    for path in &package_files {
        push_unique_component(
            components,
            component_identity(RuntimeComponentRole::RuntimeDependency, source, path)?,
        );
    }
    let inventory = package_inventory_from_files(package_root, &package_files)?;
    if !packages
        .iter()
        .any(|package| package.root == inventory.root)
    {
        packages.push(inventory);
    }
    Ok(())
}

fn collect_package_files(
    directory: &Path,
    files: &mut Vec<PathBuf>,
    limit: usize,
) -> Result<(), String> {
    // npm's `.bin` directory is only a set of alternate launcher symlinks;
    // the selected canonical launcher and package payload are inventoried
    // separately, so following or accepting those mutable aliases is unsafe.
    if directory.file_name().and_then(|name| name.to_str()) == Some(".bin") {
        return Ok(());
    }
    let entries = std::fs::read_dir(directory).map_err(|error| {
        format!(
            "failed to read runtime package {}: {error}",
            directory.display()
        )
    })?;
    for entry in entries {
        let entry = entry.map_err(|error| {
            format!(
                "failed to enumerate runtime package {}: {error}",
                directory.display()
            )
        })?;
        let file_type = entry
            .file_type()
            .map_err(|error| format!("failed to inspect {}: {error}", entry.path().display()))?;
        if file_type.is_symlink() {
            return Err(format!(
                "runtime package contains unsupported symlink: {}",
                entry.path().display()
            ));
        } else if file_type.is_dir() {
            collect_package_files(&entry.path(), files, limit)?;
        } else if file_type.is_file() {
            files.push(entry.path());
        }
        if files.len() > limit {
            return Err(format!(
                "runtime package exceeds the {limit}-file verification limit"
            ));
        }
    }
    Ok(())
}

fn package_inventory(root: &Path) -> Result<RuntimePackageInventory, String> {
    let mut files = Vec::new();
    collect_package_files(root, &mut files, 20_000)?;
    files.sort();
    package_inventory_from_files(root, &files)
}

fn package_inventory_from_files(
    root: &Path,
    files: &[PathBuf],
) -> Result<RuntimePackageInventory, String> {
    let canonical_root = root.canonicalize().map_err(|error| {
        format!(
            "failed to canonicalize runtime package {}: {error}",
            root.display()
        )
    })?;
    let mut hasher = Sha256::new();
    for path in files {
        let canonical = path.canonicalize().map_err(|error| {
            format!(
                "failed to canonicalize runtime package file {}: {error}",
                path.display()
            )
        })?;
        let relative = canonical.strip_prefix(&canonical_root).map_err(|_| {
            format!(
                "runtime package file escaped its root: {}",
                canonical.display()
            )
        })?;
        let identity = component_identity(
            RuntimeComponentRole::RuntimeDependency,
            RuntimePlanSource::VerifiedExternal,
            &canonical,
        )?;
        let relative = relative.to_string_lossy();
        hasher.update(relative.len().to_le_bytes());
        hasher.update(relative.as_bytes());
        hasher.update(identity.bytes.to_le_bytes());
        hasher.update(identity.sha256.as_bytes());
    }
    Ok(RuntimePackageInventory {
        root: canonical_root,
        tree_sha256: hex::encode(hasher.finalize()),
        files: files.len(),
    })
}

fn planned_path(node_path: &Path) -> Result<String, String> {
    let node_dir = node_path.parent().ok_or_else(|| {
        format!(
            "Node runtime has no parent directory: {}",
            node_path.display()
        )
    })?;
    let mut paths = vec![node_dir.to_path_buf()];
    for system_path in ["/usr/bin", "/bin", "/usr/sbin", "/sbin"] {
        let path = PathBuf::from(system_path);
        if path.is_dir() && !paths.contains(&path) {
            paths.push(path);
        }
    }
    std::env::join_paths(paths)
        .map(|path| path.to_string_lossy().into_owned())
        .map_err(|error| format!("failed to construct plan-owned PATH: {error}"))
}

fn push_unique_component(
    components: &mut Vec<RuntimePlanComponent>,
    component: RuntimePlanComponent,
) {
    if !components
        .iter()
        .any(|existing| existing.path == component.path)
    {
        components.push(component);
    }
}

fn component_source(path: &Path, bundled: bool) -> RuntimePlanSource {
    if bundled {
        return RuntimePlanSource::Bundled;
    }
    let managed_prefix =
        super::buzz_managed_npm_prefix().and_then(|prefix| prefix.canonicalize().ok());
    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    if managed_prefix.is_some_and(|prefix| canonical.starts_with(prefix)) {
        RuntimePlanSource::Managed
    } else {
        RuntimePlanSource::VerifiedExternal
    }
}

fn component_identity(
    role: RuntimeComponentRole,
    source: RuntimePlanSource,
    path: &Path,
) -> Result<RuntimePlanComponent, String> {
    let canonical = path.canonicalize().map_err(|error| {
        format!(
            "failed to canonicalize runtime component {}: {error}",
            path.display()
        )
    })?;
    let metadata = canonical.metadata().map_err(|error| {
        format!(
            "failed to inspect runtime component {}: {error}",
            canonical.display()
        )
    })?;
    if !metadata.is_file() {
        return Err(format!(
            "runtime component is not a file: {}",
            canonical.display()
        ));
    }
    let sha256 = sha256_file(&canonical)?;
    Ok(RuntimePlanComponent {
        role,
        source,
        path: canonical,
        sha256,
        bytes: metadata.len(),
    })
}

fn sha256_file(path: &Path) -> Result<String, String> {
    let mut file = File::open(path).map_err(|error| {
        format!(
            "failed to open runtime component {}: {error}",
            path.display()
        )
    })?;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer).map_err(|error| {
            format!(
                "failed to hash runtime component {}: {error}",
                path.display()
            )
        })?;
        if read == 0 {
            break;
        }
        digest.update(&buffer[..read]);
    }
    Ok(hex::encode(digest.finalize()))
}

fn plan_identity(
    provider_family: &str,
    source: RuntimePlanSource,
    components: &[RuntimePlanComponent],
    packages: &[RuntimePackageInventory],
    generated_env: &BTreeMap<String, String>,
) -> String {
    let mut digest = Sha256::new();
    digest.update(b"buzz-runtime-plan-v2\0");
    digest.update(provider_family.as_bytes());
    digest.update([0]);
    digest.update(format!("{source:?}").as_bytes());
    digest.update([0]);
    digest.update(std::env::consts::OS.as_bytes());
    digest.update([0]);
    digest.update(std::env::consts::ARCH.as_bytes());
    for component in components {
        digest.update([0]);
        digest.update(format!("{:?}", component.role).as_bytes());
        digest.update([0]);
        digest.update(component.path.to_string_lossy().as_bytes());
        digest.update([0]);
        digest.update(component.sha256.as_bytes());
        digest.update(component.bytes.to_le_bytes());
    }
    for package in packages {
        digest.update([0]);
        digest.update(package.root.to_string_lossy().as_bytes());
        digest.update([0]);
        digest.update(package.tree_sha256.as_bytes());
        digest.update(package.files.to_le_bytes());
    }
    for (key, value) in generated_env {
        digest.update([0]);
        digest.update(key.as_bytes());
        digest.update([0]);
        digest.update(value.as_bytes());
    }
    hex::encode(digest.finalize())
}

#[cfg(test)]
mod tests {
    use super::{
        collect_package_files, component_identity, package_inventory, plan_identity,
        RuntimeComponentRole, RuntimeExecutionPlan, RuntimePlanSource, DENIED_EXECUTABLE_ENV,
    };
    use std::{collections::BTreeMap, fs};

    #[test]
    fn plan_identity_changes_with_component_bytes() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("adapter");
        fs::write(&path, b"first").expect("write component");
        let first = component_identity(
            RuntimeComponentRole::Harness,
            RuntimePlanSource::VerifiedExternal,
            &path,
        )
        .expect("first identity");
        fs::write(&path, b"second").expect("replace component");
        let second = component_identity(
            RuntimeComponentRole::Harness,
            RuntimePlanSource::VerifiedExternal,
            &path,
        )
        .expect("second identity");
        assert_ne!(
            plan_identity(
                "codex",
                RuntimePlanSource::VerifiedExternal,
                &[first],
                &[],
                &BTreeMap::new()
            ),
            plan_identity(
                "codex",
                RuntimePlanSource::VerifiedExternal,
                &[second],
                &[],
                &BTreeMap::new()
            )
        );
    }

    #[test]
    fn verification_fails_closed_after_drift() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("adapter");
        fs::write(&path, b"approved").expect("write component");
        let component = component_identity(
            RuntimeComponentRole::Harness,
            RuntimePlanSource::VerifiedExternal,
            &path,
        )
        .expect("component identity");
        let plan = RuntimeExecutionPlan {
            id: "test-plan".to_string(),
            provider_family: "codex".to_string(),
            platform: std::env::consts::OS.to_string(),
            architecture: std::env::consts::ARCH.to_string(),
            source: RuntimePlanSource::VerifiedExternal,
            components: vec![component],
            packages: vec![],
            generated_env: BTreeMap::new(),
            denied_env: DENIED_EXECUTABLE_ENV
                .iter()
                .map(|key| (*key).to_string())
                .collect(),
        };
        plan.verify().expect("approved bytes verify");
        fs::write(&path, b"drifted").expect("replace component");
        assert!(plan.verify().is_err());
    }

    #[test]
    fn package_inventory_includes_payload_and_skips_launcher_aliases() {
        let dir = tempfile::tempdir().expect("temp dir");
        let nested = dir.path().join("dist");
        let aliases = dir.path().join("node_modules/.bin");
        fs::create_dir_all(&nested).expect("create payload directory");
        fs::create_dir_all(&aliases).expect("create alias directory");
        let payload = nested.join("index.js");
        let alias = aliases.join("codex-acp");
        fs::write(&payload, b"export {};").expect("write payload");
        fs::write(&alias, b"ignored launcher alias").expect("write alias");

        let mut files = Vec::new();
        collect_package_files(dir.path(), &mut files, 20).expect("collect package");
        assert!(files.contains(&payload));
        assert!(!files.contains(&alias));

        let inventory = package_inventory(dir.path()).expect("inventory package");
        let plan = RuntimeExecutionPlan {
            id: "package-plan".to_string(),
            provider_family: "codex".to_string(),
            platform: std::env::consts::OS.to_string(),
            architecture: std::env::consts::ARCH.to_string(),
            source: RuntimePlanSource::VerifiedExternal,
            components: vec![],
            packages: vec![inventory],
            generated_env: BTreeMap::new(),
            denied_env: vec![],
        };
        plan.verify().expect("package tree verifies");
        fs::write(dir.path().join("added.js"), b"unexpected").expect("add package file");
        assert!(plan.verify().is_err());
    }
}
