use std::path::Path;

use crate::managed_agents::{
    discovery::{
        classify_runtime, codex_adapter_availability, codex_adapter_availability_with_plan,
        find_command, resolve_command, KnownAcpRuntime,
    },
    AcpAvailabilityStatus,
};

use super::{cli_probe, Requirement};

/// Requirements for CLI-login runtimes (claude, codex).
pub(super) fn requirements(
    probe_args: &[&str],
    setup_copy: &str,
    runtime: &KnownAcpRuntime,
) -> Vec<Requirement> {
    let adapter_result = runtime
        .commands
        .iter()
        .find_map(|cmd| find_command(cmd).map(|path| (*cmd, path)));
    let underlying_cli_found = runtime
        .underlying_cli
        .map(|cli| find_command(cli).is_some())
        .unwrap_or(false);

    let (mut availability, adapter_command, adapter_path) =
        classify_runtime(adapter_result, runtime.underlying_cli, underlying_cli_found);
    let runtime_plan = if runtime.id == "codex"
        && availability == AcpAvailabilityStatus::Available
        && !cfg!(windows)
    {
        let Some(adapter_command) = adapter_command.as_deref() else {
            return vec![invalid_plan_requirement(
                setup_copy,
                "Codex adapter command disappeared during readiness",
            )];
        };
        let plan = match crate::managed_agents::runtime_plan::resolve_runtime_execution_plan(
            adapter_command,
        ) {
            Ok(Some(plan)) => plan,
            Ok(None) => {
                return vec![invalid_plan_requirement(
                    setup_copy,
                    "Codex runtime plan is unavailable on this platform",
                )];
            }
            Err(error) => return vec![invalid_plan_requirement(setup_copy, &error)],
        };
        if let Err(error) = plan.verify() {
            return vec![invalid_plan_requirement(setup_copy, &error)];
        }
        availability = codex_adapter_availability_with_plan(&plan);
        Some(plan)
    } else {
        None
    };
    if runtime.id == "codex"
        && availability == AcpAvailabilityStatus::Available
        && cfg!(windows)
    {
        availability = adapter_path
            .as_deref()
            .map(|path| codex_adapter_availability(Path::new(path)))
            .unwrap_or(AcpAvailabilityStatus::AdapterOutdated);
    }

    match availability {
        AcpAvailabilityStatus::Available => {
            let binary_path = runtime_plan
                .as_ref()
                .and_then(|plan| plan.provider_cli_path().map(Path::to_path_buf))
                .or_else(|| resolve_command(probe_args[0]));
            let Some(binary_path) = binary_path else {
                return vec![missing_requirement(
                    probe_args,
                    setup_copy,
                    AcpAvailabilityStatus::Available,
                )];
            };
            let augmented_path = runtime_plan
                .as_ref()
                .and_then(|plan| plan.generated_environment("PATH").map(str::to_string))
                .or_else(cli_probe::augmented_path);
            let probe_outcome = if let Some(plan) = runtime_plan.as_ref() {
                cli_probe::login_probe_with_runtime_plan(&binary_path, probe_args, plan)
            } else {
                cli_probe::login_probe(&binary_path, probe_args, augmented_path.as_deref())
            };
            match probe_outcome {
                cli_probe::ProbeOutcome::LoggedIn => vec![],
                cli_probe::ProbeOutcome::LoggedOut => vec![missing_requirement(
                    probe_args,
                    setup_copy,
                    AcpAvailabilityStatus::Available,
                )],
                cli_probe::ProbeOutcome::ConfigInvalid { stderr_excerpt } => {
                    vec![Requirement::CliConfigInvalid {
                        probe_args: probe_args.iter().map(|value| value.to_string()).collect(),
                        setup_copy: setup_copy.to_string(),
                        diagnostic: stderr_excerpt,
                    }]
                }
            }
        }
        other => vec![missing_requirement(probe_args, setup_copy, other)],
    }
}

fn invalid_plan_requirement(setup_copy: &str, diagnostic: &str) -> Requirement {
    Requirement::CliConfigInvalid {
        probe_args: Vec::new(),
        setup_copy: setup_copy.to_string(),
        diagnostic: format!("runtime execution plan refused readiness: {diagnostic}"),
    }
}

fn missing_requirement(
    probe_args: &[&str],
    setup_copy: &str,
    availability: AcpAvailabilityStatus,
) -> Requirement {
    Requirement::CliLogin {
        probe_args: probe_args.iter().map(|value| value.to_string()).collect(),
        setup_copy: setup_copy.to_string(),
        availability,
    }
}
