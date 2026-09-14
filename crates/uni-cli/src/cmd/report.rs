use anyhow::{Context, Result};
use crate::{dot_uni};

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

pub(crate) fn cmd_report(as_json: bool) -> Result<()> {
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

pub(crate) fn cmd_explain(arg: Option<String>, as_json: bool) -> Result<()> {
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
