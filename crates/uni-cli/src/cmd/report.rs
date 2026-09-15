use crate::dot_uni;
use anyhow::{Context, Result};

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

/// GitHub Actions workflow commands. A drifted claim becomes an inline
/// annotation on the file that moved, so the pull request shows why the proof
/// stopped applying without anyone opening the log.
fn github_annotations(v: &serde_json::Value) -> Vec<String> {
    let mut out = Vec::new();
    let claims = v["claims"].as_array().cloned().unwrap_or_default();
    for c in claims.iter().filter(|c| c["state"] != "Valid") {
        let claim = c["claim_id"].as_str().unwrap_or("claim");
        let reasons = v["stale"]
            .get(claim)
            .and_then(|r| r.as_array())
            .cloned()
            .unwrap_or_default();
        let level = if c["critical"].as_bool().unwrap_or(false) {
            "error"
        } else {
            "warning"
        };
        if reasons.is_empty() {
            out.push(format!(
                "::{level} title=claim {claim}::no valid evidence for this revision; run `uni verify {claim}`"
            ));
            continue;
        }
        // One annotation per file, preferring the reason that names it: a
        // file-less reason ("tracked files were modified") would otherwise
        // duplicate the file-naming one and land on the workflow file.
        let mut files: Vec<String> = Vec::new();
        for r in &reasons {
            if let Some(detail) = r["detail"].as_str() {
                for f in files_from_detail(detail) {
                    if !files.contains(&f) {
                        files.push(f);
                    }
                }
            }
        }
        if files.is_empty() {
            let detail = reasons[0]["detail"].as_str().unwrap_or("context changed");
            let dimension = reasons[0]["dimension"].as_str().unwrap_or("stale");
            out.push(format!(
                "::{level} title=claim {claim}::{detail} ({dimension}); run `uni verify {claim}`"
            ));
            continue;
        }
        for f in files {
            let best = reasons
                .iter()
                .find(|r| {
                    r["detail"]
                        .as_str()
                        .map(|d| files_from_detail(d).contains(&f))
                        .unwrap_or(false)
                })
                .unwrap();
            let detail = best["detail"].as_str().unwrap_or("context changed");
            let dimension = best["dimension"].as_str().unwrap_or("stale");
            out.push(format!(
                "::{level} file={f},title=claim {claim}::{detail} ({dimension}); run `uni verify {claim}`"
            ));
        }
    }
    out
}

/// Every path named in a `changed: a, b` / `added: a` / `removed: a` detail.
fn files_from_detail(detail: &str) -> Vec<String> {
    let mut out = Vec::new();
    for (marker, rest) in [("changed: ", 9), ("added: ", 7), ("removed: ", 9)] {
        if let Some(i) = detail.find(marker) {
            let list = &detail[i + rest..];
            let list = list.split(')').next().unwrap_or(list);
            for part in list.split(',') {
                let part = part.trim();
                if !part.is_empty() {
                    out.push(part.to_string());
                }
            }
        }
    }
    out
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
        println!(
            "Intent      {}",
            i.get("id").and_then(|x| x.as_str()).unwrap_or("?")
        );
    }
    let s = &r["summary"];
    println!(
        "Claims      {}/{} verified",
        s["claims_verified"], s["claims_total"]
    );
    println!(
        "Assurance   {} (independent actor: {}, identity: {})",
        r["assurance"].as_str().unwrap_or("A0"),
        if r["independent_actor"].as_bool().unwrap_or(false) {
            "YES"
        } else {
            "NO"
        },
        r["identity_assurance"].as_str().unwrap_or("SELF-DECLARED")
    );
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

