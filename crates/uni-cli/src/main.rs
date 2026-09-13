use anyhow::{anyhow, Context, Result};
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
    }
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
    let evidences = uni_verify::assure_contract(&ir, &du, &ws)?;
    // persist evidence + stale check against current git
    let (cur_sha, cur_dirty) = uni_evidence::git_info(&ws);
    let mut stored = vec![];
    for mut ev in evidences {
        if uni_evidence::is_stale(&ev, &cur_sha, cur_dirty) {
            ev.state = uni_evidence::EvidenceState::Stale;
        }
        uni_evidence::save_json(&uni_evidence::evidence_path(&du, &ev.claim_id), &ev)?;
        stored.push(ev);
    }
    let decision = uni_decision::evaluate_intent(&ir, &stored);
    uni_evidence::save_json(&du.join("decisions").join("last.json"), &decision)?;
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
    match decision.decision {
        uni_decision::Decision::Accepted => Ok(()),
        uni_decision::Decision::Rejected => Err(anyhow!("UNI REJECTED")),
        _ => Err(anyhow!("UNI EVIDENCE_REQUIRED")),
    }
}

fn cmd_explain(arg: Option<String>, as_json: bool) -> Result<()> {
    let path = dot_uni().join("decisions").join("last.json");
    let text = std::fs::read_to_string(&path).context("no decision yet (run uni verify first)")?;
    if as_json || arg.is_none() {
        println!("{text}");
        return Ok(());
    }
    let _filter = arg.unwrap();
    println!("{text}");
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
    // Heuristic extraction: lines starting with FR-/Requirement -> candidate claims.
    let mut claims = vec![];
    for line in spec.lines().chain(plan.lines()) {
        let t = line.trim();
        if t.starts_with("FR-") || t.starts_with("- [ ]") || t.starts_with("Requirement") {
            let id = t
                .split_whitespace()
                .next()
                .unwrap_or("claim")
                .trim_matches(['-', '[', ']', ' '])
                .to_lowercase();
            let id = id.chars().take(40).collect::<String>();
            if !id.is_empty() {
                claims.push(serde_json::json!({
                    "id": id,
                    "required": true,
                    "ensure": t.chars().take(200).collect::<String>(),
                }));
            }
        }
    }
    if claims.is_empty() {
        claims.push(
            serde_json::json!({"id": "intent-satisfied", "required": true, "ensure": "spec acceptance criteria met"}),
        );
    }
    let candidate = serde_json::json!({
        "uniVersion": "0.1",
        "intent": {"id": dir.file_name().unwrap_or_default().to_string_lossy(), "domain": "software"},
        "claims": claims,
        "note": "CANDIDATE — review required. LLMs propose, humans authorize.",
        "sources": {"constitution_chars": constitution.len(), "spec_chars": spec.len(), "plan_chars": plan.len()},
    });
    let out = serde_json::to_string_pretty(&candidate)?;
    if as_json {
        println!("{out}");
    } else {
        println!("{out}\n\n--- REVIEW REQUIRED: save approved contract to uni/intents/ ---");
    }
    Ok(())
}
