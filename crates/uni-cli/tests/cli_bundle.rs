use std::path::PathBuf;
use std::process::Command;

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_uni")
}

fn mk_repo(tag: &str) -> PathBuf {
    use std::time::{SystemTime, UNIX_EPOCH};
    let dir = std::env::temp_dir().join(format!(
        "uni-bundle-{tag}-{}",
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

fn setup(dir: &PathBuf, requirement: bool) {
    std::fs::write(
        dir.join(".uni/config.toml"),
        "[verifiers]\n\"p\" = \"true\"\n",
    )
    .unwrap();
    let req = if requirement {
        "  REQUIRE behavior(\"does it\")\n"
    } else {
        ""
    };
    std::fs::write(
        dir.join("c.uni"),
        format!("VERSION 0.1\nDOMAIN software\nINTENT bnd\nGOAL\n g\nCLAIM x REQUIRED\n  ENSURE g\nVERIFY x\n  USING p\n{req}ACCEPT WHEN\n  required_claims == VERIFIED\n  AND critical_failures == 0\n"),
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
            "a",
        ],
    );
}

fn run(args: &[&str], dir: &PathBuf) -> std::process::Output {
    Command::new(bin())
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap()
}

/// Happy path: export then verify offline, coverage complete.
#[test]
fn bundle_export_verify_round_trip() {
    let dir = mk_repo("ok");
    setup(&dir, false);
    assert_eq!(run(&["verify", "c.uni"], &dir).status.code(), Some(0));
    let out = run(&["bundle", "export", "c.uni", "--out", "b.jsonl"], &dir);
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v = run(&["bundle", "verify", "b.jsonl"], &dir);
    assert_eq!(
        v.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&v.stderr)
    );
    let stdout = String::from_utf8_lossy(&v.stdout);
    assert!(
        stdout.contains("claims covered by evidence: 1/1"),
        "{stdout}"
    );
    assert!(stdout.contains("OK: true"), "{stdout}");
}

/// Transport tampering is detected offline by per-record sha256.
#[test]
fn bundle_verify_detects_transport_tampering() {
    let dir = mk_repo("tamper");
    setup(&dir, false);
    assert_eq!(run(&["verify", "c.uni"], &dir).status.code(), Some(0));
    assert_eq!(
        run(&["bundle", "export", "c.uni", "--out", "b.jsonl"], &dir)
            .status
            .code(),
        Some(0)
    );

    let text = std::fs::read_to_string(dir.join("b.jsonl")).unwrap();
    let mut lines: Vec<String> = text.lines().map(str::to_string).collect();
    for l in lines.iter_mut() {
        let mut rec: serde_json::Value = serde_json::from_str(l).unwrap();
        if rec["kind"] == "evidence" {
            rec["body"]["exit_code"] =
                serde_json::json!(if rec["body"]["exit_code"].as_i64() == Some(0) {
                    1
                } else {
                    0
                });
            *l = serde_json::to_string(&rec).unwrap();
            break;
        }
    }
    std::fs::write(dir.join("t.jsonl"), lines.join("\n") + "\n").unwrap();
    let v = run(&["bundle", "verify", "t.jsonl"], &dir);
    assert_ne!(v.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&v.stdout).contains("sha256 mismatch"));
}

/// A requirement without its binding in the bundle is a cross-check error.
#[test]
fn bundle_verify_flags_missing_binding() {
    let dir = mk_repo("binding");
    setup(&dir, true);
    let b = run(
        &[
            "bind",
            "--claim",
            "x",
            "--verifier",
            "p",
            "--requirement",
            "behavior(\"does it\")",
        ],
        &dir,
    );
    assert_eq!(
        b.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&b.stderr)
    );
    assert_eq!(run(&["verify", "c.uni"], &dir).status.code(), Some(0));
    assert_eq!(
        run(&["bundle", "export", "c.uni", "--out", "b.jsonl"], &dir)
            .status
            .code(),
        Some(0)
    );
    assert_eq!(
        run(&["bundle", "verify", "b.jsonl"], &dir).status.code(),
        Some(0)
    );

    // Remove the binding record from the bundle: cross-check must fail.
    let text = std::fs::read_to_string(dir.join("b.jsonl")).unwrap();
    let kept: Vec<&str> = text
        .lines()
        .filter(|l| {
            serde_json::from_str::<serde_json::Value>(l)
                .map(|v| v["kind"] != "binding")
                .unwrap_or(true)
        })
        .collect();
    std::fs::write(dir.join("nobind.jsonl"), kept.join("\n") + "\n").unwrap();
    let v = run(&["bundle", "verify", "nobind.jsonl"], &dir);
    assert_ne!(v.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&v.stdout);
    assert!(stdout.contains("no binding is bundled"), "{stdout}");
}

/// Bundle verification never writes to the live cache.
#[test]
fn bundle_verify_is_read_only() {
    let dir = mk_repo("readonly");
    setup(&dir, false);
    assert_eq!(run(&["verify", "c.uni"], &dir).status.code(), Some(0));
    assert_eq!(
        run(&["bundle", "export", "c.uni", "--out", "b.jsonl"], &dir)
            .status
            .code(),
        Some(0)
    );
    let before_decision = std::fs::read_to_string(dir.join(".uni/decisions/last.json")).unwrap();
    let events_before = std::fs::read_to_string(dir.join(".uni/events.jsonl")).unwrap();
    assert_eq!(
        run(&["bundle", "verify", "b.jsonl"], &dir).status.code(),
        Some(0)
    );
    assert_eq!(
        before_decision,
        std::fs::read_to_string(dir.join(".uni/decisions/last.json")).unwrap()
    );
    assert_eq!(
        events_before,
        std::fs::read_to_string(dir.join(".uni/events.jsonl")).unwrap(),
        "bundle verify must not append events"
    );
}
