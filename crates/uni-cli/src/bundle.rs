//! Evidence bundles (v0.3): export the audit surface of a decision to a
//! transport file, and verify that file offline.
//!
//! Design stance: a bundle is EVIDENCE TRANSPORT, never authority. Import
//! (verification) never injects proofs into a live cache; it checks internal
//! consistency and reports. Per-record sha256 detects transport tampering;
//! context cross-checks detect a bundle whose pieces do not belong together.

use anyhow::{anyhow, Context, Result};
use std::path::Path;

pub const BUNDLE_VERSION: &str = "uni-bundle-0.1";

pub struct BundleHeader {
    pub version: String,
    pub intent: String,
    pub registry_hash: String,
    pub contract_hash: String,
    pub created_at: String,
    pub tool: String,
    pub records: usize,
}

/// One transport record. `sha256` covers the canonical serialization of `body`,
/// so any edit of the body in transit is detectable offline.
pub fn record(kind: &str, body: &serde_json::Value) -> Result<serde_json::Value> {
    let canonical = serde_json::to_string(body)?;
    Ok(serde_json::json!({
        "kind": kind,
        "sha256": uni_evidence::sha256_hex(canonical.as_bytes()),
        "body": body,
    }))
}

/// Export a contract's audit surface: contract text, registry + policies
/// snapshots, evidence files, bindings, the last decision, and the event
/// journal. Everything the decision rests on, nothing regenerable beyond it.
pub fn export(dot_uni: &Path, contract_path: &Path, out: &Path) -> Result<BundleHeader> {
    let contract_text = std::fs::read_to_string(contract_path)
        .with_context(|| format!("read {}", contract_path.display()))?;
    let (ir, _) = {
        let ast = uni_parser::parse(&contract_text)?;
        let ir = uni_ir::compile(&ast)?;
        (ir, ast)
    };
    let registry_text = std::fs::read_to_string(dot_uni.join("config.toml")).unwrap_or_default();
    let registry_hash = uni_evidence::sha256_hex(registry_text.as_bytes());
    let contract_hash = uni_evidence::sha256_hex(contract_text.as_bytes());

    let mut lines: Vec<String> = vec![];
    let mut records = 0usize;
    let mut push = |kind: &str, body: serde_json::Value| -> Result<()> {
        lines.push(serde_json::to_string(&record(kind, &body)?)?);
        records += 1;
        Ok(())
    };

    push("contract", serde_json::json!({
        "path": contract_path.display().to_string(),
        "text": contract_text,
        "hash": contract_hash,
        "ir": ir,
    }))?;
    push("registry", serde_json::json!({ "text": registry_text, "hash": registry_hash }))?;

    // Policies: the merged view is already persisted at decision time via
    // policy_hash on evidence; ship the raw files for audit.
    let policies_dir = dot_uni.join("policies");
    if let Ok(rd) = std::fs::read_dir(&policies_dir) {
        let mut files: Vec<_> = rd.flatten().map(|e| e.path()).collect();
        files.sort();
        for f in files {
            if f.extension().map(|x| x == "toml" || x == "rego").unwrap_or(false) {
                if let Ok(text) = std::fs::read_to_string(&f) {
                    push("policy", serde_json::json!({
                        "name": f.file_name().unwrap_or_default().to_string_lossy(),
                        "text": text,
                    }))?;
                }
            }
        }
    }

    // Evidence: only files whose claim appears in this contract.
    let claim_ids: std::collections::BTreeSet<&str> =
        ir.claims.iter().map(|c| c.id.as_str()).collect();
    let ev_dir = dot_uni.join("evidence");
    if let Ok(rd) = std::fs::read_dir(&ev_dir) {
        let mut files: Vec<_> = rd.flatten().map(|e| e.path()).collect();
        files.sort();
        for f in files {
            let Some(ev) = uni_evidence::load_json::<uni_evidence::Evidence>(&f) else {
                continue;
            };
            if claim_ids.contains(ev.claim_id.as_str()) {
                push("evidence", serde_json::to_value(&ev)?)?;
            }
        }
    }

    // Bindings referenced by this contract's requirements.
    for v in &ir.verification {
        if v.requirement.is_some() {
            if let Some(b) = uni_evidence::binding::load_binding(dot_uni, &v.claim_id) {
                push("binding", serde_json::to_value(&b)?)?;
            }
        }
    }

    // Last decision, if it belongs to this intent.
    if let Some(decision) = uni_evidence::load_json::<serde_json::Value>(
        &dot_uni.join("decisions").join("last.json"),
    ) {
        if decision
            .get("intent")
            .and_then(|i| i.get("id"))
            .and_then(|i| i.as_str())
            == Some(ir.intent.id.as_str())
        {
            push("decision", decision)?;
        }
    }

    // Event journal (append-only, shipped as one record for audit).
    if let Ok(text) = std::fs::read_to_string(dot_uni.join("events.jsonl")) {
        let events: Vec<serde_json::Value> = text
            .lines()
            .filter(|l| !l.trim().is_empty())
            .filter_map(|l| serde_json::from_str(l).ok())
            .collect();
        push("events", serde_json::json!({ "events": events }))?;
    }

    let header = BundleHeader {
        version: BUNDLE_VERSION.to_string(),
        intent: ir.intent.id.clone(),
        registry_hash,
        contract_hash,
        created_at: chrono::Utc::now().to_rfc3339(),
        tool: format!("uni {}", env!("CARGO_PKG_VERSION")),
        records,
    };
    let header_line = serde_json::to_string(&serde_json::json!({
        "kind": "header",
        "sha256": uni_evidence::sha256_hex(
            serde_json::to_string(&serde_json::json!({
                "version": &header.version,
                "intent": &header.intent,
                "registry_hash": &header.registry_hash,
                "contract_hash": &header.contract_hash,
            }))?.as_bytes()),
        "body": {
            "version": header.version,
            "intent": header.intent,
            "registry_hash": header.registry_hash,
            "contract_hash": header.contract_hash,
            "created_at": header.created_at,
            "tool": header.tool,
            "records": header.records,
        }
    }))?;

    let mut text = header_line;
    for l in &lines {
        text.push('\n');
        text.push_str(l);
    }
    text.push('\n');
    if let Some(p) = out.parent() {
        std::fs::create_dir_all(p)?;
    }
    // Atomic write, same discipline as evidence.
    let tmp = out.with_extension(format!("tmp-{}", std::process::id()));
    std::fs::write(&tmp, text)?;
    std::fs::rename(&tmp, out)?;
    Ok(header)
}

