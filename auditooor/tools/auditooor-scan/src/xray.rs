//! One-shot recon pack. Mechanical half of `/auditooor XRAY`.
//!
//! Combines detect + seams + surface + detectors + entry census + git into a
//! single JSON object so the LLM spends tokens on the hunt verdict, not on
//! stitching five CLI calls. Hunt posture is a *ranking of attention*, never
//! a finding.

use crate::entries::{scan_repo_entries, Access};
use crate::git::analyze as git_analyze;
use crate::seams::analyze_all;
use crate::{json_escape as esc, repo_maturity, scan_repo, walk_files};
use std::collections::BTreeMap;
use std::path::Path;

/// Hunt posture the orchestrator should adopt after recon.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Posture {
    /// Unguarded value on sloppy or mixed code — hunt now.
    Hunt,
    /// Audited-looking core; reachable value sits in periphery glue.
    Seam,
    /// High complexity, no permissionless value, no seams — likely fortress.
    Fortress,
    /// Need the invariant campaign; per-file muscle will miss the 89%.
    Invariant,
    /// Every entry point is gated. Nothing an unprivileged caller can reach, so
    /// nothing to steal and nothing to pay. This is Abort B, as a verdict.
    NoEntry,
}

impl Posture {
    pub fn as_str(&self) -> &'static str {
        match self {
            Posture::Hunt => "HUNT",
            Posture::Seam => "SEAM",
            Posture::Fortress => "FORTRESS",
            Posture::Invariant => "INVARIANT",
            Posture::NoEntry => "ABORT",
        }
    }
}

/// Decide the hunt posture.
///
/// `perm_entries` is the *function-level* census (was this function reachable by
/// an unprivileged caller). `perm_value` is a *file-level syntactic* count of
/// value-moving sites and is far noisier — on Fluid `contracts/config` it read
/// 783 while the true count of permissionless entries was 0.
///
/// WEAK JOINT THIS CLOSES (field-reported, Fluid hunt 2026-09-19): posture ORed
/// the two (`perm_value > 0 || perm_entries > 0`), so the noisy file-level count
/// could override an accurate census of zero and return SEAM — "point the fleet
/// here" — at a directory where every entry was multisig-gated. When the census
/// has run, it decides; the syntactic count only ranks attention within it.
fn classify_posture(
    tactic: &str,
    perm_value: usize,
    perm_entries: usize,
    total_entries: usize,
    seam_n: usize,
    avg_complexity: u32,
) -> (Posture, String) {
    // The census ran and found every entry gated. Nothing else can override it:
    // a file full of `transfer` calls behind `onlyOwner` is not an attack surface.
    if total_entries > 0 && perm_entries == 0 {
        return (
            Posture::NoEntry,
            format!(
                "ABORT: {total_entries} mutating entry point(s), {} reachable by an unprivileged caller. Every way in is role- or admin-gated, so there is no unprivileged path to value — nothing to steal, nothing to pay. Do not spawn hunters. Retarget, or hunt the contracts that hold the roles.",
                0
            ),
        );
    }
    if seam_n > 0 && perm_entries > 0 {
        return (
            Posture::Seam,
            "un-audited glue with reachable value — point the fleet at SEAM contracts, skip the fortress core".into(),
        );
    }
    if avg_complexity >= 12 && perm_value == 0 && perm_entries == 0 && seam_n == 0 {
        return (
            Posture::Fortress,
            "mature core, no permissionless value, no seams — EV of a fleet here is ~0; retarget or ABORT".into(),
        );
    }
    if tactic.starts_with("SLOPPY") && (perm_entries > 0 || (total_entries == 0 && perm_value > 0)) {
        return (
            Posture::Hunt,
            "sloppy code + unguarded value — low-hanging AC/flow bugs first, then invariants".into(),
        );
    }
    if perm_entries > 0 || (total_entries == 0 && perm_value > 0) {
        return (
            Posture::Invariant,
            "value is reachable but the paying bugs are protocol-logic — author invariants, do not grind signatures".into(),
        );
    }
    (
        Posture::Fortress,
        "no permissionless value-moving entry — confirm live TVL before spending a fleet".into(),
    )
}