pub(crate) fn cmd_explain(arg: Option<String>, as_json: bool, annotations: bool) -> Result<()> {
    let path = dot_uni().join("decisions").join("last.json");
    let text = std::fs::read_to_string(&path).context("no decision yet (run uni verify first)")?;
    let v: serde_json::Value = serde_json::from_str(&text)?;
    if annotations {
        for line in github_annotations(&v) {
            println!("{line}");
        }
        return Ok(());
    }
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
            println!(
                "  {mark} {:<24} {}",
                c["claim_id"].as_str().unwrap_or("?"),
                c["state"].as_str().unwrap_or("?")
            );
        }
    }

    let summary = v["claims"]
        .as_array()
        .map(|a| {
            let tested = a.iter().filter(|c| c["state"] == "Valid").count();
            format!("{}/{} verified", tested, a.len())
        })
        .unwrap_or_default();
    // Persisted assurance wins; legacy files fall back to the decision-only base.
    let assurance = v["assurance"]
        .as_str()
        .map(str::to_string)
        .unwrap_or_else(|| {
            let a = uni_decision::assurance_of_json(&v["decision"]);
            format!("A{a}")
        });
    let independent = v["independent_actor"].as_bool().unwrap_or(false);
    let identity = v["identity_assurance"].as_str().unwrap_or("SELF-DECLARED");
    println!("\nSummary");
    println!("  Claims     {summary}");
    println!(
        "  Assurance  {assurance} (independent actor: {}, identity: {})",
        if independent { "YES" } else { "NO" },
        identity
    );
    println!("\n{}", v["reason"].as_str().unwrap_or(""));

    // One block per claim that is not Valid, telling the story: what was
    // proven before, what is being delivered now, and why the proof stopped
    // applying. This is the product's central message, so it is spelled out.
    let failing: Vec<serde_json::Value> = v["claims"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter(|c| c["state"] != "Valid")
                .cloned()
                .collect()
        })
        .unwrap_or_default();
    let current_commit = v["commit"].as_str().unwrap_or("");
    for c in failing {
        let claim_id = c["claim_id"].as_str().unwrap_or("");
        let ev = uni_evidence::latest_for_claim(&dot_uni(), claim_id);
        println!("\nCLAIM {claim_id}");
        match &ev {
            None => {
                println!("  status       EVIDENCE_REQUIRED");
                println!("  found        no evidence bound to this revision");
                println!("  action       uni verify {claim_id}");
            }
            Some(e) => {
                let reasons = v["stale"].get(claim_id).and_then(|r| r.as_array());
                println!("  status       {:?}", e.state);
                println!("  command      {}", e.command);
                let files: Vec<String> = e.artifact_files.keys().cloned().collect();
                if !files.is_empty() {
                    println!("  watched      {}", files.join(", "));
                }
                match reasons {
                    // Drift is the interesting case: the reasons name what moved,
                    // so restating the commit twice would only add noise.
                    Some(rs) if !rs.is_empty() => {
                        for r in rs {
                            println!(
                                "  why          {} ({})",
                                r["detail"].as_str().unwrap_or(""),
                                r["dimension"].as_str().unwrap_or("")
                            );
                        }
                        if let Some(at) = e.expires_at {
                            println!("  expired      {}", at.to_rfc3339());
                        }
                    }
                    _ => {
                        println!("  exit_code    {}", e.exit_code);
                        println!(
                            "  proven on    {}",
                            &e.commit_sha[..e.commit_sha.len().min(8)]
                        );
                        if !current_commit.is_empty() {
                            println!(
                                "  delivering   {}",
                                &current_commit[..current_commit.len().min(8)]
                            );
                        }
                        if !e.output_excerpt.is_empty() {
                            let last = e.output_excerpt.lines().last().unwrap_or("").trim();
                            if !last.is_empty() {
                                println!("  output       {last}");
                            }
                        }
                    }
                }
                println!("  action       uni verify {claim_id}");
            }
        }
    }

    if let Some(f) = arg {
        let needle = f.to_lowercase();
        if !needle.is_empty() && !text.to_lowercase().contains(&needle) {
            println!("\nNo match for '{f}' in last decision.");
        }
    }
    Ok(())
}
