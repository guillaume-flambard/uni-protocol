use anyhow::{anyhow, Context, Result};
mod brief;
mod bundle;
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
    Verify {
        file: PathBuf,
        /// Verifier actor identity (e.g. ci:build-12). Self-declared unless an
        /// identity adapter verifies it; enables at most A3-D, never A3.
        #[arg(long)]
        actor: Option<String>,
        /// Reserved: signed provenance (A4) has no producer yet.
        #[arg(long, default_value_t = false)]
        attest: bool,
    },
    Explain { claim_or_intent: Option<String> },
    Inspect { file: PathBuf },
    ImportSpeckit { dir: PathBuf },
    Report,
    Events,
    Lint { file: PathBuf },
    Doctor,
    #[command(subcommand)]
    Pack(PackCmd),
    /// Authorize a claim's resolution requirement to a concrete verifier
    /// (human act; AI may propose, only `uni bind` authorizes).
    Bind {
        #[arg(long)]
        claim: String,
        #[arg(long)]
        verifier: String,
        /// Human-readable resolution requirement (optional for plain bindings).
        #[arg(long, default_value = "")]
        requirement: String,
        /// Concrete test selector for `{{selector}}` template verifiers.
        #[arg(long)]
        selector: Option<String>,
    },
    /// List authorized verifier bindings.
    Bindings,
    /// Emit the deterministic work order for an implementing agent
    /// (claims + the exact evidence each one requires). Guidance, not authority.
    Brief {
        file: PathBuf,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    #[command(subcommand)]
    Bundle(BundleCmd),
}

#[derive(Subcommand)]
enum BundleCmd {
    /// Export a contract's audit surface (contract, registry, evidence, bindings, decision, events)
    Export {
        file: PathBuf,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Verify a bundle offline: record integrity + context cross-checks. Never touches the live cache.
    Verify { file: PathBuf },
}

#[derive(Subcommand)]
enum PackCmd {
    /// List domain packs found in ./packs (and .uni/packs when installed).
    List,
    /// Materialize a pack template into uni/intents/<name>.uni for editing.
    Template { pack: String, name: String },
}

fn dot_uni() -> PathBuf {
    PathBuf::from(".uni")
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Init => cmd_init(),
        Cmd::Compile { file } => cmd_compile(&file, cli.json),
        Cmd::Verify { file, actor, attest } => cmd_verify(&file, cli.json, actor.as_deref(), attest),
        Cmd::Explain { claim_or_intent } => cmd_explain(claim_or_intent, cli.json),
        Cmd::Inspect { file } => cmd_inspect(&file, cli.json),
        Cmd::ImportSpeckit { dir } => cmd_import_speckit(&dir, cli.json),
        Cmd::Report => cmd_report(cli.json),
        Cmd::Events => cmd_events(cli.json, 50),
        Cmd::Lint { file } => cmd_lint(&file, cli.json),
        Cmd::Doctor => cmd_doctor(cli.json),
        Cmd::Pack(sub) => cmd_pack(sub, cli.json),
        Cmd::Bind { claim, verifier, requirement, selector } => {
            cmd_bind(&claim, &verifier, &requirement, selector.as_deref(), cli.json)
        }
        Cmd::Bindings => cmd_bindings(cli.json),
        Cmd::Bundle(sub) => cmd_bundle(sub, cli.json),
        Cmd::Brief { file, out } => cmd_brief(&file, out.as_deref(), cli.json),
    }
}

