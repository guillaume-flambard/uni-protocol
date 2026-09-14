use std::path::PathBuf;
use std::process::Command;

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_uni")
}

fn mk_repo(tag: &str) -> PathBuf {
    use std::time::{SystemTime, UNIX_EPOCH};
    let dir = std::env::temp_dir().join(format!(
        "uni-events-{tag}-{}",
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

fn setup(dir: &PathBuf) {
    std::fs::write(dir.join(".uni/config.toml"), "[verifiers]\n\"ok\" = \"true\"\n").unwrap();
    std::fs::write(
        dir.join("c.uni"),
        "VERSION 0.1\nDOMAIN software\nINTENT j\nGOAL\n g\nCLAIM x REQUIRED\n  ENSURE g\nVERIFY x\n  USING ok\nACCEPT WHEN\n  required_claims == VERIFIED\n  AND critical_failures == 0\n",
    )
    .unwrap();
    git(dir, &["init", "-q"]);
    git(dir, &["add", "."]);
    git(dir, &["-c", "user.email=t@t", "-c", "user.name=t", "commit", "-q", "-m", "a"]);
}

/// The journal rotates past its cap instead of growing without bound, the
/// archives are readable with `--all`, and the live journal is never pruned.
#[test]
fn journal_rotates_and_archives_are_readable() {
    let dir = mk_repo("rotate");
    setup(&dir);

    // Fill the journal past the 1 MiB cap with valid records.
    let filler: String = (0..15000)
        .map(|i| format!("{{\"event\":\"Filler\",\"timestamp\":\"2026-01-01T00:00:00Z\",\"attributes\":{{\"i\":\"{i}\"}}}}\n"))
        .collect();
    assert!(filler.len() > 1_048_576, "filler must exceed the cap: {}", filler.len());
    std::fs::write(dir.join(".uni/events.jsonl"), &filler).unwrap();

    // A verify appends, which triggers the rotation first.
    assert_eq!(run(&["verify", "c.uni"], &dir).status.code(), Some(0));

    let archives: Vec<_> = std::fs::read_dir(dir.join(".uni"))
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|n| n.starts_with("events.") && n != "events.jsonl" && n.ends_with(".jsonl"))
        .collect();
    assert_eq!(archives.len(), 1, "one archive after the cap: {archives:?}");
    let live = std::fs::read_to_string(dir.join(".uni/events.jsonl")).unwrap();
    assert!(live.contains("DecisionIssued"), "the live journal keeps the new run");
    assert!(!live.contains("Filler"), "the old content moved out");

    // Default read is the current window; --all includes the archive.
    let current = run(&["events", "--json"], &dir);
    let all = run(&["events", "--all", "--json"], &dir);
    let n_current = String::from_utf8_lossy(&current.stdout).lines().filter(|l| !l.trim().is_empty()).count();
    let n_all = String::from_utf8_lossy(&all.stdout).lines().filter(|l| !l.trim().is_empty()).count();
    assert!(n_all > n_current, "--all must read more: {n_current} vs {n_all}");
    assert!(String::from_utf8_lossy(&all.stdout).contains("Filler"));

    // doctor reports the size and the rotation cap.
    let doctor = run(&["doctor"], &dir);
    let out = String::from_utf8_lossy(&doctor.stdout);
    assert!(out.contains("rotates at 1024 KiB"), "{out}");
    assert!(out.contains("1 archive(s)"), "{out}");
}
