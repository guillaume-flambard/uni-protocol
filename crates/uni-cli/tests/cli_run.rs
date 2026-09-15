use std::path::PathBuf;
use std::process::Command;

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_uni")
}

fn mk_repo(tag: &str) -> PathBuf {
    use std::time::{SystemTime, UNIX_EPOCH};
    let dir = std::env::temp_dir().join(format!(
        "uni-run-{tag}-{}",
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
            "base",
        ],
    );
}

/// The executor's exit code is data, not truth: a failed agent whose work is
/// sound still accepts, and a successful agent whose work is not still fails.
#[test]
fn executor_exit_code_never_decides() {
    let dir = mk_repo("verdict");
    setup(&dir);

    // Executor fails, but does the work: ACCEPTED, exit 0.
    let o = run(
        &[
            "run",
            "c.uni",
            "--",
            "printf 'ready\\n' > marker.txt; exit 7",
        ],
        &dir,
    );
    assert_eq!(
        o.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&o.stderr)
    );
    let out = String::from_utf8_lossy(&o.stdout);
    assert!(out.contains("executor exit 7"), "{out}");
    assert!(out.contains("Accepted"), "{out}");

    // Executor succeeds, but breaks the work: REJECTED/required, exit non-zero.
    let o2 = run(
        &[
            "run",
            "c.uni",
            "--",
            "printf 'not yet\\n' > marker.txt; exit 0",
        ],
        &dir,
    );
    assert_ne!(
        o2.status.code(),
        Some(0),
        "the decision, not the executor, drives the exit"
    );
    let out2 = String::from_utf8_lossy(&o2.stdout);
    assert!(out2.contains("executor exit 0"), "{out2}");
    assert!(out2.contains("EvidenceRequired"), "{out2}");
}

/// The executor is killed on timeout and the verification still runs.
#[test]
fn executor_timeout_is_killed_and_verified() {
    let dir = mk_repo("timeout");
    setup(&dir);
    let o = run(
        &["run", "c.uni", "--timeout-ms", "400", "--", "sleep 30"],
        &dir,
    );
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

/// `uni run --brief` writes the work order into the workspace and exports its
/// absolute path as UNI_BRIEF, so an executor that was never told about the
/// brief can still read it. The study's finding is why this exists: a brief that
/// is merely present is ignored.
#[test]
fn brief_flag_hands_the_work_order_to_the_executor() {
    let dir = mk_repo("brief");
    setup(&dir);

    // The executor's job: prove it saw a readable work order by dumping it and
    // its own path to a file we can inspect.
    let o = run(
        &[
            "run",
            "c.uni",
            "--brief",
            "--",
            "printf '%s\\n' \"$UNI_BRIEF\" > seen.txt && cat \"$UNI_BRIEF\" >> seen.txt && printf 'ready\\n' > marker.txt",
        ],
        &dir,
    );
    let out = format!(
        "{}{}",
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    );
    assert_eq!(o.status.code(), Some(0), "{out}");

    let seen = std::fs::read_to_string(dir.join("seen.txt")).unwrap();
    let mut lines = seen.lines();
    let path = lines.next().unwrap();
    assert!(
        path.ends_with(".uni/brief.md") && path.starts_with('/'),
        "UNI_BRIEF must be an absolute path to the brief, got: {path}"
    );
    let body: String = lines.collect::<Vec<_>>().join("\n");
    assert!(
        body.contains("x"),
        "the brief must describe the claim: {body}"
    );
    assert!(
        body.to_lowercase().contains("ready"),
        "the brief must name the evidence the claim needs: {body}"
    );

    // And without the flag, nothing is written and no variable is set.
    let dir2 = mk_repo("nobrief");
    setup(&dir2);
    let o2 = run(
        &[
            "run",
            "c.uni",
            "--",
            "printf '%s' \"${UNI_BRIEF:-unset}\" > seen2.txt && printf 'ready\\n' > marker.txt",
        ],
        &dir2,
    );
    assert_eq!(o2.status.code(), Some(0));
    assert_eq!(
        std::fs::read_to_string(dir2.join("seen2.txt")).unwrap(),
        "unset",
        "without --brief, the executor must not be given a brief path"
    );
    assert!(
        !dir2.join(".uni/brief.md").exists(),
        "without --brief, no work order is written"
    );
}
