use anyhow::{Result, anyhow};
use std::path::{PathBuf};
use clap::Subcommand;
use crate::{dot_uni};

#[derive(Subcommand)]
pub enum PackCmd {
    /// List domain packs found in ./packs (and .uni/packs when installed).
    List,
    /// Materialize a pack template into uni/intents/<name>.uni for editing.
    Template { pack: String, name: String },
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

pub(crate) fn cmd_pack(sub: PackCmd, as_json: bool) -> Result<()> {
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
