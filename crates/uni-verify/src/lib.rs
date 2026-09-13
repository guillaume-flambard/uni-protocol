use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::time::Instant;
use uni_evidence::{git_info, sha256_hex, Evidence, EvidenceState};
use uni_ir::Ir;

/// Trusted verifier registry lives in `.uni/config.toml` (never inline untrusted commands).
/// Minimal format:
/// ```toml
/// [verifiers]
/// "project.check" = "cargo check"
/// "project.tests" = "cargo test"
/// ```
pub fn load_registry(dot_uni: &std::path::Path) -> HashMap<String, String> {
    let path = dot_uni.join("config.toml");
    let Ok(text) = std::fs::read_to_string(&path) else {
        return HashMap::new();
    };
    let Ok(val) = text.parse::<toml::Value>() else {
        return HashMap::new();
    };
    val.get("verifiers")
        .and_then(|v| v.as_table())
        .map(|t| {
            t.iter()
                .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                .collect()
        })
        .unwrap_or_default()
}

/// Resolve the shell command for a verification:
/// - inline `shell "..."` is allowed ONLY if it matches an allowlisted registry value
///   or if no registry exists yet (bootstrap T1) and workspace is the uni repo itself.
/// - otherwise the verifier_ref must exist in the registry.
pub fn resolve_command(
    verifier_ref: &str,
    inline_shell: Option<&str>,
    registry: &HashMap<String, String>,
) -> Result<String> {
    if verifier_ref == "shell" {
        let cmd = inline_shell.ok_or_else(|| anyhow!("shell verifier needs a command"))?;
        if registry.is_empty() {
            return Ok(cmd.to_string()); // bootstrap
        }
        if registry.values().any(|v| v == cmd) {
            return Ok(cmd.to_string());
        }
        return Err(anyhow!(
            "inline shell command not in trusted registry (.uni/config.toml [verifiers]): {cmd}"
        ));
    }
    // registry key, optional inline override must still match registry
    let trusted = registry
        .get(verifier_ref)
        .ok_or_else(|| anyhow!("unknown verifier '{verifier_ref}' (not in .uni/config.toml [verifiers])"))?;
    if let Some(inline) = inline_shell {
        if inline != trusted {
            return Err(anyhow!(
                "inline override for '{verifier_ref}' does not match trusted registry value"
            ));
        }
    }
    Ok(trusted.clone())
}

pub fn run_shell(
    claim_id: &str,
    command: &str,
    workspace: &std::path::Path,
    timeout_secs: u64,
) -> Result<Evidence> {
    let start = Instant::now();
    let (commit_sha, workspace_dirty) = git_info(workspace);
    // Minimal timeout: run via `timeout` when available, else direct.
    let output = if which_timeout() {
        std::process::Command::new("timeout")
            .arg(timeout_secs.to_string())
            .arg("sh")
            .arg("-c")
            .arg(command)
            .current_dir(workspace)
            .output()?
    } else {
        std::process::Command::new("sh")
            .arg("-c")
            .arg(command)
            .current_dir(workspace)
            .output()?
    };
    let combined = [output.stdout.clone(), output.stderr.clone()].concat();
    let excerpt: String = String::from_utf8_lossy(&combined).chars().take(2000).collect();
    let code = output.status.code().unwrap_or(-1);
    Ok(Evidence {
        id: format!("{claim_id}-{}", &sha256_hex(command.as_bytes())[..8]),
        claim_id: claim_id.to_string(),
        producer: "shell-verifier".into(),
        command: command.to_string(),
        exit_code: code,
        output_hash: sha256_hex(&combined),
        output_excerpt: excerpt,
        commit_sha,
        workspace_dirty,
        state: if code == 0 {
            EvidenceState::Valid
        } else {
            EvidenceState::Invalid
        },
        created_at: chrono::Utc::now(),
        duration_ms: start.elapsed().as_millis(),
    })
}

fn which_timeout() -> bool {
    std::process::Command::new("which")
        .arg("timeout")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// High seam: assure a full contract in one call (used by CLI + tests).
pub fn assure_contract(
    ir: &Ir,
    dot_uni: &std::path::Path,
    workspace: &std::path::Path,
) -> Result<Vec<Evidence>> {
    let registry = load_registry(dot_uni);
    let mut out = vec![];
    for v in &ir.verification {
        let cmd = resolve_command(&v.verifier_ref, v.inline_shell.as_deref(), &registry)?;
        out.push(run_shell(&v.claim_id, &cmd, workspace, 300)?);
    }
    Ok(out)
}
