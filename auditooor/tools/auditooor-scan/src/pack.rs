//! On-disk recon pack. This is the artifact Pashov x-ray writes in 15 LLM minutes
//! and fizz consumes. We write it in one process so `/auditooor FUZZ` and the
//! 12-agent fleet start from the same files, not conversation memory.

use std::fs;
use std::io::{self, Write};
use std::path::Path;

/// Write `{out}/` containing xray.json, source.md, entries.md, PROPERTIES.md, hunt.md.
/// Optional `--fleet` copies agent bundles next to source.md.
/// How much machine the pack is being built for.
///
/// The default used to be: concatenate every in-scope file, then inline that
/// whole blob into 15 agent bundles. On a mid-size protocol that is the single
/// largest token line item in a run, and most of it is source no agent will
/// ever cite. `PackOpts` makes the pack *earn* every byte it hands out.
pub struct PackOpts {
    /// Only write bundles for these roles (bare names, e.g. `money-map`).
    /// `None` = every `*-agent.md` in the agents dir.
    pub roles: Option<Vec<String>>,
    /// Inline only the top-N files by `risk_score` into each bundle; the rest
    /// are listed as a read-on-demand manifest. 0 = inline everything.
    pub focus_files: usize,
    /// Hard cap on inlined source bytes per bundle.
    pub inline_cap: usize,
}

impl Default for PackOpts {
    fn default() -> Self {
        // LITE is the default pack: 3 headline hunters, top-8 focus set.
        Self {
            roles: Some(vec!["money-map".into(), "lifecycle".into(), "spec-divergence".into()]),
            focus_files: 8,
            inline_cap: 120_000,
        }
    }
}

impl PackOpts {
    /// Every role, every file — what the old default did. Buy it with DEEP.
    pub fn deep() -> Self {
        Self { roles: None, focus_files: 0, inline_cap: 400_000 }
    }
}

pub fn write_pack(
    root: &Path,
    out: &Path,
    xray_json: &str,
    fleet_agents: Option<&Path>,
    opts: &PackOpts,
) -> io::Result<Vec<String>> {
    fs::create_dir_all(out)?;
    let mut written = Vec::new();

    fs::write(out.join("xray.json"), xray_json)?;
    written.push("xray.json".into());

    // source.md always holds the full tree — it is the read-on-demand backstop.
    let source = build_source_md(root, &[]);
    fs::write(out.join("source.md"), &source)?;
    written.push("source.md".into());

    let focus = focus_set(root, opts.focus_files);
    let bundle_source = if focus.is_empty() {
        source.clone()
    } else {
        let f = build_source_md(root, &focus);
        fs::write(out.join("focus.md"), &f)?;
        written.push("focus.md".into());
        f
    };

    let entries_md = entries_md_from_json(xray_json);
    fs::write(out.join("entries.md"), entries_md)?;
    written.push("entries.md".into());

    // PROPERTIES.md is also produced from the same xray JSON's properties array
    // if present; otherwise a stub.
    let props = props_md_from_json(xray_json);
    fs::write(out.join("PROPERTIES.md"), props)?;
    written.push("PROPERTIES.md".into());

    let hunt = hunt_md_from_json(xray_json);
    fs::write(out.join("hunt.md"), hunt)?;
    written.push("hunt.md".into());

    if let Some(agents) = fleet_agents {
        let n = write_fleet_bundles(out, &bundle_source, agents, opts)?;
        written.push(format!("fleet bundles: {n}"));
    }
    Ok(written)
}

/// Top-N in-scope files by `risk_score`. Empty when `n == 0` (inline everything).
fn focus_set(root: &Path, n: usize) -> Vec<String> {
    if n == 0 {
        return Vec::new();
    }
    crate::scan_repo(root).into_iter().take(n).map(|s| s.path).collect()
}