/// Packs are directory bundles `packs/<name>/pack.toml` + `templates/*.uni`.
/// Resolution order: ./packs, then .uni/packs (installed packs).
fn discover_packs() -> Vec<(String, toml::Value)> {
    let mut out = vec![];
    for root in ["packs", ".uni/packs"] {
        let rd = match std::fs::read_dir(root) {
            Ok(rd) => rd,
            Err(_) => continue,
        };
        for e in rd.flatten() {
            let manifest = e.path().join("pack.toml");
            if let Ok(text) = std::fs::read_to_string(&manifest) {
                if let Ok(v) = text.parse::<toml::Value>() {
                    let name = e.path().file_name().unwrap_or_default().to_string_lossy().to_string();
                    out.push((name, v));
                }
            }
        }
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

fn cmd_pack(sub: PackCmd, as_json: bool) -> Result<()> {
    match sub {
        PackCmd::List => {
            let packs = discover_packs();
            if as_json {
                let v: Vec<serde_json::Value> = packs
                    .iter()
                    .map(|(dir, doc)| {
                        let p = doc.get("pack");
                        serde_json::json!({
                            "dir": dir,
                            "name": p.and_then(|x| x.get("name")).and_then(|x| x.as_str()).unwrap_or(dir),
                            "version": p.and_then(|x| x.get("version")).and_then(|x| x.as_str()).unwrap_or("?"),
                            "description": p.and_then(|x| x.get("description")).and_then(|x| x.as_str()).unwrap_or(""),
                            "templates": doc.get("template").and_then(|t| t.as_array()).map(|a| a.iter().filter_map(|t| Some(serde_json::json!({
                                "name": t.get("name")?.as_str()?,
                                "description": t.get("description").and_then(|d| d.as_str()).unwrap_or(""),
                            }))).collect::<Vec<_>>()).unwrap_or_default(),
                        })
                    })
                    .collect();
                println!("{}", serde_json::json!({ "packs": v }));
            } else if packs.is_empty() {
                println!("no packs found (looked in ./packs and .uni/packs)");
            } else {
                for (dir, doc) in &packs {
                    let p = doc.get("pack");
                    println!("{} {} - {}",
                        p.and_then(|x| x.get("name")).and_then(|x| x.as_str()).unwrap_or(dir),
                        p.and_then(|x| x.get("version")).and_then(|x| x.as_str()).unwrap_or("?"),
                        p.and_then(|x| x.get("description")).and_then(|x| x.as_str()).unwrap_or(""));
                    if let Some(t) = doc.get("template").and_then(|t| t.as_array()) {
                        for tt in t {
                            println!("  {:<24} {}",
                                tt.get("name").and_then(|n| n.as_str()).unwrap_or("?"),
                                tt.get("description").and_then(|d| d.as_str()).unwrap_or(""));
                        }
                    }
                }
            }
        }
        PackCmd::Template { pack, name } => {
            let rel = format!("packs/{pack}/templates/{name}.uni");
            let src = [std::path::PathBuf::from(&rel), dot_uni().join("packs").join(&pack).join("templates").join(format!("{name}.uni"))]
                .into_iter()
                .find(|p| p.exists())
                .ok_or_else(|| anyhow!("template '{pack}/{name}' not found (uni pack list shows available templates)"))?;
            std::fs::create_dir_all("uni/intents")?;
            let dst = PathBuf::from("uni/intents").join(format!("{name}.uni"));
            std::fs::copy(&src, &dst)?;
            if as_json {
                println!("{}", serde_json::json!({ "written": dst.display().to_string(), "pack": pack, "template": name }));
            } else {
                println!("wrote {}\nedit claims + verifier refs, then: uni lint {}", dst.display(), dst.display());
            }
        }
    }
    Ok(())
}

fn cmd_brief(file: &Path, out: Option<&Path>, as_json: bool) -> Result<()> {
    let (ir, _) = load_contract(file)?;
    let du = dot_uni();
    let registry = uni_verify::load_registry(&du);
    let registry_text = std::fs::read_to_string(du.join("config.toml")).unwrap_or_default();
    let (claims, problems) = brief::build(&ir, &registry);
    let body = if as_json {
        serde_json::to_string_pretty(&brief::to_json(&ir, &claims, &problems, &brief::registry_hash(&registry_text))?)?
    } else {
        brief::to_markdown(&ir, &claims, &problems)
    };
    match out {
        Some(path) => {
            if let Some(p) = path.parent() {
                std::fs::create_dir_all(p)?;
            }
            let tmp = path.with_extension(format!("tmp-{}", std::process::id()));
            std::fs::write(&tmp, format!("{body}\n"))?;
            std::fs::rename(&tmp, path)?;
            if !as_json {
                eprintln!("brief written: {}", path.display());
            }
        }
        None => println!("{body}"),
    }
    Ok(())
}

fn cmd_bundle(sub: BundleCmd, as_json: bool) -> Result<()> {
    match sub {
        BundleCmd::Export { file, out } => {
            let du = dot_uni();
            let out = out.unwrap_or_else(|| {
                PathBuf::from(format!("uni-bundle-{}.jsonl", du.display().to_string().replace(['/', '.'], "-")))
            });
            let header = bundle::export(&du, &file, &out)?;
            if as_json {
                println!("{}", serde_json::json!({
                    "bundle": out.display().to_string(),
                    "version": header.version,
                    "intent": header.intent,
                    "records": header.records,
                    "registry_hash": header.registry_hash,
                    "contract_hash": header.contract_hash,
                }));
            } else {
                println!("bundle written: {}", out.display());
                println!("intent:  {}", header.intent);
                println!("records: {}", header.records);
                println!("tool:    {}", header.tool);
            }
        }
        BundleCmd::Verify { file } => {
            let (header, report) = bundle::verify(&file)?;
            let ok = report.integrity_errors.is_empty() && report.cross_check_errors.is_empty();
            if as_json {
                println!("{}", serde_json::json!({
                    "bundle": file.display().to_string(),
                    "intent": header.intent,
                    "records": report.records,
                    "claims_total": report.claims_total,
                    "claims_covered": report.claims_covered,
                    "integrity_errors": report.integrity_errors,
                    "cross_check_errors": report.cross_check_errors,
                    "ok": ok,
                }));
            } else {
                println!("bundle: {}", file.display());
                println!("intent: {}  records: {}", header.intent, report.records);
                println!("claims covered by evidence: {}/{}", report.claims_covered, report.claims_total);
                for e in &report.integrity_errors {
                    println!("  INTEGRITY {e}");
                }
                for e in &report.cross_check_errors {
                    println!("  CROSS-CHECK {e}");
                }
                println!("\nOK: {ok}");
            }
            if !ok {
                return Err(anyhow!("uni bundle verify: bundle is not internally consistent"));
            }
        }
    }
    Ok(())
}

/// Human authorization act for VerifierBindings, journaled as such.
fn cmd_bind(
    claim: &str,
    verifier: &str,
    requirement: &str,
    selector: Option<&str>,
    as_json: bool,
) -> Result<()> {
    let du = dot_uni();
    let by = uni_evidence::Actor::local().id;
    let b = uni_evidence::binding::authorize(&du, claim, verifier, requirement, selector, &by)?;
    events::append(&[events::Event {
        name: "BindingAuthorized",
        attrs: vec![
            ("uni.claim.id".into(), claim.to_string()),
            ("uni.verifier.id".into(), verifier.to_string()),
            ("uni.binding.hash".into(), b.binding_hash.clone()),
            ("uni.binding.selector".into(), b.selector.clone().unwrap_or_default()),
            ("uni.binding.by".into(), by),
        ],
    }])?;
    if as_json {
        println!("{}", serde_json::to_string_pretty(&b)?);
    } else {
        println!("authorized: claim '{claim}' -> verifier '{verifier}'");
        if !requirement.is_empty() {
            println!("requirement: {requirement}");
        }
        if let Some(sel) = &b.selector {
            println!("selector:    {sel}");
        }
        println!("binding:     {}", b.binding_hash);
    }
    Ok(())
}

fn cmd_bindings(as_json: bool) -> Result<()> {
    let du = dot_uni();
    let dir = uni_evidence::binding::bindings_dir(&du);
    let mut out = vec![];
    if let Ok(rd) = std::fs::read_dir(&dir) {
        for e in rd.flatten() {
            if e.path().extension().map(|x| x == "json").unwrap_or(false) {
                if let Some(b) =
                    uni_evidence::binding::load_binding(&du, e.path().file_stem().and_then(|s| s.to_str()).unwrap_or(""))
                {
                    out.push(b);
                }
            }
        }
    }
    out.sort_by(|a, b| a.claim_id.cmp(&b.claim_id));
    if as_json {
        println!("{}", serde_json::to_string_pretty(&out)?);
    } else if out.is_empty() {
        println!("no authorized bindings (.uni/bindings/ is empty)");
    } else {
        for b in &out {
            let sel = b.selector.as_deref().unwrap_or("-");
            println!("{:<24} -> {:<24} selector {:<20} [{}] by {} at {}",
                b.claim_id, b.verifier_ref, sel, b.binding_hash, b.authorized_by, b.authorized_at);
        }
    }
    Ok(())
}

/// Workspace health check, read-only: git binding, registry, policies,
/// evidence/journal writability. Exit 0 when everything is healthy.
fn cmd_doctor(as_json: bool) -> Result<()> {
    let du = dot_uni();
    let ws = std::env::current_dir()?;
    let mut checks: Vec<(String, bool, String)> = vec![];
    let ok_uni = du.exists();
    checks.push((
        ".uni present".into(),
        ok_uni,
        if ok_uni { String::new() } else { "run `uni init`".into() },
    ));
    let (sha, dirty) = uni_evidence::git_info(&ws);
    let git_ok = sha != "no-git" && !sha.is_empty();
    checks.push((
        "git binding".into(),
        git_ok,
        if git_ok {
            format!("{} dirty={}", &sha[..sha.len().min(8)], dirty)
        } else {
            "no repository".into()
        },
    ));
    let registry = uni_verify::load_registry(&du);
    checks.push(("registry".into(), true, format!("{} verifier(s)", registry.len())));
    let pol = uni_decision::load_policies(&du.join("policies"));
    checks.push((
        "policies".into(),
        true,
        format!(
            "reject_on_invalid={} escalate_on_stale={} escalate_on_missing={} min_ratio={:.2}",
            pol.reject_on_invalid, pol.escalate_on_stale, pol.escalate_on_missing, pol.min_verified_ratio
        ),
    ));
    let ev_ok = std::fs::create_dir_all(du.join("evidence")).is_ok();
    checks.push(("evidence dir writable".into(), ev_ok, String::new()));
    let jr_ok = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(events::journal_path())
        .is_ok();
    checks.push(("journal writable".into(), jr_ok, String::new()));

    let failed = checks.iter().filter(|(_, ok, _)| !ok).count();
    if as_json {
        println!(
            "{}",
            serde_json::json!({
                "healthy": failed == 0,
                "checks": checks.iter().map(|(name, ok, detail)| serde_json::json!({
                    "check": name, "ok": ok, "detail": detail,
                })).collect::<Vec<_>>(),
            })
        );
    } else {
        println!("uni doctor");
        for (name, ok, detail) in &checks {
            println!("  {} {:<22} {}", if *ok { "OK  " } else { "FAIL" }, name, detail);
        }
        println!("\nhealthy: {}", failed == 0);
    }
    if failed > 0 {
        return Err(anyhow!("uni doctor: {failed} check(s) failed"));
    }
    Ok(())
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
    // v0.4: a selector-template verifier needs an authorized selector binding,
    // and the specific message beats the generic unbound-requirement warning.
    for v in &ir.verification {
        let Some(spec) = registry.get(&v.verifier_ref) else { continue };
        if !uni_verify::is_selector_template(spec) {
            continue;
        }
        let ok = match uni_evidence::binding::load_binding(&dot_uni(), &v.claim_id) {
            Some(b) => {
                b.verifier_ref == v.verifier_ref
                    && b.requirement == v.requirement.clone().unwrap_or_default()
                    && b.selector.as_deref().map(|s| !s.trim().is_empty()).unwrap_or(false)
            }
            None => false,
        };
        if !ok {
            findings.push((1, "selector-template".into(), format!(
                "claim '{}' uses selector template '{}' without an authorized selector binding (run: uni bind --claim {} --verifier {} --selector <test-name>)",
                v.claim_id, v.verifier_ref, v.claim_id, v.verifier_ref
            )));
        }
    }
    let selector_flagged: Vec<String> = findings
        .iter()
        .filter(|(_, kind, _)| kind == "selector-template")
        .filter_map(|(_, _, msg)| {
            // the claim id is the token after the first quote
            msg.split('\'').nth(1).map(|s| s.to_string())
        })
        .collect();
    for v in ir.verification.iter().filter(|v| v.requirement.is_some()) {
        if selector_flagged.contains(&v.claim_id) {
            continue; // the specific selector message already says what to do
        }
        let req = v.requirement.as_deref().unwrap_or("");
        match uni_evidence::binding::load_binding(&dot_uni(), &v.claim_id) {
            Some(b) if b.verifier_ref == v.verifier_ref && b.requirement == req => {}
            _ => findings.push((1, "unbound-requirement".into(), format!(
                "claim '{}' has a REQUIRE but no matching authorized binding (run: uni bind --claim {} --verifier {})",
                v.claim_id, v.claim_id, v.verifier_ref
            ))),
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
        // Persisted at verify time; legacy files fall back to the decision-only base.
        "assurance": v.get("assurance").cloned().unwrap_or_else(|| {
            let a = uni_decision::assurance_of_json(&v["decision"]);
            serde_json::json!(format!("A{a}"))
        }),
        "independent_actor": v.get("independent_actor").cloned().unwrap_or(serde_json::Value::Bool(false)),
        "identity_assurance": v.get("identity_assurance").cloned().unwrap_or(serde_json::json!("SELF-DECLARED")),
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
    println!("Assurance   {} (independent actor: {}, identity: {})",
        r["assurance"].as_str().unwrap_or("A0"),
        if r["independent_actor"].as_bool().unwrap_or(false) { "YES" } else { "NO" },
        r["identity_assurance"].as_str().unwrap_or("SELF-DECLARED"));
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

/// v0.2 authorization gate for resolution requirements. Returns the binding
/// hash the evidence must carry, or fails hard: a requirement without a
/// matching authorized binding never executes. `None` requirement = no gate.
/// Authorized resolution of a claim's verification (v0.4).
#[derive(Default)]
struct Resolution {
    binding_hash: Option<String>,
    selector: Option<String>,
}

/// Gate: a verification runs only under an authorized binding when it either
/// declares a resolution REQUIRE (v0.2) or uses a `{{selector}}` template
/// (v0.4). A template plus an authorization is what removes the study's
/// evidence-name coupling: the worker names the test, the human authorizes it.
fn require_binding(
    du: &Path,
    claim_id: &str,
    verifier_ref: &str,
    requirement: Option<&str>,
    is_template: bool,
) -> Result<Resolution> {
    let req = requirement.unwrap_or("");
    if !is_template && requirement.is_none() {
        return Ok(Resolution::default());
    }
    match uni_evidence::binding::load_binding(du, claim_id) {
        Some(b) if b.verifier_ref == verifier_ref && b.requirement == req => {
            if is_template && b.selector.as_deref().unwrap_or("").trim().is_empty() {
                return Err(anyhow!(
                    "claim '{claim_id}' uses a selector-template verifier but its binding has no selector; authorize with: uni bind --claim {claim_id} --verifier {verifier_ref} --selector <test-name>"
                ));
            }
            Ok(Resolution {
                binding_hash: Some(b.binding_hash),
                selector: b.selector,
            })
        }
        Some(b) => Err(anyhow!(
            "claim '{claim_id}' was rebound (bound: requirement '{}' verifier '{}' selector {:?}; contract expects requirement '{req}' on verifier '{verifier_ref}'); re-authorize with: uni bind --claim {claim_id} --verifier {verifier_ref} --selector <test-name>",
            b.requirement,
            b.verifier_ref,
            b.selector,
        )),
        None => Err(anyhow!(
            "claim '{claim_id}' needs an authorized binding (requirement '{req}', template {is_template}) but none exists; authorize with: uni bind --claim {claim_id} --verifier {verifier_ref} --selector <test-name>"
        )),
    }
}

fn cmd_verify(file: &Path, as_json: bool, actor_flag: Option<&str>, attest: bool) -> Result<()> {
    if attest {
        return Err(anyhow!(
            "signed provenance (A4) is reserved: no signer is configured in v0.2 (see docs/decisions.md)"
        ));
    }
    let (ir, _) = load_contract(file)?;
    let ws = std::env::current_dir()?;
    let du = dot_uni();
    // B3 actor model: the executor is whoever runs this command (local,
    // self-declared, capped at A2); the verifier actor defaults to the same
    // identity unless --actor names a distinct one (still self-declared:
    // a flag is a declaration, not a proof; max A3-D, never A3).
    let executor = uni_evidence::Actor::local();
    let actor = match actor_flag {
        Some(id) => uni_evidence::Actor::declared(id),
        None => executor.clone(),
    };
    let external_scheme = actor_flag.map(|id| {
        id.split_once("://").map(|(s, _)| s).unwrap_or("")
    });
    // v0.2 has no identity adapters: a scheme prefix is recorded but stays
    // self-declared, and the journal says so explicitly.
    let identity_unverified_warning =
        matches!(external_scheme, Some("spiffe") | Some("entra") | Some("oidc"));
    let (cur_sha, cur_dirty) = uni_evidence::git_info(&ws);
    let registry = uni_verify::load_registry(&du);
    // Verification Context, computed once per run: any drift on these
    // dimensions invalidates stored evidence (B1); policy drift instead
    // forces a decision recompute, which every verify does anyway.
    let contract_text = std::fs::read_to_string(file).unwrap_or_default();
    let contract_hash = uni_evidence::sha256_hex(contract_text.as_bytes());
    let registry_text = std::fs::read_to_string(du.join("config.toml")).unwrap_or_default();
    let registry_hash = uni_evidence::sha256_hex(registry_text.as_bytes());
    let platform = uni_evidence::platform();
    // B2: trust-boundary diff against the last acknowledged registry snapshot.
    // A changed registry never silently reuses old evidence (B1 already stales
    // it); here we name what changed and flag it for CI/human review.
    let prev_registry_hash = std::fs::read_to_string(du.join(".registry.hash"))
        .unwrap_or_default()
        .trim()
        .to_string();
    let prev_registry_text =
        std::fs::read_to_string(du.join(".registry.snapshot.toml")).unwrap_or_default();
    let trust_boundary_changed =
        !prev_registry_hash.is_empty() && prev_registry_hash != registry_hash;
    let tb_diff = if trust_boundary_changed {
        uni_verify::registry_diff(&prev_registry_text, &registry_text)
    } else {
        uni_verify::RegistryDiff::default()
    };
    if trust_boundary_changed {
        if !as_json {
            println!("REGISTRY_CHANGED");
            println!("Previous: sha256:{}", &prev_registry_hash[..12.min(prev_registry_hash.len())]);
            println!("Current:  sha256:{}", &registry_hash[..12]);
            println!(
                "Existing evidence: STALE\nAuthorization: REQUIRED ({} added, {} removed, {} changed)",
                tb_diff.added.len(),
                tb_diff.removed.len(),
                tb_diff.changed.len()
            );
            for name in tb_diff.added.iter().chain(tb_diff.removed.iter()).chain(tb_diff.changed.iter()).take(10) {
                println!("  - {name}");
            }
        }
    }
    // 1) Try persisted evidence first (cheap, content-addressed).
    let mut stored = vec![];
    let mut need_run = vec![];
    let mut stale_ids: Vec<String> = vec![];
    let mut journal: Vec<events::Event> = vec![events::Event {
        name: "IntentVerified",
        attrs: vec![
            ("uni.intent.id".into(), ir.intent.id.clone()),
            ("uni.contract.version".into(), ir.uni_version.clone()),
        ],
    }];
    if identity_unverified_warning {
        journal.push(events::Event {
            name: "IdentityUnverified",
            attrs: vec![
                ("uni.actor.id".into(), actor.id.clone()),
                ("uni.actor.source".into(), actor.source.clone()),
                ("uni.actor.assurance".into(), actor.assurance.clone()),
            ],
        });
    }
    if trust_boundary_changed {
        journal.push(events::Event {
            name: "RegistryChanged",
            attrs: vec![
                ("uni.registry.previous".into(), prev_registry_hash[..8.min(prev_registry_hash.len())].into()),
                ("uni.registry.current".into(), registry_hash[..8].into()),
                ("uni.registry.added".into(), tb_diff.added.join(",")),
                ("uni.registry.removed".into(), tb_diff.removed.join(",")),
                ("uni.registry.changed".into(), tb_diff.changed.join(",")),
            ],
        });
    }
    for v in &ir.verification {
        // v0.2 authorization gate: a verification carrying a resolution
        // requirement executes ONLY under a matching authorized binding.
        // No binding, or a binding for different text/verifier -> hard error,
        // never a silent run. AI may propose; only `uni bind` authorizes.
        let base_spec = registry.get(&v.verifier_ref).cloned();
        let is_template = base_spec
            .as_ref()
            .map(uni_verify::is_selector_template)
            .unwrap_or(false);
        let resolution = require_binding(
            &du,
            &v.claim_id,
            &v.verifier_ref,
            v.requirement.as_deref(),
            is_template,
        )?;
        // Resolve the selector into the command before anything is hashed or
        // fingerprinted: two selectors on the same key are different proofs.
        let resolved_spec = match &base_spec {
            Some(spec) => Some(uni_verify::with_selector(spec, resolution.selector.as_deref())?),
            None => None,
        };
        let binding_hash = resolution.binding_hash.clone();
        // content-bound evidence: hash computed from the verifier's watched files
        let current_ah = resolved_spec
            .as_ref()
            .and_then(|spec| uni_verify::artifact_hash(spec, &ws));
        let fingerprint = match &resolved_spec {
            Some(spec) => uni_verify::spec_fingerprint(&v.verifier_ref, spec, &actor.id),
            None => format!("inline:{}:{}", &v.verifier_ref, actor.id),
        };
        let ctx = uni_evidence::EvidenceContext {
            fingerprint,
            commit_sha: cur_sha.clone(),
            workspace_dirty: cur_dirty,
            artifact_hash: current_ah,
            registry_hash: registry_hash.clone(),
            contract_hash: contract_hash.clone(),
            platform: platform.clone(),
            binding_hash: binding_hash.clone(),
        };
        match uni_evidence::load_valid_for_claim(&du, &v.claim_id, &ctx) {
            uni_evidence::CacheOutcome::Hit(mut ev) => {
                // The proof is reused, but the decision context is now:
                // independence is evaluated against the CURRENT executor.
                ev.executor = executor.clone();
                journal.push(events::Event {
                    name: "EvidenceReused",
                    attrs: vec![
                        ("uni.claim.id".into(), v.claim_id.clone()),
                        ("uni.intent.id".into(), ir.intent.id.clone()),
                    ],
                });
                stored.push(ev)
            }
            uni_evidence::CacheOutcome::Stale => {
                journal.push(events::Event {
                    name: "EvidenceStale",
                    attrs: vec![
                        ("uni.claim.id".into(), v.claim_id.clone()),
                        ("uni.intent.id".into(), ir.intent.id.clone()),
                    ],
                });
                stale_ids.push(v.claim_id.clone());
                need_run.push((v.claim_id.clone(), v.verifier_ref.clone(), v.inline_shell.clone(), binding_hash.clone(), resolved_spec.clone()));
            }
            uni_evidence::CacheOutcome::Miss => {
                need_run.push((v.claim_id.clone(), v.verifier_ref.clone(), v.inline_shell.clone(), binding_hash.clone(), resolved_spec.clone()))
            }
        }
    }
    // Policy source selection: OPA bundle when both rego + opa binary exist, else TOML stack.
    // Resolved BEFORE running verifiers so fresh evidence records the policy
    // it was gathered under (audit dimension; drift forces recompute, not re-run).
    let opa_bundle = du.join("policies/opa.rego");
    let policies_dir = du.join("policies");
    let opa_available = opa_bundle.exists()
        && std::process::Command::new("opa")
            .arg("version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
    let provider: Box<dyn uni_decision::PolicyProvider> = if opa_available {
        Box::new(uni_decision::OpaPolicy { bundle: opa_bundle })
    } else {
        Box::new(uni_decision::TomlPolicy { dir: &policies_dir })
    };
    let policy = provider.resolve();
    let policy_hash =
        uni_evidence::sha256_hex(serde_json::to_string(&policy).unwrap_or_default().as_bytes());
    // 2) Re-run only for missing/stale/invalid claims (unlocked: reruns are
    // idempotent and deterministic, so concurrent runs only duplicate work).
    let mut fresh: Vec<(uni_evidence::Evidence, String)> = vec![];
    for (claim_id, ref_r, inline, binding, resolved) in need_run {
        // A resolved template comes back pre-substituted; plain verifiers are
        // still resolved through the trusted registry.
        let spec = match resolved {
            Some(spec) => spec,
            None => uni_verify::resolve_command(&ref_r, inline.as_deref(), &registry)?,
        };
        journal.push(events::Event {
            name: "EvidenceRun",
            attrs: vec![
                ("uni.claim.id".into(), claim_id.clone()),
                ("uni.verifier.id".into(), ref_r.clone()),
                ("uni.intent.id".into(), ir.intent.id.clone()),
            ],
        });
        let fingerprint = uni_verify::spec_fingerprint(&ref_r, &spec, &actor.id);
        let mut ev = uni_verify::run_spec(&claim_id, &ref_r, &spec, &ws, spec.timeout, &actor, &executor)?;
        ev.registry_hash = registry_hash.clone();
        ev.contract_hash = contract_hash.clone();
        ev.platform = platform.clone();
        ev.policy_hash = policy_hash.clone();
        ev.binding_hash = binding.unwrap_or_default();
        if uni_evidence::is_stale(&ev, &cur_sha, cur_dirty) {
            ev.state = uni_evidence::EvidenceState::Stale;
        }
        fresh.push((ev, fingerprint));
    }
    for (ev, _) in &fresh {
        stored.push(ev.clone());
    }
    // Policy already resolved above (recorded on fresh evidence for audit).
    let mut decision = uni_decision::apply_policy(uni_decision::evaluate_intent(&ir, &stored), &policy);
    // Stale-but-unreprovable escalation: a claim whose previous proof drifted
    // out of context and could NOT be re-proven needs a human, not a retry.
    // (When the re-run succeeds the claim is Valid and this never fires.)
    if policy.escalate_on_stale && decision.decision != uni_decision::Decision::Accepted {
        let valid: std::collections::HashSet<&str> = decision
            .claims
            .iter()
            .filter(|c| c.state == uni_evidence::EvidenceState::Valid)
            .map(|c| c.claim_id.as_str())
            .collect();
        if stale_ids.iter().any(|id| !valid.contains(id.as_str())) {
            decision.decision = uni_decision::Decision::Escalated;
            decision.reason = format!("policy escalate_on_stale: stale proof could not be renewed: {}", decision.reason);
        }
    }
    let assurance = uni_decision::assurance_for(&decision.decision, &stored);
    let independent = uni_decision::independence(&stored) == uni_decision::Independence::Independent;
    let identity = uni_decision::identity_assurance(&stored);
    let last = serde_json::json!({
        "intent": {"id": ir.intent.id, "domain": ir.intent.domain, "goal": ir.intent.goal},
        "decision": decision.decision,
        "reason": decision.reason,
        "claims": decision.claims,
        "assurance": assurance,
        "independent_actor": independent,
        "identity_assurance": identity,
    });
    journal.push(events::Event {
        name: "DecisionIssued",
        attrs: vec![
            ("uni.intent.id".into(), ir.intent.id.clone()),
            ("uni.decision.state".into(), format!("{:?}", decision.decision)),
            ("uni.assurance.level".into(), assurance.to_string()),
            ("uni.actor.independent".into(), independent.to_string()),
            ("uni.actor.identity_assurance".into(), identity.to_string()),
        ],
    });
    // 3) Persist phase, serialized: evidence files + journal + last.json are
    // written atomically under an exclusive lock so concurrent verifies can
    // never interleave or truncate each other's state.
    {
        let _lock = uni_evidence::acquire_lock(&du)?;
        for (ev, fingerprint) in &fresh {
            uni_evidence::save_json(
                &uni_evidence::evidence_path(&du, &ev.claim_id, fingerprint),
                ev,
            )?;
        }
        events::append(&journal)?;
        uni_evidence::save_json(&du.join("decisions").join("last.json"), &last)?;
        // Acknowledge the current registry as the new trust-boundary baseline
        // (only after a completed verify: a failed run leaves the flag armed).
        std::fs::write(du.join(".registry.hash"), &registry_hash)?;
        std::fs::write(du.join(".registry.snapshot.toml"), &registry_text)?;
    }
    if as_json {
        println!(
            "{}",
            serde_json::json!({
                "intent": ir.intent.id,
                "decision": decision.decision,
                "reason": decision.reason,
                "claims": decision.claims,
                "evidence": stored,
                "trust_boundary_changed": trust_boundary_changed,
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
    // Persisted assurance wins; legacy files fall back to the decision-only base.
    let assurance = v["assurance"].as_str().map(str::to_string).unwrap_or_else(|| {
        let a = uni_decision::assurance_of_json(&v["decision"]);
        format!("A{a}")
    });
    let independent = v["independent_actor"].as_bool().unwrap_or(false);
    let identity = v["identity_assurance"].as_str().unwrap_or("SELF-DECLARED");
    println!("\nSummary");
    println!("  Claims     {summary}");
    println!("  Assurance  {assurance} (independent actor: {}, identity: {})",
        if independent { "YES" } else { "NO" }, identity);
    println!("\n{}", v["reason"].as_str().unwrap_or(""));

    if let Some(c) = v["claims"].as_array().and_then(|a| a.iter().find(|c| c["state"] != "Valid")) {
        let claim_id = c["claim_id"].as_str().unwrap_or("");
        let ev = uni_evidence::latest_for_claim(&dot_uni(), claim_id);
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
    let tasks = read_opt("tasks.md");
    if spec.trim().is_empty() {
        return Err(anyhow!("no spec.md in {}", dir.display()));
    }
    // Candidate extraction (deterministic heuristics, never authoritative):
    // - "FR-xxx ..." lines → one claim each
    // - "#### Scenario: ..." headings (Spec Kit format) → one claim each
    // - "- [ ]" acceptance checkboxes → one claim each
    let mut claims: Vec<(String, String)> = vec![];
    // Strip markdown list markers anywhere they appear: bullets, numeric
    // ("1. " / "1) "), and heading hashes. Requirements nested in numbered
    // lists are common in real specs and used to be silently lost.
    let strip_markers = |line: &str| -> String {
        let mut t = line.trim();
        let digits = t.chars().take_while(|c| c.is_ascii_digit()).count();
        if digits > 0 {
            let rest = &t[digits..];
            if let Some(r) = rest.strip_prefix(". ").or_else(|| rest.strip_prefix(") ")) {
                t = r.trim_start();
            }
        }
        for p in ["- ", "* ", "+ "] {
            if let Some(r) = t.strip_prefix(p) {
                t = r.trim_start();
            }
        }
        t = t.strip_prefix("#### ").unwrap_or(t);
        t = t.strip_prefix("### ").unwrap_or(t);
        t.to_string()
    };
    for line in spec.lines().chain(plan.lines()).chain(tasks.lines()) {
        let owned = strip_markers(line);
        let t: &str = owned.as_str();
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
        } else if t.starts_with("[ ]") || t.starts_with("[x]") {
            (format!("check-{:02}", claims.len() + 1), t[4..].trim().to_string())
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

    // Ship the work order next to the candidate: the study showed that leaving
    // the registry->test-name hop implicit causes correct work to be rejected.
    // Unresolvable verifiers surface here, before anyone starts implementing.
    let out_brief = dot_uni()
        .join("contracts")
        .join(format!("candidate-{intent_id}.brief.md"));
    let (brief_claims, brief_problems) = match uni_parser::parse(&dsl)
        .and_then(|ast| uni_ir::compile(&ast))
    {
        Ok(ir) => {
            let registry = uni_verify::load_registry(&dot_uni());
            let (claims, problems) = brief::build(&ir, &registry);
            std::fs::write(&out_brief, format!("{}\n", brief::to_markdown(&ir, &claims, &problems)))?;
            (claims.len(), problems)
        }
        Err(e) => (0, vec![format!("candidate did not compile: {e}")]),
    };
    let candidate = serde_json::json!({
        "uniVersion": "0.1",
        "intent": {"id": intent_id, "domain": "software"},
        "claims": claims.iter().map(|(id, ensure)| serde_json::json!({
            "id": id, "required": true, "ensure": ensure,
        })).collect::<Vec<_>>(),
        "note": "CANDIDATE — review required. LLMs/heuristics propose, humans authorize.",
        "candidate_dsl": out_dsl.display().to_string(),
        "candidate_brief": out_brief.display().to_string(),
        "candidate_claims": brief_claims,
        "candidate_problems": brief_problems,
        "sources": {"constitution_chars": constitution.len(), "spec_chars": spec.len(), "plan_chars": plan.len(), "tasks_chars": tasks.len()},
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
