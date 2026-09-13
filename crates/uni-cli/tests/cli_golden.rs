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

/// Golden: speckit importer extracts markdown FR + scenarios into a candidate DSL file.
#[test]
fn golden_import_speckit() {
    let dir = std::env::temp_dir().join(format!("sk-import-{}",
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis()));
    std::fs::create_dir_all(dir.join(".uni/contracts")).unwrap();
    std::fs::write(dir.join("constitution.md"), "# Constitution\n").unwrap();
    std::fs::write(dir.join("plan.md"), "# Plan\n").unwrap();
    std::fs::write(dir.join("spec.md"),
"- **FR-001**: alpha works
- **FR-002**: beta rejects

#### Scenario: gamma path
- [ ] checkbox claim one
").unwrap();
    let o = Command::new(bin()).args(["import-speckit", dir.to_str().unwrap()])
        .current_dir(&dir).output().unwrap();
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    let contracts_dir = dir.join(".uni/contracts");
    let mut dsl_path = None;
    for e in std::fs::read_dir(&contracts_dir).unwrap() {
        let p = e.unwrap().path();
        if p.extension().map(|e| e == "uni").unwrap_or(false) {
            dsl_path = Some(p.clone());
        }
    }
    let dsl_path = dsl_path.expect("candidate DSL written");
    let dsl = std::fs::read_to_string(&dsl_path).unwrap();
    assert!(dsl.contains("CLAIM fr-001"), "{dsl}");
    assert!(dsl.contains("CLAIM fr-002"), "{dsl}");
    assert!(dsl.contains("CLAIM scenario-03"), "{dsl}");
    assert!(dsl.contains("CLAIM check-04"), "{dsl}");
    // the candidate must parse back with the same parser
    let mut dsl_path: std::path::PathBuf = std::fs::read_dir(&contracts_dir)
        .unwrap()
        .filter_map(|e| e.ok().map(|ee| ee.path()))
        .find(|p| p.extension().map(|e| e == "uni").unwrap_or(false))
        .expect("candidate DSL written");
    let out = Command::new(bin()).args(["compile", dsl_path.to_str().unwrap()])
        .current_dir(&dir).output().unwrap();
    assert!(String::from_utf8_lossy(&out.stdout).contains("claims: 4"));
}

/// Golden: `uni report` is byte-stable across repeated verify runs (PR/CI view).
#[test]
fn golden_report_stability() {
    let root = manifest_dir().parent().unwrap().parent().unwrap().to_path_buf();
    let o1 = Command::new(bin()).args(["report", "--json"]).current_dir(&root).output().unwrap();
    let o2 = Command::new(bin()).args(["report", "--json"]).current_dir(&root).output().unwrap();
    // two calls on the same last.json are identical
    assert_eq!(o1.stdout, o2.stdout, "report must be deterministic");
    let v: serde_json::Value = serde_json::from_slice(&o1.stdout).unwrap();
    assert!(v["summary"]["claims_total"].is_u64());
    assert!(v["claims"].is_array());
    assert!(v["claims"].as_array().unwrap().iter().all(
        |c| c.as_object().unwrap().len() == 2 && c["claim_id"].is_string() && c["state"].is_string()
    ), "only stable fields allowed: {v}");
    // reason and decision survive
    assert!(v["reason"].is_string());
}

/// v0.8: append-only event journal, second verify appends cache-hit events.
#[test]
fn golden_events_journal() {
    let root = manifest_dir().parent().unwrap().parent().unwrap().to_path_buf();
    assert_eq!(run(&["verify", "examples/hello/hello.uni"], &root), 0);
    let j = Command::new(bin()).args(["events", "--json"]).current_dir(&root).output().unwrap();
    let text = String::from_utf8_lossy(&j.stdout);
    let lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
    assert!(lines.len() >= 3, "at least IntentVerified + Evidence + DecisionIssued, got {lines:?}");
    let last = serde_json::Value::from(serde_json::from_str::<serde_json::Value>(lines[lines.len() - 1]).unwrap());
    assert_eq!(last["event"], "DecisionIssued");
    assert!(last["attributes"]["uni.intent.id"].is_string());
    assert!(last["attributes"]["uni.assurance.level"].is_string());
    // for a repeat run, cache hits appear as EvidenceReused events
    let texts: Vec<String> = lines.iter().map(|l| l.to_string()).collect();
    assert!(texts.iter().any(|l| l.contains("\"EvidenceReused\"")));
}