/// Concatenate in-scope source. With a non-empty `focus`, inline only those
/// files and list the rest as a read-on-demand manifest — an agent that needs a
/// deferred file reads it by path, instead of every agent paying for every file.
fn build_source_md(root: &Path, focus: &[String]) -> String {
    let focused = !focus.is_empty();
    let mut s = String::from("# In-scope source\n\n");
    if focused {
        s.push_str("**Focus set** — the top files by `risk_score`, inlined. Everything else is \
listed at the bottom: read those by path only if your lane actually needs them.\n\n");
    } else {
        s.push_str("Concatenated by `auditooor-scan pack`. Agents read this instead of globbing the repo.\n\n");
    }
    let mut deferred: Vec<String> = Vec::new();
    for f in crate::walk_files(root) {
        let rel = f.strip_prefix(root).unwrap_or(&f).to_string_lossy().to_string();
        if focused && !focus.iter().any(|p| *p == rel) {
            deferred.push(rel);
            continue;
        }
        let src = match fs::read_to_string(&f) {
            Ok(x) => x,
            Err(_) => continue,
        };
        s.push_str(&format!("### {rel}\n\n```\n{src}\n```\n\n"));
    }
    if !deferred.is_empty() {
        s.push_str("## Read on demand (not inlined)\n\n");
        s.push_str("Lower `risk_score`. Read the path directly if a lead points at it.\n\n");
        for d in &deferred {
            s.push_str(&format!("- `{d}`\n"));
        }
        s.push('\n');
    }
    s
}

fn json_string_field(obj: &str, key: &str) -> String {
    let pat = format!("\"{key}\": \"");
    if let Some(i) = obj.find(&pat) {
        let rest = &obj[i + pat.len()..];
        let mut out = String::new();
        let mut chars = rest.chars();
        while let Some(c) = chars.next() {
            if c == '\\' {
                if let Some(n) = chars.next() {
                    out.push(n);
                }
            } else if c == '"' {
                break;
            } else {
                out.push(c);
            }
        }
        return out;
    }
    String::new()
}

fn hunt_md_from_json(json: &str) -> String {
    let verdict = json_string_field(json, "verdict");
    let why = json_string_field(json, "why");
    let mut s = String::from("# Hunt brief (machine)\n\n");
    s.push_str(&format!("**Posture:** `{verdict}`\n\n{why}\n\n"));
    s.push_str("LLM: overlay protocol-specific invariant wording on `PROPERTIES.md`. Do not rewrite posture.\n");
    s.push_str("Next: `FORTRESS` → ABORT; `INVARIANT`/`HUNT`/`SEAM` → `/auditooor FUZZ` then fleet if still +EV.\n");
    s
}

fn entries_md_from_json(json: &str) -> String {
    let mut s = String::from("# Entry points\n\nMachine census. Permissionless + value-flow rows are the impact map.\n\n");
    s.push_str("| access | flow | contract.fn | loc |\n|---|---|---|---|\n");
    // crude row scrape from JSON objects in "entries"
    for chunk in json.split('{') {
        if !chunk.contains("\"name\":") || !chunk.contains("\"access\":") {
            continue;
        }
        let file = json_string_field(&format!("{{{chunk}"), "file");
        let contract = json_string_field(&format!("{{{chunk}"), "contract");
        let name = json_string_field(&format!("{{{chunk}"), "name");
        let access = json_string_field(&format!("{{{chunk}"), "access");
        let flow = json_string_field(&format!("{{{chunk}"), "value_flow");
        if name.is_empty() || contract.is_empty() {
            continue;
        }
        let line = chunk
            .split("\"line\": ")
            .nth(1)
            .and_then(|r| r.split(',').next())
            .unwrap_or("?");
        s.push_str(&format!(
            "| {access} | {flow} | `{contract}.{name}` | {file}:{line} |\n"
        ));
    }
    s
}

