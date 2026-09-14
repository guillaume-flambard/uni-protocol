use std::path::PathBuf;
use std::process::Command;

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_uni")
}

fn mk_repo(tag: &str) -> PathBuf {
    use std::time::{SystemTime, UNIX_EPOCH};
    let dir = std::env::temp_dir().join(format!(
        "uni-bind-{tag}-{}",
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis()
    ));
    std::fs::create_dir_all(dir.join(".uni/evidence")).unwrap();
    dir
}

fn git(cwd: &PathBuf, args: &[&str]) {
    assert!(Command::new("git").args(args).current_dir(cwd).status().unwrap().success());
}

fn run(args: &[&str], dir: &PathBuf) -> std::process::Output {
    Command::new(bin()).args(args).current_dir(dir).output().unwrap()
}

/// A selector-template verifier: the contract states the requirement, the
/// worker names the test, the human authorizes which test counts.
fn setup_template(dir: &PathBuf) {
    std::fs::write(
        dir.join(".uni/config.toml"),
        "[verifiers]\n\"suite\" = {\"run\" = \"cargo test {{selector}} -- --exact\", \"expect\" = \"test result: ok. 1 passed\"}\n",
    )
    .unwrap();
    std::fs::write(dir.join("Cargo.toml"), "[package]\nname = \"tpl\"\nversion = \"0.1.0\"\nedition = \"2021\"\n").unwrap();
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::write(dir.join("src/lib.rs"), "pub fn alpha() -> u32 { 1 }\n").unwrap();
    std::fs::create_dir_all(dir.join("tests")).unwrap();
    std::fs::write(
        dir.join("tests/it.rs"),
        "use tpl::alpha;\n\n#[test]\nfn alpha_ok() {\n    assert_eq!(alpha(), 1);\n}\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("c.uni"),
        "VERSION 0.1\nDOMAIN software\nINTENT bind-demo\nGOAL\n g\nCLAIM alpha REQUIRED\n  ENSURE alpha returns 1\nVERIFY alpha\n  USING suite\n  REQUIRE behavior(\"alpha returns 1\")\nACCEPT WHEN\n  required_claims == VERIFIED\n  AND critical_failures == 0\n",
    )
    .unwrap();
    git(dir, &["init", "-q"]);
    git(dir, &["add", "."]);
    git(dir, &["-c", "user.email=t@t", "-c", "user.name=t", "commit", "-q", "-m", "base"]);
}

/// v0.4: a template verifier without an authorized selector never executes.
#[test]
fn template_requires_authorized_selector() {
    let dir = mk_repo("tpl-noselector");
    setup_template(&dir);
    let o = run(&["verify", "c.uni"], &dir);
    assert_ne!(o.status.code(), Some(0));
    let err = String::from_utf8_lossy(&o.stderr).to_string();
    assert!(err.contains("--selector"), "{err}");

    // Lint says the same thing before anything runs.
    let l = run(&["lint", "c.uni"], &dir);
    let out = String::from_utf8_lossy(&l.stdout).to_string();
    assert!(out.contains("selector-template"), "{out}");
}

/// v0.4: authorizing a selector makes it run; re-authorizing to another
/// selector invalidates the previous evidence (the selector is part of the
/// authorization, not a free parameter).
#[test]
fn selector_binding_authorizes_and_invalidates() {
    let dir = mk_repo("tpl-bind");
    setup_template(&dir);

    let b = run(
        &[
            "bind",
            "--claim",
            "alpha",
            "--verifier",
            "suite",
            "--requirement",
            "behavior(\"alpha returns 1\")",
            "--selector",
            "alpha_ok",
        ],
        &dir,
    );
    assert_eq!(b.status.code(), Some(0), "{}", String::from_utf8_lossy(&b.stderr));
    let first = run(&["verify", "c.uni"], &dir);
    assert_eq!(first.status.code(), Some(0), "{}", String::from_utf8_lossy(&first.stderr));
    let stdout = String::from_utf8_lossy(&first.stdout).to_string();
    assert!(stdout.contains("Accepted"), "{stdout}");
    // The evidence must record the resolved command, not the template.
    let j = run(&["--json", "verify", "c.uni"], &dir);
    let v: serde_json::Value = serde_json::from_slice(&j.stdout).unwrap();
    let cmd = v["evidence"][0]["command"].as_str().unwrap_or("");
    assert!(cmd.contains("alpha_ok") && !cmd.contains("{{selector}}"), "resolved command: {cmd}");

    // Re-authorize to a selector that selects no test: the old proof must not
    // silently satisfy the new authorization.
    let b2 = run(
        &[
            "bind",
            "--claim",
            "alpha",
            "--verifier",
            "suite",
            "--requirement",
            "behavior(\"alpha returns 1\")",
            "--selector",
            "does_not_exist",
        ],
        &dir,
    );
    assert_eq!(b2.status.code(), Some(0));
    let second = run(&["verify", "c.uni"], &dir);
    assert_ne!(second.status.code(), Some(0), "a new selector must not reuse the old evidence");
    let journal = std::fs::read_to_string(dir.join(".uni/events.jsonl")).unwrap();
    assert!(journal.contains("BindingAuthorized"), "bind must be journaled");
    assert!(journal.contains("does_not_exist"), "the authorizing act must name the selector");
}

