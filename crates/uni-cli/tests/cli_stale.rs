use std::path::PathBuf;
use std::process::Command;

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_uni")
}

fn mk_repo(tag: &str) -> PathBuf {
    use std::time::{SystemTime, UNIX_EPOCH};
    let dir = std::env::temp_dir().join(format!(
        "uni-stale-{tag}-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis()
    ));
    std::fs::create_dir_all(dir.join(".uni/evidence")).unwrap();
    dir
}

fn git(cwd: &PathBuf, args: &[&str]) {
    assert!(Command::new("git")
        .args(args)
        .current_dir(cwd)
        .status()
        .unwrap()
        .success());
}

fn run(args: &[&str], dir: &PathBuf) -> std::process::Output {
    Command::new(bin())
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap()
}

fn setup(dir: &PathBuf) {
    std::fs::write(
        dir.join(".uni/config.toml"),
        "[verifiers.\"ledger.integrity\"]\nrun = \"grep -q 'A: u32 = 1' src/ledger.rs\"\nfiles = [\"src/ledger.rs\"]\n",
    )
    .unwrap();
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::write(dir.join("src/ledger.rs"), "const A: u32 = 1;\n").unwrap();
    std::fs::write(
        dir.join("c.uni"),
        "VERSION 0.1\nDOMAIN software\nINTENT pay\nGOAL\n g\nCLAIM ledger-integrity REQUIRED\n  ENSURE the ledger invariant holds\nVERIFY ledger-integrity\n  USING ledger.integrity\nACCEPT WHEN\n  required_claims == VERIFIED\n  AND critical_failures == 0\n",
    )
    .unwrap();
    git(dir, &["init", "-q"]);
    git(dir, &["add", "."]);
    git(
        dir,
        &[
            "-c",
            "user.email=t@t",
            "-c",
            "user.name=t",
            "commit",
            "-q",
            "-m",
            "82ab31",
        ],
    );
}

/// The central experience: a proof established on one revision, drift, and
/// `uni explain` naming exactly which files moved and which dimension drifted.
#[test]
fn explain_names_the_drift_in_plain_words() {
    let dir = mk_repo("drift");
    setup(&dir);
    assert_eq!(
        run(&["verify", "c.uni"], &dir).status.code(),
        Some(0),
        "proof established"
    );

    // Ship a different revision of the watched subject.
    std::fs::write(dir.join("src/ledger.rs"), "const A: u32 = 2;\n").unwrap();
    git(&dir, &["add", "."]);
    git(
        &dir,
        &[
            "-c",
            "user.email=t@t",
            "-c",
            "user.name=t",
            "commit",
            "-q",
            "-m",
            "912be0",
        ],
    );

    let v = run(&["verify", "c.uni"], &dir);
    assert_ne!(v.status.code(), Some(0), "a drifted proof must not accept");
    let human = String::from_utf8_lossy(&v.stdout).to_string();
    assert!(human.contains("artifact moved from"), "{human}");

    let e = run(&["explain"], &dir);
    let ex = String::from_utf8_lossy(&e.stdout).to_string();
    assert!(ex.contains("CLAIM ledger-integrity"), "{ex}");
    assert!(ex.contains("watched      src/ledger.rs"), "{ex}");
    assert!(ex.contains("commit_changed"), "{ex}");
    assert!(ex.contains("changed: src/ledger.rs"), "{ex}");
    assert!(
        ex.contains("action       uni verify ledger-integrity"),
        "{ex}"
    );

    // The journal carries the reason labels, so a machine reader gets them too.
    let journal = std::fs::read_to_string(dir.join(".uni/events.jsonl")).unwrap();
    assert!(journal.contains("uni.stale.reasons"), "{journal}");
    assert!(journal.contains("commit_changed"), "{journal}");
    assert!(journal.contains("subject_changed"), "{journal}");

    // A further run must not lose the narration: the drift is remembered until
    // the claim is proved again.
    let _ = run(&["verify", "c.uni"], &dir);
    let ex2 = String::from_utf8_lossy(&run(&["explain"], &dir).stdout).to_string();
    assert!(
        ex2.contains("changed: src/ledger.rs"),
        "drift must survive later runs: {ex2}"
    );

    // And the JSON decision records them per claim.
    let last: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(dir.join(".uni/decisions/last.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        last["stale"]["ledger-integrity"][0]["dimension"], "commit_changed",
        "{last}"
    );
    assert!(
        last["commit"].is_string(),
        "the decision names the revision it was made against"
    );
}

/// A registry change is a named dimension in the narration, not just a re-run.
#[test]
fn explain_names_a_verifier_config_change() {
    let dir = mk_repo("registry");
    setup(&dir);
    assert_eq!(run(&["verify", "c.uni"], &dir).status.code(), Some(0));

    std::fs::write(
        dir.join(".uni/config.toml"),
        "[verifiers.\"ledger.integrity\"]\nrun = \"grep -q 'A: u32 = 1' src/ledger.rs\"\nfiles = [\"src/ledger.rs\"]\ntimeout = 42\n",
    )
    .unwrap();
    git(&dir, &["add", "."]);
    git(
        &dir,
        &[
            "-c",
            "user.email=t@t",
            "-c",
            "user.name=t",
            "commit",
            "-q",
            "-m",
            "registry",
        ],
    );
    let v = run(&["verify", "c.uni"], &dir);
    let human = String::from_utf8_lossy(&v.stdout).to_string();
    // The boundary is named on the run that crosses it, and the proof is
    // re-established, so the outcome is Accepted: staleness forces
    // re-verification, it does not fail a claim that re-proves.
    assert!(
        human.contains("REGISTRY_CHANGED") || human.contains("Accepted"),
        "{human}"
    );
    let journal = std::fs::read_to_string(dir.join(".uni/events.jsonl")).unwrap();
    assert!(journal.contains("verifier_config_changed"), "{journal}");
    assert!(
        journal.contains("trusted verifier registry changed"),
        "{journal}"
    );
}

#[test]
fn annotations_point_at_the_file_that_moved() {
    let dir = mk_repo("ann");
    setup(&dir);
    let _ = run(&["verify", "c.uni"], &dir);
    std::fs::write(dir.join("src/ledger.rs"), "const A: u32 = 2;\n").unwrap();
    let _ = run(&["verify", "c.uni"], &dir);

    let out = run(&["explain", "--annotations"], &dir);
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    assert!(
        text.contains("::warning file=src/ledger.rs,title=claim ledger-integrity"),
        "{text}"
    );
    assert!(text.contains("watched subject moved"), "{text}");
    assert!(text.contains("uni verify ledger-integrity"), "{text}");
}
