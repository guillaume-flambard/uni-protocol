use anyhow::{anyhow, Context, Result};
mod events;
use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(name = "uni", version, about = "Outcome Assurance Protocol CLI")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
    #[arg(long, global = true)]
    json: bool,
}

#[derive(Subcommand)]
enum Cmd {
    Init,
    Compile { file: PathBuf },
    Verify { file: PathBuf },
    Explain { claim_or_intent: Option<String> },
    Inspect { file: PathBuf },
    ImportSpeckit { dir: PathBuf },
    Report,
    Events,
    Lint { file: PathBuf },
}

fn dot_uni() -> PathBuf {
    PathBuf::from(".uni")
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Init => cmd_init(),
        Cmd::Compile { file } => cmd_compile(&file, cli.json),
        Cmd::Verify { file } => cmd_verify(&file, cli.json),
        Cmd::Explain { claim_or_intent } => cmd_explain(claim_or_intent, cli.json),
        Cmd::Inspect { file } => cmd_inspect(&file, cli.json),
        Cmd::ImportSpeckit { dir } => cmd_import_speckit(&dir, cli.json),
        Cmd::Report => cmd_report(cli.json),
        Cmd::Events => cmd_events(cli.json, 50),
        Cmd::Lint { file } => cmd_lint(&file, cli.json),
    }
}

fn cmd_lint(file: &Path, as_json: bool) -> Result<()> {
    let (ir, _) = load_contract(file)?;
    let registry = uni_verify::load_registry(&dot_uni());
    // (severity, kind, message); severity 2 = error, 1 = warning
    let mut findings: Vec<(u8, String, String)> = vec![];
    for c in &ir.claims {
        if !ir.verification.iter().any(|v| v.claim_id == c.id) {
            findings.push((2, "missing-verify".into(), format!("claim '{}' has no VERIFY", c.id)));
        }
    }
    if !registry.is_empty() {
        for v in &ir.verification {
            if v.verifier_ref != "shell" && !registry.contains_key(&v.verifier_ref) {
                findings.push((1, "unknown-verifier".into(), format!(
                    "VERIFY {} uses '{}' not found in .uni/config.toml (may fail at verify time)",
                    v.claim_id, v.verifier_ref
                )));
            }
        }
    }
    for (i, a) in ir.verification.iter().enumerate() {
        for b in ir.verification.iter().skip(i + 1) {
            if b.claim_id == a.claim_id && b.verifier_ref == a.verifier_ref && b.inline_shell == a.inline_shell {
                findings.push((1, "duplicate-verify".into(), format!(
                    "VERIFY '{}' → '{}' declared twice", a.claim_id, a.verifier_ref
                )));
            }
        }
    }
    let errors = findings.iter().filter(|(s, _, _)| *s == 2).count();
    let warns = findings.len() - errors;
    if as_json {
        println!(
            "{}",
            serde_json::json!({
                "intent": ir.intent.id,
                "errors": errors,
                "warnings": warns,
                "findings": findings.iter().map(|(sev, kind, msg)| serde_json::json!({
                    "severity": if *sev == 2 { "error" } else { "warning" },
                    "kind": kind,
                    "message": msg,
                })).collect::<Vec<_>>(),
            })
        );
    } else {
        println!("uni lint — {}", ir.intent.id);
        for (sev, kind, msg) in &findings {
            println!("  {} {kind:<18} {msg}", if *sev == 2 { "ERROR" } else { "WARN " });
        }
        if findings.is_empty() {
            println!("  clean: coverage complete, registry refs ok");
        }
        println!("\nerrors={errors} warnings={warns}");
    }
    if errors > 0 {
        return Err(anyhow!("uni lint: {errors} error(s)"));
    }
    Ok(())
}

