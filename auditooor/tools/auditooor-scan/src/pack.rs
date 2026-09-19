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
    /// Write the full concatenated `source.md`. False in LITE: an unfocused
    /// source file on disk is a thing the orchestrator wanders into, and
    /// re-reading it costs more than the pack saved.
    pub write_full_source: bool,
    /// Inline the senior-auditor SOP into every bundle. DEEP only — in LITE the
    /// role file plus the shared/bounty rules already carry the method.
    pub include_sop: bool,
    /// Phase-0 prior-art brief, prepended to every bundle as "do not chase".
    pub brief: Option<String>,
}

impl Default for PackOpts {
    fn default() -> Self {
        // LITE is the default pack: 3 headline hunters, top-8 focus set.
        Self {
            roles: Some(vec!["money-map".into(), "lifecycle".into(), "spec-divergence".into()]),
            focus_files: 8,
            inline_cap: 120_000,
            write_full_source: false,
            include_sop: false,
            brief: None,
        }
    }
}

impl PackOpts {
    /// Every role, every file — what the old default did. Buy it with DEEP.
    pub fn deep() -> Self {
        Self {
            roles: None,
            focus_files: 0,
            inline_cap: 400_000,
            write_full_source: true,
            include_sop: true,
            brief: None,
        }
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

    let focus = focus_set(root, opts.focus_files);
    let bundle_source = if focus.is_empty() {
        // DEEP: source.md holds the whole tree and is the bundle body.
        let source = build_source_md(root, &[]);
        fs::write(out.join("source.md"), &source)?;
        written.push("source.md".into());
        source
    } else {
        // LITE: focus.md is the ONLY concatenated source on disk. The deferred
        // files are a path manifest at its end — read one by path when a lead
        // points at it, never the whole tree.
        let f = build_source_md(root, &focus);
        fs::write(out.join("focus.md"), &f)?;
        written.push("focus.md".into());
        if opts.write_full_source {
            let source = build_source_md(root, &[]);
            fs::write(out.join("source.md"), &source)?;
            written.push("source.md".into());
        }
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
    let sop = if opts.include_sop {
        read_if(&agents.join("senior-auditor-sop.md"))
            .or_else(|| read_if(&parent.join("senior-auditor-sop.md")))
            .unwrap_or_default()
    } else {
        String::new()
    };
    let shared = read_if(&agents.join("shared-rules.md")).unwrap_or_default();
    let bounty = read_if(&agents.join("bounty-rules.md")).unwrap_or_default();
    let mut n = 0usize;
    let dir = match fs::read_dir(agents) {
        Ok(d) => d,
        Err(_) => return Ok(0),
    };
    let fleet_dir = out.join("fleet");
    fs::create_dir_all(&fleet_dir)?;
    // Clear stale bundles from an earlier pack of the same directory. Leaving
    // them turns a 3-bundle LITE pack into a 5-bundle one on disk, and the
    // "3 agents max" rule is only real if the files agree with it.
    if let Ok(existing) = fs::read_dir(&fleet_dir) {
        for e in existing.flatten() {
            let n = e.file_name().to_string_lossy().to_string();
            if n.ends_with("-bundle.md") {
                let _ = fs::remove_file(e.path());
            }
        }
    }
    // Cap source inline so a big protocol doesn't explode across N copies.
    let inline = if source.len() < opts.inline_cap {
        source
    } else {
        "(source is over the inline cap — Read ../focus.md, then named files by path. Do not read the whole tree.)\n"
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
        if let Some(brief) = &opts.brief {
            b.push_str("## Already known — DO NOT CHASE\n\n");
            b.push_str("Prior art gathered in Phase 0. A lead matching any class below is dead \
before you write it down: it pays zero and a PoC for it is pure waste.\n\n");
            b.push_str(brief);
            b.push_str("\n\n");
        }
        b.push_str("## Source\n\n");
        b.push_str(inline);
        if opts.include_sop {
            b.push_str("\n## SOP\n\n");
            b.push_str(&sop);
        }
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

    #[test]
    fn lite_pack_writes_no_full_source_and_defers_the_rest() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures");
        let out = std::env::temp_dir().join(format!("auditooor-lite-{}", std::process::id()));
        let _ = fs::remove_dir_all(&out);
        let json = crate::xray::generate(&root, 5);
        let opts = PackOpts { focus_files: 2, ..PackOpts::default() };
        write_pack(&root, &out, &json, None, &opts).unwrap();
        // The whole-tree file must not exist: it is the thing the orchestrator
        // wanders into, and re-reading it costs more than the pack saved.
        assert!(!out.join("source.md").exists());
        let focus = fs::read_to_string(out.join("focus.md")).unwrap();
        assert!(focus.contains("Read on demand (not inlined)"));
        // Exactly the focus set is inlined.
        assert_eq!(focus.matches("\n### ").count() + usize::from(focus.starts_with("### ")), 2);
        let _ = fs::remove_dir_all(&out);
    }

    #[test]
    fn lite_bundles_carry_the_brief_and_skip_the_sop() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures");
        let agents = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../references/hunters");
        if !agents.is_dir() {
            return; // running outside the skill tree
        }
        let out = std::env::temp_dir().join(format!("auditooor-brief-{}", std::process::id()));
        let _ = fs::remove_dir_all(&out);
        let json = crate::xray::generate(&root, 5);
        let opts = PackOpts {
            brief: Some("- first-depositor inflation: mitigated in v2, known".into()),
            ..PackOpts::default()
        };
        write_pack(&root, &out, &json, Some(&agents), &opts).unwrap();
        let b = fs::read_to_string(out.join("fleet/money-map-bundle.md")).unwrap();
        assert!(b.contains("DO NOT CHASE"));
        // A re-pack with a different role set must not leave the old bundles.
        let opts2 = PackOpts { roles: Some(vec!["periphery".into()]), ..PackOpts::default() };
        write_pack(&root, &out, &json, Some(&agents), &opts2).unwrap();
        assert!(!out.join("fleet/money-map-bundle.md").exists());
        assert!(out.join("fleet/periphery-bundle.md").is_file());
        assert!(b.contains("first-depositor inflation"));
        assert!(!b.contains("## SOP"));
        let _ = fs::remove_dir_all(&out);
    }
}
