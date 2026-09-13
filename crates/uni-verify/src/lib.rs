use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::time::Instant;
use uni_evidence::{git_info, sha256_hex, Evidence, EvidenceState};
use uni_ir::Ir;

/// Trusted verifier registry lives in `.uni/config.toml` (never inline untrusted commands).
/// Two forms:
/// ```toml
/// [verifiers]
/// "project.check" = "cargo check"                 # exit-code only
///
/// [verifiers."project.upper-test"]               # exit code + output expectation
/// run = "cargo test clamp_upper_works"
/// expect = "test result: ok. 1 passed"
/// ```
#[derive(Debug, Clone)]
pub struct VerifierSpec {
    pub run: String,
    pub expect: String,
}

pub fn load_registry(dot_uni: &std::path::Path) -> std::collections::HashMap<String, VerifierSpec> {
    let path = dot_uni.join("config.toml");
    let Ok(text) = std::fs::read_to_string(&path) else {
        return std::collections::HashMap::new();
    };
    let Ok(val) = text.parse::<toml::Value>() else {
        return std::collections::HashMap::new();
    };
    let mut out = std::collections::HashMap::new();
    if let Some(t) = val.get("verifiers").and_then(|v| v.as_table()) {
        for (k, v) in t {
            if let Some(s) = v.as_str() {
                out.insert(k.clone(), VerifierSpec { run: s.to_string(), expect: String::new() });
            } else if let Some(tbl) = v.as_table() {
                let run = tbl.get("run").and_then(|r| r.as_str()).unwrap_or("").to_string();
                let expect = tbl.get("expect").and_then(|e| e.as_str()).unwrap_or("").to_string();
                if !run.is_empty() {
                    out.insert(k.clone(), VerifierSpec { run, expect });
                }
            }
        }
    }
    out
}

/// Resolve the spec for a verification:
/// - inline `shell "..."` allowed only if an allowlisted registry value (bootstrap: empty registry + uni workspace).
/// - otherwise verifier_ref must exist in the registry.
pub fn resolve_command(
    verifier_ref: &str,
    inline_shell: Option<&str>,
    registry: &std::collections::HashMap<String, VerifierSpec>,
) -> Result<VerifierSpec> {
    if verifier_ref == "shell" {
        let cmd = inline_shell.ok_or_else(|| anyhow!("shell verifier needs a command"))?;
        if registry.is_empty() {
            return Ok(VerifierSpec { run: cmd.to_string(), expect: String::new() }); // bootstrap
        }
        if registry.values().any(|v| v.run == cmd) {
            return Ok(VerifierSpec { run: cmd.to_string(), expect: String::new() });
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
        if inline != trusted.run {
            return Err(anyhow!(
                "inline override for '{verifier_ref}' does not match trusted registry value"
            ));
        }
    }
    Ok(trusted.clone())
}

pub fn run_spec(
    claim_id: &str,
    spec: &VerifierSpec,
    workspace: &std::path::Path,
    timeout_secs: u64,
) -> Result<Evidence> {
    let mut ev = run_shell(claim_id, &spec.run, workspace, timeout_secs)?;
    if !spec.expect.is_empty() && !ev.output_excerpt.contains(&spec.expect) {
        ev.state = EvidenceState::Invalid;
    }
    Ok(ev)
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
        let spec = resolve_command(&v.verifier_ref, v.inline_shell.as_deref(), &registry)?;
        out.push(run_spec(&v.claim_id, &spec, workspace, 300)?);
    }
    Ok(out)
}
