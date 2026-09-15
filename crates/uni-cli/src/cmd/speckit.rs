use crate::dot_uni;
use anyhow::{anyhow, Result};
use std::path::Path;

/// Minimal SpecKit importer (MVP): constitution.md + spec.md + plan.md -> candidate contract JSON.
/// Never authoritative: prints DIFF/REVIEW reminder; human must approve into uni/intents/.
pub(crate) fn cmd_import_speckit(dir: &Path, as_json: bool) -> Result<()> {
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
            let desc = t
                .split_once(|c| c == ':' || c == ' ')
                .map(|(_, d)| d.trim())
                .unwrap_or(t);
            (
                head.trim_end_matches(['*', ':']).to_lowercase(),
                desc.to_string(),
            )
        } else if let Some(b) = t.strip_prefix("**FR-") {
            let head: &str = b.split([':', '*', ' ']).next().unwrap_or("");
            let desc = b
                .split_once("**:")
                .map(|(_, d)| d.trim())
                .unwrap_or(t.trim_start_matches("**"));
            (
                format!("fr-{}", head.trim_matches(['*', ':'])),
                desc.to_string(),
            )
        } else if t.starts_with("Requirement") {
            (
                t.split_whitespace().next().unwrap_or("").to_lowercase(),
                t.to_string(),
            )
        } else if let Some(s) = t.strip_prefix("Scenario:") {
            (
                format!("scenario-{:02}", claims.len() + 1),
                s.trim().to_string(),
            )
        } else if t.starts_with("[ ]") || t.starts_with("[x]") {
            (
                format!("check-{:02}", claims.len() + 1),
                t[4..].trim().to_string(),
            )
        } else {
            continue;
        };
        if !maybe_id.is_empty() && !claims.iter().any(|(c, _)| *c == maybe_id) {
            claims.push((maybe_id, text.chars().take(160).collect()));
        }
    }
    let intent_id = dir
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
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
        dsl.push_str(&format!(
            "CLAIM {id} REQUIRED\n  ENSURE {}\n",
            ensure.replace('\n', " ")
        ));
    }
    for (id, _) in &claims {
        dsl.push_str(&format!("VERIFY {id}\n  USING project.tests\n"));
    }
    dsl.push_str("ACCEPT WHEN\n  required_claims == VERIFIED\n  AND critical_failures == 0\n");

    let out_dsl = dot_uni()
        .join("contracts")
        .join(format!("candidate-{intent_id}.uni"));
    std::fs::create_dir_all(dot_uni().join("contracts"))?;
    std::fs::write(&out_dsl, &dsl)?;

    // Ship the work order next to the candidate: the study showed that leaving
    // the registry->test-name hop implicit causes correct work to be rejected.
    // Unresolvable verifiers surface here, before anyone starts implementing.
    let out_brief = dot_uni()
        .join("contracts")
        .join(format!("candidate-{intent_id}.brief.md"));
    let (brief_claims, brief_problems) =
        match uni_parser::parse(&dsl).and_then(|ast| uni_ir::compile(&ast)) {
            Ok(ir) => {
                let registry = uni_verify::load_registry(&dot_uni());
                let (claims, problems) = crate::brief::build(&ir, &registry);
                std::fs::write(
                    &out_brief,
                    format!("{}\n", crate::brief::to_markdown(&ir, &claims, &problems)),
                )?;
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