fn cmd_events(as_json: bool, max: usize) -> Result<()> {
    let evts = events::read_all()?;
    let n = evts.len();
    if as_json {
        for e in &evts {
            println!("{}", serde_json::to_string(e)?);
        }
    } else {
        for e in evts.iter().skip(n.saturating_sub(max)) {
            let attrs: Vec<String> = e
                .attributes
                .iter()
                .map(|(k, v)| match v {
                    serde_json::Value::String(s) => format!("{k}={s}"),
                    other => format!("{k}={other}"),
                })
                .collect();
            println!("{:<22} {} {}", e.event, e.timestamp, attrs.join(" "));
        }
        println!("
{n} events (append-only .uni/events.jsonl)");
    }
    Ok(())
}

/// CI/PR-facing view of the last decision: stable shape, no volatile fields
/// (no timestamps, durations, excerpts). One byte change = real state change.
fn stable_report() -> Result<serde_json::Value> {
    let path = dot_uni().join("decisions").join("last.json");
    let text = std::fs::read_to_string(&path).context("no decision yet (run uni verify first)")?;
    let v: serde_json::Value = serde_json::from_str(&text)?;
    let claims = v["claims"].as_array().cloned().unwrap_or_default();
    let total = claims.len();
    let verified = claims.iter().filter(|c| c["state"] == "Valid").count();
    Ok(serde_json::json!({
        "intent": v.as_object().and_then(|o| o.get("intent")).cloned().unwrap_or(serde_json::Value::Null),
        "decision": v["decision"],
        "reason": v["reason"],
        "summary": {"claims_total": total, "claims_verified": verified},
        "claims": claims.iter().map(|c| serde_json::json!({
            "claim_id": c["claim_id"], "state": c["state"],
        })).collect::<Vec<_>>(),
    }))
}

fn cmd_report(as_json: bool) -> Result<()> {
    let r = stable_report()?;
    if as_json {
        println!("{}", serde_json::to_string_pretty(&r)?);
        return Ok(());
    }
    println!("UNI Assurance");
    println!("-------------");
    if let Some(i) = r["intent"].as_object() {
        println!("Intent      {}", i.get("id").and_then(|x| x.as_str()).unwrap_or("?"));
    }
    let s = &r["summary"];
    println!("Claims      {}/{} verified",
        s["claims_verified"], s["claims_total"]);
    let a = assurance_level_x(&r);
    println!("Assurance   A{a} ({})", match a {
        0 => "DECLARED", 1 => "ARTIFACT", 2 => "VERIFIED",
        3 => "INDEPENDENTLY_VERIFIED", 4 => "ATTESTED", _ => "?",
    });
    if let Some(claims) = r["claims"].as_array() {
        println!("\nClaims");
        for c in claims {
            let mark = match c["state"].as_str() {
                Some("Valid") => "PASS",
                Some("Stale") => "STALE",
                _ => "FAIL",
            };
            println!("  {:<24} {mark}", c["claim_id"].as_str().unwrap_or("?"));
        }
    }
    println!("\nDecision    {}", r["decision"].as_str().unwrap_or("?"));
    println!("Reason      {}", r["reason"].as_str().unwrap_or("?"));
    Ok(())
}

fn cmd_init() -> Result<()> {
    for d in ["contracts", "evidence", "decisions", "artifacts"] {
        std::fs::create_dir_all(dot_uni().join(d))?;
    }
    std::fs::create_dir_all("uni/intents")?;
    let cfg = dot_uni().join("config.toml");
    if !cfg.exists() {
        std::fs::write(
            &cfg,
            r#"[verifiers]
"project.check" = "cargo check"
"project.tests" = "cargo test"
"project.build" = "cargo build"
"#,
        )?;
    }
    println!("initialized .uni/ + uni/intents/");
    Ok(())
}

fn load_contract(file: &Path) -> Result<(uni_ir::Ir, uni_parser::Contract)> {
    let src = std::fs::read_to_string(file)
        .with_context(|| format!("read {}", file.display()))?;
    let ast = uni_parser::parse(&src)?;
    let ir = uni_ir::compile(&ast)?;
    Ok((ir, ast))
}

fn cmd_compile(file: &Path, as_json: bool) -> Result<()> {
    let (ir, _) = load_contract(file)?;
    let out = uni_ir::to_json(&ir)?;
    if as_json {
        println!("{out}");
    } else {
        println!("intent: {}", ir.intent.id);
        println!("claims: {}", ir.claims.len());
        println!("verifications: {}", ir.verification.len());
        println!("--json for canonical IR");
    }
    Ok(())
}

fn cmd_inspect(file: &Path, as_json: bool) -> Result<()> {
    cmd_compile(file, as_json)
}

fn cmd_verify(file: &Path, as_json: bool) -> Result<()> {
    let (ir, _) = load_contract(file)?;
    let ws = std::env::current_dir()?;
    let du = dot_uni();
    let (cur_sha, cur_dirty) = uni_evidence::git_info(&ws);
    let registry = uni_verify::load_registry(&du);
    // 1) Try persisted evidence first (cheap, content-addressed).
    let mut stored = vec![];
    let mut need_run = vec![];
    let mut journal: Vec<events::Event> = vec![events::Event {
        name: "IntentVerified",
        attrs: vec![
            ("uni.intent.id".into(), ir.intent.id.clone()),
            ("uni.contract.version".into(), ir.uni_version.clone()),
        ],
    }];
    for v in &ir.verification {
        // content-bound evidence: hash computed from the verifier's watched files
        let current_ah = registry
            .get(&v.verifier_ref)
            .and_then(|spec| uni_verify::artifact_hash(spec, &ws));
        match uni_evidence::load_valid_for_claim(&du, &v.claim_id, &cur_sha, cur_dirty, current_ah.as_deref())
        {
            Some(ev) => {
                journal.push(events::Event {
                    name: "EvidenceReused",
                    attrs: vec![
                        ("uni.claim.id".into(), v.claim_id.clone()),
                        ("uni.intent.id".into(), ir.intent.id.clone()),
                    ],
                });
                stored.push(ev)
            }
            None => need_run.push((v.claim_id.clone(), v.verifier_ref.clone(), v.inline_shell.clone())),
        }
    }
    // 2) Re-run only for missing/stale/invalid claims.
    for (claim_id, ref_r, inline) in need_run {
        let spec = uni_verify::resolve_command(&ref_r, inline.as_deref(), &registry)?;
        journal.push(events::Event {
            name: "EvidenceRun",
            attrs: vec![
                ("uni.claim.id".into(), claim_id.clone()),
                ("uni.verifier.id".into(), ref_r.clone()),
                ("uni.intent.id".into(), ir.intent.id.clone()),
            ],
        });
        let mut ev = uni_verify::run_spec(&claim_id, &spec, &ws, 300)?;
        if uni_evidence::is_stale(&ev, &cur_sha, cur_dirty) {
            ev.state = uni_evidence::EvidenceState::Stale;
        }
        uni_evidence::save_json(&uni_evidence::evidence_path(&du, &ev.claim_id), &ev)?;
        stored.push(ev);
    }
    let policy = uni_decision::load_policies(&du.join("policies"));
    let decision = uni_decision::apply_policy(uni_decision::evaluate_intent(&ir, &stored), &policy);
    let last = serde_json::json!({
        "intent": {"id": ir.intent.id, "domain": ir.intent.domain, "goal": ir.intent.goal},
        "decision": decision.decision,
        "reason": decision.reason,
        "claims": decision.claims,
    });
    journal.push(events::Event {
        name: "DecisionIssued",
        attrs: vec![
            ("uni.intent.id".into(), ir.intent.id.clone()),
            ("uni.decision.state".into(), format!("{:?}", decision.decision)),
            ("uni.assurance.level".into(), format!("A{}", match decision.decision {
                uni_decision::Decision::Accepted => 2,
                uni_decision::Decision::Rejected => 1,
                _ => 0,
            })),
        ],
    });
    events::append(&journal)?;
    uni_evidence::save_json(&du.join("decisions").join("last.json"), &last)?;
    if as_json {
        println!(
            "{}",
            serde_json::json!({
                "intent": ir.intent.id,
                "decision": decision.decision,
                "reason": decision.reason,
                "claims": decision.claims,
                "evidence": stored,
            })
        );
    } else {
        println!("\nUNI Verification — {}", ir.intent.id);
        for c in &decision.claims {
            let mark = match c.state {
                uni_evidence::EvidenceState::Valid => "PASS",
                uni_evidence::EvidenceState::Invalid => "FAIL",
                uni_evidence::EvidenceState::Stale => "STALE",
            };
            println!("{:<24} {mark}  {}", c.claim_id, c.detail);
        }
        println!("\nDecision: {:?}", decision.decision);
        println!("Reason: {}", decision.reason);
    }
    if !as_json
        && decision.decision != uni_decision::Decision::Accepted
        && decision.decision != uni_decision::Decision::Rejected
    {
        // PRD §16 error UX: propose the exact next command per missing claim.
        let mut hints = vec![];
        for c in &decision.claims {
            if c.state != uni_evidence::EvidenceState::Valid {
                hints.push(format!("uni verify {}", c.claim_id));
            }
        }
        println!("\nRun:\n  {}", hints.join("\n  "));
    }
    match decision.decision {
        uni_decision::Decision::Accepted => Ok(()),
        uni_decision::Decision::Rejected => Err(anyhow!("UNI REJECTED")),
        uni_decision::Decision::Escalated => Err(anyhow!("UNI ESCALATED")),
        _ => Err(anyhow!("UNI EVIDENCE_REQUIRED")),
    }
}

fn assurance_level_x(v: &serde_json::Value) -> u8 {
    match v["decision"].as_str() {
        Some("Accepted") => 2,
        Some("Rejected") => 1,
        _ => 0,
    }
}

fn cmd_explain(arg: Option<String>, as_json: bool) -> Result<()> {
    let path = dot_uni().join("decisions").join("last.json");
    let text = std::fs::read_to_string(&path).context("no decision yet (run uni verify first)")?;
    let v: serde_json::Value = serde_json::from_str(&text)?;
    if as_json {
        println!("{text}");
        return Ok(());
    }
    println!("Decision");
    println!("{}", v["decision"].as_str().unwrap_or("?"));

    println!("\nRequired claims");
    if let Some(claims) = v["claims"].as_array() {
        for c in claims {
            let mark = match c["state"].as_str() {
                Some("Valid") => "✓",
                Some("Stale") => "⏳",
                _ => "✗",
            };
            println!("  {mark} {:<24} {}", c["claim_id"].as_str().unwrap_or("?"), c["state"].as_str().unwrap_or("?"));
        }
    }

    let summary = v["claims"].as_array().map(|a| {
        let tested = a.iter().filter(|c| c["state"] == "Valid").count();
        format!("{}/{} verified", tested, a.len())
    }).unwrap_or_default();
    let a = assurance_level_x(&v);
    println!("\nSummary");
    println!("  Claims     {summary}");
    println!("  Assurance  A{a} ({} )", match a { 0 => "DECLARED", 1 => "ARTIFACT", 2 => "VERIFIED", 3 => "INDEPENDENTLY_VERIFIED", 4 => "ATTESTED", _ => "?" });
    println!("\n{}", v["reason"].as_str().unwrap_or(""));

    if let Some(c) = v["claims"].as_array().and_then(|a| a.iter().find(|c| c["state"] != "Valid")) {
        let claim_id = c["claim_id"].as_str().unwrap_or("");
        let ev = uni_evidence::load_json::<uni_evidence::Evidence>(
            &uni_evidence::evidence_path(&dot_uni(), claim_id));
        println!("\nCLAIM {claim_id}");
        match &ev {
            None => {
                println!("Status:\nEVIDENCE_REQUIRED\n\nRequired:\n  trusted registry verifier\n\nFound:\n  no valid evidence bound to this commit\n\nRun:\n  uni verify {claim_id}");
            }
            Some(e) => {
                println!("  status       {:?}", e.state);
                println!("  command      {}", e.command);
                println!("  exit_code    {}", e.exit_code);
                println!("  commit       {}", &e.commit_sha[..e.commit_sha.len().min(8)]);
                println!("  duration_ms  {}", e.duration_ms);
            }
        }
    }

    if let Some(f) = arg {
        let needle = f.to_lowercase();
        if !needle.is_empty() && !format!("{text}").to_lowercase().contains(&needle) {
            println!("\nNo match for '{f}' in last decision.");
        }
    }
    Ok(())
}

/// Minimal SpecKit importer (MVP): constitution.md + spec.md + plan.md -> candidate contract JSON.
/// Never authoritative: prints DIFF/REVIEW reminder; human must approve into uni/intents/.
fn cmd_import_speckit(dir: &Path, as_json: bool) -> Result<()> {
    let read_opt = |n: &str| {
        std::fs::read_to_string(dir.join(n))
            .unwrap_or_default()
            .chars()
            .take(4000)
            .collect::<String>()
    };
    let constitution = read_opt("constitution.md");
    let spec = read_opt("spec.md");
    let plan = read_opt("plan.md");
    if spec.trim().is_empty() {
        return Err(anyhow!("no spec.md in {}", dir.display()));
    }
    // Candidate extraction (deterministic heuristics, never authoritative):
    // - "FR-xxx ..." lines → one claim each
    // - "#### Scenario: ..." headings (Spec Kit format) → one claim each
    // - "- [ ]" acceptance checkboxes → one claim each
    let mut claims: Vec<(String, String)> = vec![];
    for line in spec.lines().chain(plan.lines()) {
        let mut t = line.trim();
        t = t.strip_prefix("- ").unwrap_or(t); // list marker
        t = t.strip_prefix("#### ").unwrap_or(t); // heading level 4
        let (maybe_id, text) = if t.starts_with("FR-") {
            // FR-001: description | FR-001 description
            let head: &str = t.split([':', ' ']).next().unwrap_or("");
            let desc = t.split_once(|c| c == ':' || c == ' ').map(|(_, d)| d.trim()).unwrap_or(t);
            (head.trim_end_matches(['*', ':']).to_lowercase(), desc.to_string())
        } else if let Some(b) = t.strip_prefix("**FR-") {
            let head: &str = b.split([':', '*', ' ']).next().unwrap_or("");
            let desc = b
                .split_once("**:")
                .map(|(_, d)| d.trim())
                .unwrap_or(t.trim_start_matches("**"));
            (format!("fr-{}", head.trim_matches(['*', ':'])), desc.to_string())
        } else if t.starts_with("Requirement") {
            (t.split_whitespace().next().unwrap_or("").to_lowercase(), t.to_string())
        } else if let Some(s) = t.strip_prefix("Scenario:") {
            (format!("scenario-{:02}", claims.len() + 1), s.trim().to_string())
        } else if t.starts_with("[ ]") {
            (format!("check-{:02}", claims.len() + 1), t.trim_start_matches("[ ]").trim().to_string())
        } else {
            continue;
        };
        if !maybe_id.is_empty() && !claims.iter().any(|(c, _)| *c == maybe_id) {
            claims.push((maybe_id, text.chars().take(160).collect()));
        }
    }
    let intent_id = dir.file_name().unwrap_or_default().to_string_lossy().to_string();
    if claims.is_empty() {
        claims.push((
            "intent-satisfied".into(),
            "spec acceptance criteria met".into(),
        ));
    }

    // Candidate .uni DSL + canonical IR JSON both written for human review.
    let mut dsl = format!(
        "VERSION 0.1\nDOMAIN software\nINTENT candidate-{intent_id}\nGOAL\n  Imported from Spec Kit (candidate — review required).\n"
    );
    for (id, ensure) in &claims {
        dsl.push_str(&format!("CLAIM {id} REQUIRED\n  ENSURE {}\n", ensure.replace('\n', " ")));
    }
    for (id, _) in &claims {
        dsl.push_str(&format!("VERIFY {id}\n  USING project.tests\n"));
    }
    dsl.push_str("ACCEPT WHEN\n  required_claims == VERIFIED\n  AND critical_failures == 0\n");

    let out_dsl = dot_uni().join("contracts").join(format!("candidate-{intent_id}.uni"));
    std::fs::create_dir_all(dot_uni().join("contracts"))?;
    std::fs::write(&out_dsl, &dsl)?;
    let candidate = serde_json::json!({
        "uniVersion": "0.1",
        "intent": {"id": intent_id, "domain": "software"},
        "claims": claims.iter().map(|(id, ensure)| serde_json::json!({
            "id": id, "required": true, "ensure": ensure,
        })).collect::<Vec<_>>(),
        "note": "CANDIDATE — review required. LLMs/heuristics propose, humans authorize.",
        "candidate_dsl": out_dsl.display().to_string(),
        "sources": {"constitution_chars": constitution.len(), "spec_chars": spec.len(), "plan_chars": plan.len()},
    });
    let out = serde_json::to_string_pretty(&candidate)?;
    if as_json {
        println!("{out}");
    } else {
        println!("candidate contract written: {}\n\n--- REVIEW REQUIRED — edit claims/verifiers, then approve ---\n{out}",
            out_dsl.display());
    }
    Ok(())
}