fn props_md_from_json(json: &str) -> String {
    if !json.contains("\"properties\"") {
        return "# PROPERTIES\n\n(no seeds)\n".into();
    }
    let mut s = String::from("# PROPERTIES (mechanical seeds)\n\n");
    for chunk in json.split('{') {
        if !chunk.contains("\"statement\":") || !chunk.contains("\"guarantee\":") {
            continue;
        }
        let obj = format!("{{{chunk}");
        let id = json_string_field(&obj, "id");
        if !id.starts_with("P-") {
            continue;
        }
        let kind = json_string_field(&obj, "kind");
        let g = json_string_field(&obj, "guarantee");
        let stmt = json_string_field(&obj, "statement");
        s.push_str(&format!("- [ ] `{id}` ({kind}, {g}) {stmt}\n"));
    }
    s
}

fn write_fleet_bundles(
    out: &Path,
    source: &str,
    agents: &Path,
    opts: &PackOpts,
) -> io::Result<usize> {
    let parent = agents.parent().unwrap_or(agents);
    let sop = read_if(&agents.join("senior-auditor-sop.md"))
        .or_else(|| read_if(&parent.join("senior-auditor-sop.md")))
        .unwrap_or_default();
    let shared = read_if(&agents.join("shared-rules.md")).unwrap_or_default();
    let bounty = read_if(&agents.join("bounty-rules.md")).unwrap_or_default();
    let mut n = 0usize;
    let dir = match fs::read_dir(agents) {
        Ok(d) => d,
        Err(_) => return Ok(0),
    };
    let fleet_dir = out.join("fleet");
    fs::create_dir_all(&fleet_dir)?;
    // Cap source inline so a big protocol doesn't explode across N copies.
    let inline = if source.len() < opts.inline_cap {
        source
    } else {
        "(source is over the inline cap — Read ../source.md and ../focus.md; start with focus.md.)\n"
    };
    for ent in dir.flatten() {
        let p = ent.path();
        let name = ent.file_name().to_string_lossy().to_string();
        if !name.ends_with("-agent.md") {
            continue;
        }
        if let Some(roles) = &opts.roles {
            let bare = name.trim_end_matches("-agent.md");
            if !roles.iter().any(|r| r.trim_end_matches("-agent") == bare) {
                continue;
            }
        }
        let body = fs::read_to_string(&p).unwrap_or_default();
        let mut b = String::new();
        b.push_str(&format!("# Bundle {name}\n\n"));
        b.push_str("## Source\n\n");
        b.push_str(inline);
        b.push_str("\n## SOP\n\n");
        b.push_str(&sop);
        b.push_str("\n## Specialty\n\n");
        b.push_str(&body);
        b.push_str("\n## Shared rules\n\n");
        b.push_str(&shared);
        b.push_str("\n## Bounty rules (override shared-rules where they conflict)\n\n");
        b.push_str(&bounty);
        let out_name = name.replace("-agent.md", "-bundle.md");
        fs::write(fleet_dir.join(&out_name), b)?;
        n += 1;
    }
    let mut idx = fs::File::create(fleet_dir.join("README.md"))?;
    writeln!(
        idx,
        "{n} bundle(s). Spawn one agent per `*-bundle.md` in parallel. Completeness: every (contract, function) named in any FINDING/LEAD must appear in the merged report. Bundles inline the focus set only; deferred files are listed at the end of the Source section and read by path on demand."
    )?;
    Ok(n)
}

fn read_if(p: &Path) -> Option<String> {
    fs::read_to_string(p).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn pack_writes_core_files() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures");
        let out = std::env::temp_dir().join(format!("auditooor-pack-{}", std::process::id()));
        let _ = fs::remove_dir_all(&out);
        let json = crate::xray::generate(&root, 5);
        let files = write_pack(&root, &out, &json, None, &PackOpts::deep()).unwrap();
        assert!(files.iter().any(|f| f == "xray.json"));
        assert!(out.join("source.md").is_file());
        assert!(out.join("hunt.md").is_file());
        let src = fs::read_to_string(out.join("source.md")).unwrap();
        assert!(src.contains("OpenVault"));
        let _ = fs::remove_dir_all(&out);
    }
}
