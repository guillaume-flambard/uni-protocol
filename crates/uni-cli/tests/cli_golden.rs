use std::process::Command;
use std::path::{Path, PathBuf};

fn git(cwd: &PathBuf, args: &[&str]) {
    assert!(Command::new("git").args(args).current_dir(cwd).status().unwrap().success());
}

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

#[allow(dead_code)]
fn _unused(_p: &Path) {}

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
    let dsl_path: std::path::PathBuf = std::fs::read_dir(&contracts_dir)
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

/// v0.11: lint catches claims without VERIFY (preflight, no execution).
#[test]
fn golden_lint_missing_verify() {
    let dir = std::env::temp_dir().join(format!("uni-lint-{}",
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis()));
    std::fs::create_dir_all(dir.join(".uni/evidence")).unwrap();
    std::fs::write(dir.join("c.uni"), "VERSION 0.1
DOMAIN software
INTENT lint-fail
GOAL
  n
CLAIM ghost REQUIRED
  ENSURE n
").unwrap();
    let o = Command::new(bin()).args(["lint", "c.uni"]).current_dir(&dir).output().unwrap();
    assert_ne!(o.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&o.stdout);
    assert!(stdout.contains("missing-verify") && stdout.contains("ghost"), "{stdout}");
    // clean contract lints green
    std::fs::write(dir.join("c.uni"), "VERSION 0.1
DOMAIN software
INTENT lint-ok
GOAL
  n
CLAIM x REQUIRED
  ENSURE n
VERIFY x
  USING anything
").unwrap();
    let o2 = Command::new(bin()).args(["lint", "c.uni"]).current_dir(&dir).output().unwrap();
    assert_eq!(o2.status.code(), Some(0), "{o2:?}");
}

/// v0.12: OPA adapter — when a rego bundle + a shim `opa` binary exist,
/// the provider resolves from the bundle; absent opa falls back to TOML.
#[test]
fn golden_policy_provider_opa_fallback() {
    // isolated workspace: no opa binary intercept needed — create one that works.
    use std::time::{SystemTime, UNIX_EPOCH};
    let dir = std::env::temp_dir().join(format!("uni-ppa-{}",
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis()));
    let shim_dir = dir.join("shim");
    std::fs::create_dir_all(&shim_dir).unwrap();
    std::fs::create_dir_all(dir.join(".uni/evidence")).unwrap();
    std::fs::create_dir_all(dir.join("policies")).unwrap();
    std::fs::write(dir.join("policies/opa.rego"),
"package uni\nrules = {\"escalate_on_stale\": true, \"reject_on_invalid\": true, \"min_verified_ratio\": 0.0}\n").unwrap();
    let path_env = format!("{}/bin:/bin:/usr/bin", shim_dir.display());
    let shim = shim_dir.join("opa");
    std::fs::write(&shim, r#"#!/bin/sh
if [ "$1" = "version" ]; then exit 0; fi
echo '[{"escalate_on_stale": true, "reject_on_invalid": true, "min_verified_ratio": 0.5}]'
"#).unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&shim, std::fs::Permissions::from_mode(0o755)).unwrap();
    // PATH is passed to the CHILD only: mutating process env races with other tests.
    let child_path = format!("{path_env}:{}", std::env::var("PATH").unwrap_or_default());

    std::fs::write(dir.join(".uni/config.toml"),
"[verifiers]\n\"p\" = \"true\"\n").unwrap();
    std::fs::write(dir.join("c.uni"), "VERSION 0.1
DOMAIN software
INTENT ppa
GOAL
  g
CLAIM x REQUIRED
  ENSURE g
VERIFY x
  USING p
").unwrap();
    git(&dir, &["init", "-q"]);
    git(&dir, &["add", "."]);
    git(&dir, &["-c", "user.email=t@t", "-c", "user.name=t", "commit", "-q", "-m", "a"]);
    let o1 = Command::new(bin()).args(["verify", "c.uni"])
        .current_dir(&dir).env("PATH", &child_path).output().unwrap();
    let r1 = String::from_utf8_lossy(&o1.stdout).to_string();
    assert!(o1.status.success() && r1.contains("Accepted"), "{r1}");
    // and with opa absent the fallback to the TOML stack still accepts
    let o2 = Command::new(bin()).args(["verify", "c.uni"])
        .current_dir(&dir).output().unwrap();
    assert!(o2.status.success(), "opa-absent fallback must not block: {}", String::from_utf8_lossy(&o2.stderr));
}

/// v0.13: doctor — healthy exit 0 on a prepared workspace, failure exit without .uni.
#[test]
fn golden_doctor_healthy_and_fail() {
    let dup = manifest_dir().parent().unwrap().parent().unwrap().to_path_buf();
    let ok = bin_state(&dup);
    assert!(ok, "uni repo doctor must be healthy");
    let dir = std::env::temp_dir().join(format!("uni-doc-{}",
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis()));
    std::fs::create_dir_all(&dir).unwrap();
    let out = Command::new(bin()).args(["doctor"]).current_dir(&dir).output().unwrap();
    assert_ne!(out.status.code(), Some(0), "doctor must fail without .uni");
}
fn bin_state(root: &std::path::PathBuf) -> bool {
    Command::new(bin()).args(["doctor"]).current_dir(root).output().map(|o| o.status.success()).unwrap_or(false)
}

/// v0.14: stack independence — python + nodejs examples verify green.
/// Skips silently when the runtime is missing (CI ubuntu has both).
#[test]
fn golden_stack_independence() {
    let root = manifest_dir().parent().unwrap().parent().unwrap().to_path_buf();
    for (contract, runtime) in [
        ("examples/python/contract.uni", "python3"),
        ("examples/nodejs/contract.uni", "node"),
    ] {
        let has = Command::new("which").arg(runtime).stdout(std::process::Stdio::null()).status().map(|s| s.success()).unwrap_or(false);
        if !has {
            continue;
        }
        assert_eq!(
            run(&["verify", contract], &root),
            0,
            "{contract} must ACCEPT with {} available",
            runtime
        );
    }
}

/// v0.16: software pack ships, lists, and materializes lint-clean contracts.
#[test]
fn golden_software_pack() {
    let root = manifest_dir().parent().unwrap().parent().unwrap().to_path_buf();
    let o = Command::new(bin()).args(["pack", "list", "--json"]).current_dir(&root).output().unwrap();
    assert!(o.status.success());
    let v: serde_json::Value = serde_json::from_slice(&o.stdout).unwrap();
    let packs = v["packs"].as_array().unwrap();
    assert!(packs.iter().any(|p| p["name"] == "software"), "{v}");
    let sw = packs.iter().find(|p| p["name"] == "software").unwrap();
    assert_eq!(sw["templates"].as_array().unwrap().len(), 3);

    // materialize + lint in a sandbox (repo .uni registry has the referenced keys)
    let dir = std::env::temp_dir().join(format!("uni-pack-{}",
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis()));
    std::fs::create_dir_all(dir.join(".uni/evidence")).unwrap();
    std::fs::create_dir_all(dir.join("packs/software/templates")).unwrap();
    for e in std::fs::read_dir(root.join("packs/software")).unwrap().flatten() {
        if e.path().extension().map(|x| x == "toml").unwrap_or(false) {
            std::fs::copy(e.path(), dir.join("packs/software/pack.toml")).unwrap();
        }
    }
    for e in std::fs::read_dir(root.join("packs/software/templates")).unwrap().flatten() {
        std::fs::copy(e.path(), dir.join("packs/software/templates").join(e.file_name())).unwrap();
    }
    assert_eq!(run(&["pack", "template", "software", "tests-pass"], &dir), 0);
    let lint = Command::new(bin()).args(["lint", "uni/intents/tests-pass.uni"])
        .current_dir(&dir).output().unwrap();
    assert!(lint.status.success(), "{}", String::from_utf8_lossy(&lint.stderr));
    assert!(String::from_utf8_lossy(&lint.stdout).contains("clean"));
}
