use std::path::PathBuf;
use std::process::Command;

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_uni")
}

fn mk_repo(tag: &str) -> PathBuf {
    use std::time::{SystemTime, UNIX_EPOCH};
    let dir = std::env::temp_dir().join(format!(
        "uni-run-{tag}-{}",
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

/// A contract whose claim passes when `marker.txt` says ready.
fn setup(dir: &PathBuf) {
    std::fs::write(
        dir.join(".uni/config.toml"),
        "[verifiers.\"ready\"]\nrun = \"grep -q ready marker.txt\"\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("c.uni"),
        "VERSION 0.1\nDOMAIN software\nINTENT r\nGOAL\n g\nCLAIM x REQUIRED\n  ENSURE marker says ready\nVERIFY x\n  USING ready\nACCEPT WHEN\n  required_claims == VERIFIED\n  AND critical_failures == 0\n",
    )
    .unwrap();
    std::fs::write(dir.join("marker.txt"), "not yet\n").unwrap();
    git(dir, &["init", "-q"]);
    git(dir, &["add", "."]);
    git(dir, &["-c", "user.email=t@t", "-c", "user.name=t", "commit", "-q", "-m", "base"]);
}

/// The executor's exit code is data, not truth: a failed agent whose work is
/// sound still accepts, and a successful agent whose work is not still fails.
#[test]
fn executor_exit_code_never_decides() {
    let dir = mk_repo("verdict");
    setup(&dir);

    // Executor fails, but does the work: ACCEPTED, exit 0.
    let o = run(&["run", "c.uni", "--", "printf 'ready\\n' > marker.txt; exit 7"], &dir);
    assert_eq!(o.status.code(), Some(0), "{}", String::from_utf8_lossy(&o.stderr));
    let out = String::from_utf8_lossy(&o.stdout);
    assert!(out.contains("executor exit 7"), "{out}");
    assert!(out.contains("Accepted"), "{out}");

    // Executor succeeds, but breaks the work: REJECTED/required, exit non-zero.
    let o2 = run(&["run", "c.uni", "--", "printf 'not yet\\n' > marker.txt; exit 0"], &dir);
    assert_ne!(o2.status.code(), Some(0), "the decision, not the executor, drives the exit");
    let out2 = String::from_utf8_lossy(&o2.stdout);
    assert!(out2.contains("executor exit 0"), "{out2}");
    assert!(out2.contains("EvidenceRequired"), "{out2}");
}

/// The executor is killed on timeout and the verification still runs.
#[test]
fn executor_timeout_is_killed_and_verified() {
    let dir = mk_repo("timeout");
    setup(&dir);
    let o = run(&["run", "c.uni", "--timeout-ms", "400", "--", "sleep 30"], &dir);
    let out = String::from_utf8_lossy(&o.stdout);
    assert!(out.contains("killed after 400ms"), "{out}");
    // Marker unchanged, so the claim cannot be proven.
    assert_ne!(o.status.code(), Some(0));
}

/// Each run is journaled as an execution, so the audit trail names the command
/// and its exit code next to the evidence.
#[test]
fn executor_run_is_journaled() {
    let dir = mk_repo("journal");
    setup(&dir);
    let _ = run(&["run", "c.uni", "--", "true"], &dir);
    let journal = std::fs::read_to_string(dir.join(".uni/events.jsonl")).unwrap();
    assert!(journal.contains("ExecutionRun"), "{journal}");
    assert!(journal.contains("uni.execution.exit_code"), "{journal}");
    assert!(journal.contains("uni.execution.command"), "{journal}");
}
