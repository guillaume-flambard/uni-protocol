use crate::dot_uni;
use anyhow::{anyhow, Result};
use clap::Subcommand;
use std::path::{Path, PathBuf};

#[derive(Subcommand)]
pub enum BundleCmd {
    /// Export a contract's audit surface (contract, registry, evidence, bindings, decision, events)
    Export {
        file: PathBuf,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Verify a bundle offline: record integrity + context cross-checks. Never touches the live cache.
    Verify { file: PathBuf },
}

/// Render the work order for a contract. Shared by `uni brief` (which prints or
/// writes it) and `uni run --brief` (which writes it and hands its path to the
/// executor), so the two can never drift.
pub(crate) fn render_brief(file: &Path, as_json: bool) -> Result<String> {
    let (ir, _) = crate::cmd::contract::load_contract(file)?;
    let du = dot_uni();
    let registry = uni_verify::load_registry(&du);
    let registry_text = std::fs::read_to_string(du.join("config.toml")).unwrap_or_default();
    let (claims, problems) = crate::brief::build(&ir, &registry);
    let body = if as_json {
        serde_json::to_string_pretty(&crate::brief::to_json(
            &ir,
            &claims,
            &problems,
            &crate::brief::registry_hash(&registry_text),
        )?)?
    } else {
        crate::brief::to_markdown(&ir, &claims, &problems)
    };
    Ok(body)
}

/// Write a rendered artifact atomically: a partially written brief is worse
/// than no brief, because the executor would read it.
pub(crate) fn write_atomic(path: &Path, body: &str) -> Result<()> {
    if let Some(p) = path.parent() {
        std::fs::create_dir_all(p)?;
    }
    let tmp = path.with_extension(format!("tmp-{}", std::process::id()));
    std::fs::write(&tmp, body)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

pub(crate) fn cmd_brief(file: &Path, out: Option<&Path>, as_json: bool) -> Result<()> {
    let body = render_brief(file, as_json)?;
    match out {
        Some(path) => {
            write_atomic(path, &format!("{body}\n"))?;
            if !as_json {
                eprintln!("brief written: {}", path.display());
            }
        }
        None => println!("{body}"),
    }
    Ok(())
}

pub(crate) fn cmd_bundle(sub: BundleCmd, as_json: bool) -> Result<()> {
    match sub {
        BundleCmd::Export { file, out } => {
            let du = dot_uni();
            let out = out.unwrap_or_else(|| {
                PathBuf::from(format!(
                    "uni-bundle-{}.jsonl",
                    du.display().to_string().replace(['/', '.'], "-")
                ))
            });
            let header = crate::bundle::export(&du, &file, &out)?;
            if as_json {
                println!(
                    "{}",
                    serde_json::json!({
                        "bundle": out.display().to_string(),
                        "version": header.version,
                        "intent": header.intent,
                        "records": header.records,
                        "registry_hash": header.registry_hash,
                        "contract_hash": header.contract_hash,
                    })
                );
            } else {
                println!("bundle written: {}", out.display());
                println!("intent:  {}", header.intent);
                println!("records: {}", header.records);
                println!("tool:    {}", header.tool);
            }
        }
        BundleCmd::Verify { file } => {
            let (header, report) = crate::bundle::verify(&file)?;
            let ok = report.integrity_errors.is_empty() && report.cross_check_errors.is_empty();
            if as_json {
                println!(
                    "{}",
                    serde_json::json!({
                        "bundle": file.display().to_string(),
                        "intent": header.intent,
                        "records": report.records,
                        "claims_total": report.claims_total,
                        "claims_covered": report.claims_covered,
                        "integrity_errors": report.integrity_errors,
                        "cross_check_errors": report.cross_check_errors,
                        "ok": ok,
                    })
                );
            } else {
                println!("bundle: {}", file.display());
                println!("intent: {}  records: {}", header.intent, report.records);
                println!(
                    "claims covered by evidence: {}/{}",
                    report.claims_covered, report.claims_total
                );
                for e in &report.integrity_errors {
                    println!("  INTEGRITY {e}");
                }
                for e in &report.cross_check_errors {
                    println!("  CROSS-CHECK {e}");
                }
                println!("\nOK: {ok}");
            }
            if !ok {
                return Err(anyhow!(
                    "uni bundle verify: bundle is not internally consistent"
                ));
            }
        }
    }
    Ok(())
}