/// v0.4: non-template verifiers keep working with no binding at all.
#[test]
fn plain_verifier_needs_no_binding() {
    let dir = mk_repo("plain");
    std::fs::write(dir.join(".uni/config.toml"), "[verifiers]\n\"ok\" = \"true\"\n").unwrap();
    std::fs::write(
        dir.join("c.uni"),
        "VERSION 0.1\nDOMAIN software\nINTENT plain\nGOAL\n g\nCLAIM x REQUIRED\n  ENSURE g\nVERIFY x\n  USING ok\nACCEPT WHEN\n  required_claims == VERIFIED\n  AND critical_failures == 0\n",
    )
    .unwrap();
    git(&dir, &["init", "-q"]);
    git(&dir, &["add", "."]);
    git(&dir, &["-c", "user.email=t@t", "-c", "user.name=t", "commit", "-q", "-m", "a"]);
    let o = run(&["verify", "c.uni"], &dir);
    assert_eq!(o.status.code(), Some(0), "{}", String::from_utf8_lossy(&o.stderr));
}

/// v0.7: authorize a reviewed file in one act. Fifty claims no longer mean
/// fifty commands, and re-authorizing is a diff of one file.
#[test]
fn bulk_authorization_from_a_reviewed_file() {
    let dir = mk_repo("bulk");
    std::fs::write(
        dir.join(".uni/config.toml"),
        "[verifiers]\n\"suite\" = {\"run\" = \"cargo test {{selector}} -- --exact\", \"expect\" = \"test result: ok. 1 passed\"}\n",
    )
    .unwrap();
    std::fs::write(dir.join("Cargo.toml"), "[package]\nname = \"tpl2\"\nversion = \"0.1.0\"\nedition = \"2021\"\n").unwrap();
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::write(dir.join("src/lib.rs"), "pub fn alpha() -> u32 { 1 }\npub fn beta() -> u32 { 2 }\n").unwrap();
    std::fs::create_dir_all(dir.join("tests")).unwrap();
    std::fs::write(
        dir.join("tests/it.rs"),
        "use tpl2::{alpha, beta};\n\n#[test]\nfn alpha_ok() {\n    assert_eq!(alpha(), 1);\n}\n\n#[test]\nfn beta_ok() {\n    assert_eq!(beta(), 2);\n}\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("c.uni"),
        "VERSION 0.1\nDOMAIN software\nINTENT bulk\nGOAL\n g\nCLAIM alpha REQUIRED\n  ENSURE alpha\nCLAIM beta REQUIRED\n  ENSURE beta\nVERIFY alpha\n  USING suite\nVERIFY beta\n  USING suite\nACCEPT WHEN\n  required_claims == VERIFIED\n  AND critical_failures == 0\n",
    )
    .unwrap();
    git(&dir, &["init", "-q"]);
    git(&dir, &["add", "."]);
    git(&dir, &["-c", "user.email=t@t", "-c", "user.name=t", "commit", "-q", "-m", "base"]);

    // The human reviewer writes the whole mapping and authorizes it once.
    std::fs::write(
        dir.join("reviewed.toml"),
        "[bindings.alpha]\nverifier = \"suite\"\nselector = \"alpha_ok\"\n\n[bindings.beta]\nverifier = \"suite\"\nselector = \"beta_ok\"\n",
    )
    .unwrap();
    let b = run(&["bind", "--from", "reviewed.toml"], &dir);
    assert_eq!(b.status.code(), Some(0), "{}", String::from_utf8_lossy(&b.stderr));
    let listed = run(&["bindings"], &dir);
    let out = String::from_utf8_lossy(&listed.stdout).to_string();
    assert!(out.contains("alpha_ok") && out.contains("beta_ok"), "{out}");

    let o = run(&["verify", "c.uni"], &dir);
    assert_eq!(o.status.code(), Some(0), "{}", String::from_utf8_lossy(&o.stderr));

    // Re-authorizing with one selector changed invalidates that claim's proof.
    std::fs::write(
        dir.join("reviewed.toml"),
        "[bindings.alpha]\nverifier = \"suite\"\nselector = \"alpha_ok\"\n\n[bindings.beta]\nverifier = \"suite\"\nselector = \"does_not_exist\"\n",
    )
    .unwrap();
    assert_eq!(run(&["bind", "--from", "reviewed.toml"], &dir).status.code(), Some(0));
    let o2 = run(&["verify", "c.uni"], &dir);
    assert_ne!(o2.status.code(), Some(0), "the changed selector must not reuse the old proof");

    // And the file itself is the record: it carries the act and the hash.
    let stored = std::fs::read_to_string(dir.join(".uni/bindings.toml")).unwrap();
    assert!(stored.contains("authorized_by"), "{stored}");
    assert!(stored.contains("binding_hash"), "{stored}");
}
