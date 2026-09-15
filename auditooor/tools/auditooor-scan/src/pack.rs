//! On-disk recon pack. This is the artifact Pashov x-ray writes in 15 LLM minutes
//! and fizz consumes. We write it in one process so `/auditooor FUZZ` and the
//! 12-agent fleet start from the same files, not conversation memory.

use std::fs;
use std::io::{self, Write};
use std::path::Path;

/// Write `{out}/` containing xray.json, source.md, entries.md, PROPERTIES.md, hunt.md.
/// Optional `--fleet` copies agent bundles next to source.md.
pub fn write_pack(root: &Path, out: &Path, xray_json: &str, fleet_agents: Option<&Path>) -> io::Result<Vec<String>> {
    fs::create_dir_all(out)?;
    let mut written = Vec::new();

    fs::write(out.join("xray.json"), xray_json)?;
    written.push("xray.json".into());

    let source = build_source_md(root);
    fs::write(out.join("source.md"), &source)?;
    written.push("source.md".into());

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
        let n = write_fleet_bundles(out, &source, agents)?;
        written.push(format!("fleet bundles: {n}"));
    }
    Ok(written)
}

fn build_source_md(root: &Path) -> String {
    let mut s = String::from("# In-scope source\n\n");
    s.push_str("Concatenated by `auditooor-scan pack`. Agents read this instead of globbing the repo.\n\n");
    for f in crate::walk_files(root) {
        let rel = f.strip_prefix(root).unwrap_or(&f).to_string_lossy();
        let src = match fs::read_to_string(&f) {
            Ok(x) => x,
            Err(_) => continue,
        };
        s.push_str(&format!("### {rel}\n\n```\n{src}\n```\n\n"));
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

fn write_fleet_bundles(out: &Path, source: &str, agents: &Path) -> io::Result<usize> {
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
    // Cap source inline so a 2MB protocol doesn't explode 12 copies.
    let inline = if source.len() < 400_000 {
        source
    } else {
        "(source.md is large — Read ../source.md; do not skip files.)\n"
    };
    for ent in dir.flatten() {
        let p = ent.path();
        let name = ent.file_name().to_string_lossy().to_string();
        if !name.ends_with("-agent.md") {
            continue;
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
        "12 (or fewer) bundles. Spawn one agent per `*-bundle.md` in parallel. Completeness: every (contract, function) named in any FINDING/LEAD must appear in the merged report."
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
        let files = write_pack(&root, &out, &json, None).unwrap();
        assert!(files.iter().any(|f| f == "xray.json"));
        assert!(out.join("source.md").is_file());
        assert!(out.join("hunt.md").is_file());
        let src = fs::read_to_string(out.join("source.md")).unwrap();
        assert!(src.contains("OpenVault"));
        let _ = fs::remove_dir_all(&out);
    }
}
