use std::process::Command;
use std::path::{Path, PathBuf};

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_uni")
}

fn run(cmd: &[&str], cwd: &PathBuf) -> i32 {
    Command::new(bin()).args(cmd).current_dir(cwd).status().unwrap().code().unwrap()
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Golden: compile output is stable (no timestamps).
#[test]
fn golden_compile() {
    let root = manifest_dir().parent().unwrap().parent().unwrap().to_path_buf(); // repo root
    let contract = root.join("examples/hello/hello.uni");
    let o = Command::new(bin()).args(["compile", contract.to_str().unwrap()])
        .current_dir(&root).output().unwrap();
    let stdout = String::from_utf8_lossy(&o.stdout);
    assert_eq!(stdout,
        "intent: hello\nclaims: 1\nverifications: 1\n--json for canonical IR\n");
}

/// Golden: compile --json shape is stable (keys + counts, volatile values masked).
#[test]
fn golden_compile_json_shape() {
    let root = manifest_dir().parent().unwrap().parent().unwrap().to_path_buf();
    let contract = root.join("examples/hello/hello.uni");
    let o = Command::new(bin()).args(["--json", "compile", contract.to_str().unwrap()])
        .current_dir(&root).output().unwrap();
    let v: serde_json::Value = serde_json::from_slice(&o.stdout).unwrap();
    assert_eq!(v["uni_version"], "0.1");
    assert_eq!(v["intent"]["id"], "hello");
    assert_eq!(v["claims"].as_array().unwrap().len(), 1);
    assert_eq!(v["verification"][0]["claim_id"], "binary-builds");
}

/// Diff-of-setup golden: verify exit codes follow the decision (0 accepted, non-zero otherwise).
#[test]
fn golden_verify_exit_codes() {
    let root = manifest_dir().parent().unwrap().parent().unwrap().to_path_buf();
    assert_eq!(run(&["verify", "examples/hello/hello.uni"], &root), 0);
    assert_eq!(run(&["verify", "examples/multi/multi.uni"], &root), 0);
    assert_eq!(run(&["verify", "examples/booking/booking.uni"], &root), 0);
}

fn _unused(p: &Path) {}
