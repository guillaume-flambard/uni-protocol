use anyhow::{Result, anyhow};
use std::path::{Path};
use crate::{dot_uni, events};

/// Human authorization act for VerifierBindings, journaled as such.
pub(crate) fn cmd_bind(
    claim: Option<&str>,
    verifier: Option<&str>,
    requirement: &str,
    selector: Option<&str>,
    from: Option<&Path>,
    as_json: bool,
) -> Result<()> {
    let du = dot_uni();
    let by = uni_evidence::Actor::local().id;

    let authorized: Vec<uni_evidence::binding::VerifierBinding> = match (from, claim, verifier) {
        (Some(path), None, _) => {
            let batch = uni_evidence::binding::authorize_from_file(&du, path, &by)?;
            if !as_json {
                println!("authorized {} binding(s) from {}", batch.len(), path.display());
            }
            batch
        }
        (Some(_), Some(_), _) => {
            return Err(anyhow!(
                "use either --from <file> or --claim <claim>, not both"
            ))
        }
        (None, Some(claim), Some(verifier)) => vec![uni_evidence::binding::authorize(
            &du,
            claim,
            verifier,
            requirement,
            selector,
            &by,
        )?],
        (None, Some(claim), None) => {
            return Err(anyhow!(
                "claim '{claim}' needs --verifier (and --selector for a template)"
            ))
        }
        (None, None, _) => {
            return Err(anyhow!(
                "nothing to authorize: pass --claim <claim> --verifier <key> [--selector <test>], or --from <file>"
            ))
        }
    };

    let journal: Vec<events::Event> = authorized
        .iter()
        .map(|b| events::Event {
            name: "BindingAuthorized",
            attrs: vec![
                ("uni.claim.id".into(), b.claim_id.clone()),
                ("uni.verifier.id".into(), b.verifier_ref.clone()),
                ("uni.binding.hash".into(), b.binding_hash.clone()),
                ("uni.binding.selector".into(), b.selector.clone().unwrap_or_default()),
                ("uni.binding.by".into(), by.clone()),
            ],
        })
        .collect();
    events::append(&journal)?;

    if as_json {
        if authorized.len() == 1 {
            println!("{}", serde_json::to_string_pretty(&authorized[0])?);
        } else {
            println!("{}", serde_json::to_string_pretty(&authorized)?);
        }
        return Ok(());
    }
    for b in &authorized {
        println!("authorized: claim '{}' -> verifier '{}'", b.claim_id, b.verifier_ref);
        if !b.requirement.is_empty() {
            println!("  requirement: {}", b.requirement);
        }
        if let Some(sel) = &b.selector {
            println!("  selector:    {sel}");
        }
        println!("  binding:     {}", b.binding_hash);
    }
    Ok(())
}

pub(crate) fn cmd_bindings(as_json: bool) -> Result<()> {
    let du = dot_uni();
    // One reviewed file plus any legacy per-claim files, merged.
    let out: Vec<uni_evidence::binding::VerifierBinding> =
        uni_evidence::binding::load_all(&du).into_values().collect();
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