/// Emit the full x-ray JSON for `root`. `top` caps the surface list.
pub fn generate(root: &Path, top: usize) -> String {
    let scans = scan_repo(root);
    let (total_complexity, tactic) = repo_maturity(&scans);
    let files = scans.len().max(1) as u32;
    let avg_complexity = total_complexity / files;

    let mut eco: BTreeMap<String, usize> = BTreeMap::new();
    for s in &scans {
        *eco.entry(s.ecosystem.clone()).or_insert(0) += 1;
    }

    let mut contracts: Vec<(String, String)> = Vec::new();
    for f in walk_files(root) {
        if f.extension().and_then(|e| e.to_str()) != Some("sol") {
            continue;
        }
        let src = match std::fs::read_to_string(&f) {
            Ok(s) => s,
            Err(_) => continue,
        };
        let name = src.lines().find_map(|l| {
            l.trim().strip_prefix("contract ").map(|r| {
                r.split(|c: char| c.is_whitespace() || c == '{')
                    .next()
                    .unwrap_or("C")
                    .to_string()
            })
        });
        if let Some(name) = name {
            contracts.push((name, src));
        }
    }
    let seams = analyze_all(&contracts);
    let seam_n = seams.iter().filter(|s| s.classification == "SEAM").count();

    let entries = scan_repo_entries(root);
    let perm_entries = entries
        .iter()
        .filter(|e| e.access == Access::Permissionless)
        .count();
    let perm_value = scans
        .iter()
        .map(|s| s.value_sites.iter().filter(|v| v.permissionless_hint).count())
        .sum::<usize>();

    let mut det_hits = 0u32;
    let mut det_crit = 0u32;
    for s in &scans {
        det_hits += s.detector_hits;
    }
    // recount critical from detectors on evm files
    for f in walk_files(root) {
        if f.extension().and_then(|e| e.to_str()) != Some("sol") {
            continue;
        }
        if let Ok(src) = std::fs::read_to_string(&f) {
            for d in crate::detectors::scan_all(&src) {
                if d.severity == "CRITICAL" {
                    det_crit += 1;
                }
            }
        }
    }

    let (posture, why) =
        classify_posture(tactic, perm_value, perm_entries, entries.len(), seam_n, avg_complexity);
    let git = git_analyze(root);
    let hotspot_boost: std::collections::BTreeMap<String, u32> = git
        .hotspots
        .iter()
        .map(|(p, n)| (p.replace('\\', "/"), (*n).min(8)))
        .collect();
    let census = crate::census::analyze(root);
    let mut src_blob = String::new();
    let mut all_deltas = Vec::new();
    for f in walk_files(root) {
        if let Ok(s) = std::fs::read_to_string(&f) {
            src_blob.push_str(&s);
            src_blob.push('\n');
            all_deltas.extend(crate::deltas::scan_deltas(&s));
        }
    }
    let prop_seeds = crate::props::seeds(&entries, &src_blob.to_lowercase(), &all_deltas);
    let gaps = crate::deltas::conservation_gaps(&all_deltas);

    let mut out = String::from("{\n");
    out.push_str(&format!("  \"root\": \"{}\",\n", esc(&root.display().to_string())));
    out.push_str("  \"detect\": {\n");
    out.push_str("    \"ecosystems\": {");
    let ecos: Vec<String> = eco
        .iter()
        .map(|(k, v)| format!("\"{}\": {}", esc(k), v))
        .collect();
    out.push_str(&ecos.join(", "));
    out.push_str("},\n");
    out.push_str(&format!("    \"file_count\": {},\n", scans.len()));
    out.push_str(&format!("    \"total_complexity\": {},\n", total_complexity));
    out.push_str(&format!("    \"avg_complexity\": {},\n", avg_complexity));
    out.push_str(&format!("    \"tactic\": \"{}\"\n", esc(tactic)));
    out.push_str("  },\n");

    out.push_str("  \"posture\": {\n");
    out.push_str(&format!("    \"verdict\": \"{}\",\n", posture.as_str()));
    out.push_str(&format!("    \"why\": \"{}\",\n", esc(&why)));
    out.push_str(&format!("    \"permissionless_value_sites\": {},\n", perm_value));
    out.push_str(&format!("    \"permissionless_entries\": {},\n", perm_entries));
    out.push_str(&format!("    \"seam_contracts\": {},\n", seam_n));
    out.push_str(&format!("    \"detector_hits\": {},\n", det_hits));
    out.push_str(&format!("    \"detector_critical\": {}\n", det_crit));
    out.push_str("  },\n");

    out.push_str("  \"surface\": [\n");
    for (idx, s) in scans.iter().take(top).enumerate() {
        if idx > 0 {
            out.push_str(",\n");
        }
        let perm = s.value_sites.iter().filter(|v| v.permissionless_hint).count();
        let boost = hotspot_boost
            .iter()
            .find(|(hp, _)| s.path.ends_with(hp.as_str()) || hp.ends_with(&s.path))
            .map(|(_, n)| *n)
            .unwrap_or(0);
        out.push_str(&format!(
            "    {{\"path\": \"{}\", \"ecosystem\": \"{}\", \"risk_score\": {}, \"git_boost\": {}, \"permissionless_value_sites\": {}, \"entrypoints\": {}, \"detector_hits\": {}}}",
            esc(&s.path),
            esc(&s.ecosystem),
            s.risk_score + boost,
            boost,
            perm,
            s.entrypoints,
            s.detector_hits
        ));
    }
    out.push_str("\n  ],\n");

    out.push_str("  \"seams\": [\n");
    for (i, s) in seams.iter().enumerate() {
        if i > 0 {
            out.push_str(",\n");
        }
        out.push_str(&format!(
            "    {{\"contract\": \"{}\", \"classification\": \"{}\", \"periphery_score\": {}}}",
            esc(&s.contract),
            esc(&s.classification),
            s.periphery_score
        ));
    }
    out.push_str("\n  ],\n");

    out.push_str("  \"entries\": [\n");
    // permissionless + value-moving first already sorted; cap so JSON stays small
    let cap = 80usize;
    for (i, e) in entries.iter().take(cap).enumerate() {
        if i > 0 {
            out.push_str(",\n");
        }
        let mods: Vec<String> = e.modifiers.iter().map(|m| format!("\"{}\"", esc(m))).collect();
        out.push_str(&format!(
            "    {{\"file\": \"{}\", \"contract\": \"{}\", \"name\": \"{}\", \"line\": {}, \"access\": \"{}\", \"value_flow\": \"{}\", \"payable\": {}, \"modifiers\": [{}]}}",
            esc(&e.file),
            esc(&e.contract),
            esc(&e.name),
            e.line,
            e.access.as_str(),
            esc(&e.value_flow),
            e.payable,
            mods.join(", ")
        ));
    }
    out.push_str("\n  ],\n");
    out.push_str(&format!("  \"entry_count\": {},\n", entries.len()));
    out.push_str("  \"census\": ");
    out.push_str(&census.to_json());
    out.push_str(",\n  \"properties\": ");
    out.push_str(&crate::props::to_json(&prop_seeds));
    out.push_str(",\n  \"deltas\": ");
    out.push_str(&crate::deltas::to_json(&all_deltas, &gaps));
    out.push_str(",\n  \"danger\": ");
    out.push_str(&crate::danger::to_json(&crate::danger::scan(root)));
    out.push_str(",\n  \"git\": ");
    out.push_str(&git.to_json());
    out.push_str("\n}\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn fixtures_xray_is_hunt_or_seam_not_empty() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures");
        let json = generate(&root, 10);
        assert!(json.contains("\"posture\""));
        assert!(json.contains("\"entries\""));
        assert!(json.contains("OpenVault") || json.contains("deposit"));
        assert!(json.contains("\"verdict\":"));
        // fixtures are intentionally unguarded — not a fortress
        assert!(!json.contains("\"verdict\": \"FORTRESS\""));
        assert!(json.contains("\"census\""));
        assert!(json.contains("\"properties\""));
        assert!(json.contains("\"protocol_types\""));
        assert!(json.contains("\"deltas\""));
        assert!(json.contains("\"danger\""));
    }
}
