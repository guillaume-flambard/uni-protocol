use anyhow::{Result, anyhow};
use crate::{dot_uni, events};

/// Workspace health check, read-only: git binding, registry, policies,
/// evidence/journal writability. Exit 0 when everything is healthy.
pub(crate) fn cmd_doctor(as_json: bool) -> Result<()> {
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
    let jr_size = std::fs::metadata(events::journal_path())
        .map(|m| m.len())
        .unwrap_or(0);
    let jr_archives = events::archives().len();
    checks.push((
        "journal writable".into(),
        jr_ok,
        format!(
            "{} KiB current, {} archive(s), rotates at {} KiB",
            jr_size / 1024,
            jr_archives,
            events::JOURNAL_MAX_BYTES / 1024
        ),
    ));

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

pub(crate) fn cmd_events(as_json: bool, max: usize, all: bool, otlp: bool) -> Result<()> {
    let evts = if all {
        events::read_all_including_archives()?
    } else {
        events::read_all()?
    };
    if otlp {
        println!("{}", serde_json::to_string_pretty(&events::to_otlp(&evts))?);
        return Ok(());
    }
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