#[derive(Debug, Default)]
pub struct BundleReport {
    pub records: usize,
    pub integrity_errors: Vec<String>,
    pub cross_check_errors: Vec<String>,
    pub claims_covered: usize,
    pub claims_total: usize,
}

/// Offline verification: never touches the live cache.
pub fn verify(path: &Path) -> Result<(BundleHeader, BundleReport)> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("read {}", path.display()))?;
    let mut lines = text.lines().filter(|l| !l.trim().is_empty());
    let header_line = lines.next().ok_or_else(|| anyhow!("empty bundle"))?;
    let header_json: serde_json::Value = serde_json::from_str(header_line)?;
    if header_json.get("kind").and_then(|k| k.as_str()) != Some("header") {
        return Err(anyhow!("first line must be the bundle header"));
    }
    let hb = &header_json["body"];
    let header = BundleHeader {
        version: hb["version"].as_str().unwrap_or("").to_string(),
        intent: hb["intent"].as_str().unwrap_or("").to_string(),
        registry_hash: hb["registry_hash"].as_str().unwrap_or("").to_string(),
        contract_hash: hb["contract_hash"].as_str().unwrap_or("").to_string(),
        created_at: hb["created_at"].as_str().unwrap_or("").to_string(),
        tool: hb["tool"].as_str().unwrap_or("").to_string(),
        records: hb["records"].as_u64().unwrap_or(0) as usize,
    };
    if header.version != BUNDLE_VERSION {
        return Err(anyhow!(
            "unsupported bundle version '{}' (expected {BUNDLE_VERSION})",
            header.version
        ));
    }
    // Header integrity.
    let mut report = BundleReport::default();
    let expected_header_hash = uni_evidence::sha256_hex(
        serde_json::to_string(&serde_json::json!({
            "version": &header.version,
            "intent": &header.intent,
            "registry_hash": &header.registry_hash,
            "contract_hash": &header.contract_hash,
        }))?
        .as_bytes(),
    );
    if header_json["sha256"].as_str() != Some(expected_header_hash.as_str()) {
        report
            .integrity_errors
            .push("header sha256 mismatch".into());
    }

    let mut evidence_claim_ids: std::collections::BTreeSet<String> =
        Default::default();
    let mut contract_claim_ids: std::collections::BTreeSet<String> =
        Default::default();
    let mut bindings: std::collections::BTreeSet<String> = Default::default();
    let mut requirements: Vec<(String, String)> = vec![];
    let mut decision_claims: std::collections::BTreeSet<String> = Default::default();

    for line in lines {
        let rec: serde_json::Value = serde_json::from_str(line)
            .map_err(|e| anyhow!("malformed record: {e}"))?;
        report.records += 1;
        let kind = rec.get("kind").and_then(|k| k.as_str()).unwrap_or("?");
        let body = &rec["body"];
        // Per-record integrity.
        let canonical = serde_json::to_string(body)?;
        let actual = uni_evidence::sha256_hex(canonical.as_bytes());
        if rec.get("sha256").and_then(|s| s.as_str()) != Some(actual.as_str()) {
            report
                .integrity_errors
                .push(format!("record {kind}: sha256 mismatch"));
        }
        match kind {
            "contract" => {
                // Contract text must hash to the header's contract_hash.
                let text = body["text"].as_str().unwrap_or("");
                if uni_evidence::sha256_hex(text.as_bytes()) != header.contract_hash {
                    report
                        .integrity_errors
                        .push("contract text does not match header contract_hash".into());
                }
                if let Some(claims) = body["ir"]["claims"].as_array() {
                    for c in claims {
                        if let Some(id) = c["id"].as_str() {
                            contract_claim_ids.insert(id.to_string());
                        }
                    }
                }
                if let Some(verifs) = body["ir"]["verification"].as_array() {
                    for v in verifs {
                        if let Some(req) = v["requirement"].as_str() {
                            requirements.push((
                                v["claim_id"].as_str().unwrap_or("").to_string(),
                                req.to_string(),
                            ));
                        }
                    }
                }
            }
            "registry" => {
                let text = body["text"].as_str().unwrap_or("");
                if uni_evidence::sha256_hex(text.as_bytes()) != header.registry_hash {
                    report
                        .integrity_errors
                        .push("registry snapshot does not match header registry_hash".into());
                }
            }
            "evidence" => {
                let ev: uni_evidence::Evidence = serde_json::from_value(body.clone())?;
                evidence_claim_ids.insert(ev.claim_id.clone());
                if !ev.registry_hash.is_empty()
                    && ev.registry_hash != header.registry_hash
                {
                    report.cross_check_errors.push(format!(
                        "evidence '{}' was gathered under a different registry",
                        ev.claim_id
                    ));
                }
                if !ev.contract_hash.is_empty()
                    && ev.contract_hash != header.contract_hash
                {
                    report.cross_check_errors.push(format!(
                        "evidence '{}' was gathered under a different contract",
                        ev.claim_id
                    ));
                }
            }
            "binding" => {
                if let Some(c) = body["claim_id"].as_str() {
                    bindings.insert(c.to_string());
                }
            }
            "decision" => {
                if let Some(claims) = body["claims"].as_array() {
                    for c in claims {
                        if let Some(id) = c["claim_id"].as_str() {
                            decision_claims.insert(id.to_string());
                        }
                    }
                }
            }
            _ => {}
        }
    }

    if report.records != header.records {
        report.integrity_errors.push(format!(
            "header declares {} records, file has {}",
            header.records, report.records
        ));
    }
    // Every requirement must ship its authorizing binding.
    for (claim, req) in &requirements {
        if !bindings.contains(claim) {
            report.cross_check_errors.push(format!(
                "claim '{claim}' requires '{req}' but no binding is bundled"
            ));
        }
    }
    // Every decision claim must be backed by bundled evidence.
    for c in &decision_claims {
        if !evidence_claim_ids.contains(c) {
            report.cross_check_errors.push(format!(
                "decision cites claim '{c}' with no bundled evidence"
            ));
        }
    }
    // Evidence for claims that are not in the contract is suspicious.
    for c in evidence_claim_ids.difference(&contract_claim_ids) {
        report.cross_check_errors.push(format!(
            "bundled evidence for unknown claim '{c}'"
        ));
    }
    report.claims_total = contract_claim_ids.len();
    report.claims_covered = contract_claim_ids
        .intersection(&evidence_claim_ids)
        .count();
    Ok((header, report))
}
