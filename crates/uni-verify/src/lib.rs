use anyhow::{anyhow, Result};
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
    pub expect_not: String,
    /// globs of files whose content this evidence is bound to (content-addressed evidence)
    pub files: Vec<String>,
    pub timeout: u64,
}

const DEFAULT_TIMEOUT_SECS: u64 = 300;

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
                out.insert(k.clone(), VerifierSpec { run: s.to_string(), expect: String::new(), expect_not: String::new(), files: vec![], timeout: DEFAULT_TIMEOUT_SECS });
            } else if let Some(tbl) = v.as_table() {
                let run = tbl.get("run").and_then(|r| r.as_str()).unwrap_or("").to_string();
                let expect = tbl.get("expect").and_then(|e| e.as_str()).unwrap_or("").to_string();
                let expect_not = tbl.get("expect_not").and_then(|e| e.as_str()).unwrap_or("").to_string();
                let files = tbl
                    .get("files")
                    .and_then(|f| f.as_array())
                    .map(|a| {
                        a.iter()
                            .filter_map(|x| x.as_str().map(|s| s.to_string()))
                            .collect()
                    })
                    .unwrap_or_default();
                let timeout = tbl.get("timeout").and_then(|t| t.as_integer()).unwrap_or(DEFAULT_TIMEOUT_SECS as i64) as u64;
                if !run.is_empty() {
                    out.insert(k.clone(), VerifierSpec { run, expect, expect_not, files, timeout });
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
            return Ok(VerifierSpec { run: cmd.to_string(), expect: String::new(), expect_not: String::new(), files: vec![], timeout: DEFAULT_TIMEOUT_SECS }); // bootstrap
        }
        if registry.values().any(|v| v.run == cmd) {
            return Ok(VerifierSpec { run: cmd.to_string(), expect: String::new(), expect_not: String::new(), files: vec![], timeout: DEFAULT_TIMEOUT_SECS });
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

/// Content-addressed hash over the files a verifier watches (FR-010/FR-013).
/// Pattern semantics: matched against path RELATIVE to workspace; `*` within a
/// segment, `**` across segments; prefix/suffix matching otherwise.
pub fn artifact_hash(spec: &VerifierSpec, workspace: &std::path::Path) -> Option<String> {
    if spec.files.is_empty() {
        return None;
    }
    let matches_glob = |rel: &str, pat: &str| -> bool {
        // tokenize both sides; '**' eats anything
        let pat_parts: Vec<&str> = pat.split('/').collect();
        let rel_parts: Vec<&str> = rel.split('/').collect();
        fn m(p: &[&str], r: &[&str]) -> bool {
            if p.is_empty() {
                return r.is_empty();
            }
            match p[0] {
                "**" => (0..=r.len()).any(|i| m(&p[1..], &r[i..])),
                "*" => {
                    if r.is_empty() {
                        false
                    } else {
                        m(&p[1..], &r[1..])
                    }
                }
                seg => r.first() == Some(&seg) && m(&p[1..], &r[1..]),
            }
        }
        m(&pat_parts, &rel_parts)
    };
    let mut matched: Vec<(String, Option<String>)> = vec![];
    let skip = ["target", ".git", ".uni"];
    let mut stack = vec![workspace.to_path_buf()];
    while let Some(dir) = stack.pop() {
        if let Ok(rd) = std::fs::read_dir(&dir) {
            for e in rd.flatten() {
                let path = e.path();
                let rel = path
                    .strip_prefix(workspace)
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_default();
                if path.is_dir() {
                    if !skip.contains(&path.file_name().and_then(|n| n.to_str()).unwrap_or("")) {
                        stack.push(path.clone());
                    }
                } else if spec.files.iter().any(|g| matches_glob(&rel, g)) {
                    let content = std::fs::read(&path).ok().map(|b| sha256_hex(&b));
                    matched.push((rel, content));
                }
            }
        }
    }
    let acc: String = matched
        .iter()
        .map(|(rel, h)| format!("{rel}:{h:?};"))
        .collect();
    Some(sha256_hex(acc.as_bytes()))
}

/// Cache-isolating identity of a verifier spec (prevents evidence reuse
/// across contracts that merely share a claim id).
pub fn spec_fingerprint(verifier_ref: &str, spec: &VerifierSpec) -> String {
    let seed = format!(
        "{verifier_ref}|{}|{}|{}|{}",
        spec.run,
        spec.expect,
        spec.expect_not,
        spec.files.join(",")
    );
    sha256_hex(seed.as_bytes())[..12].to_string()
}

pub fn run_spec(
    claim_id: &str,
    verifier_ref: &str,
    spec: &VerifierSpec,
    workspace: &std::path::Path,
    timeout_secs: u64,
) -> Result<Evidence> {
    let (mut ev, full_output) = run_shell(claim_id, &spec.run, workspace, timeout_secs)?;
    ev.artifact_hash = artifact_hash(spec, workspace).unwrap_or_default();
    ev.fingerprint = spec_fingerprint(verifier_ref, spec);
    // expectations are checked against the FULL output, never the truncated excerpt
    if !spec.expect.is_empty() && !full_output.contains(&spec.expect) {
        ev.state = EvidenceState::Invalid;
    }
    if !spec.expect_not.is_empty() && full_output.contains(&spec.expect_not) {
        ev.state = EvidenceState::Invalid;
    }
    Ok(ev)
}

fn run_shell(
    claim_id: &str,
    command: &str,
    workspace: &std::path::Path,
    timeout_secs: u64,
) -> Result<(Evidence, String)> {
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
    let full_output = String::from_utf8_lossy(&combined).to_string();
    let excerpt: String = full_output.chars().take(2000).collect();
    let code = output.status.code().unwrap_or(-1);
    let ev = Evidence {
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
        artifact_hash: String::new(),
        fingerprint: String::new(),
    };
    Ok((ev, full_output))
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
        out.push(run_spec(&v.claim_id, &v.verifier_ref, &spec, workspace, spec.timeout)?);
    }
    Ok(out)
}
