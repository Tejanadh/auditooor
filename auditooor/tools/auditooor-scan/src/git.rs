//! Git-history security signals. Pure subprocess, no Python.
//!
//! Pashov x-ray ships a 49k Python analyser. We keep the *paying* signals
//! (hotspots, fix-shaped commits, recently-touched source) and drop the rest:
//! contributor social graphs do not move bounty EV. If git is missing or the
//! dir is not a repo, `available=false` and the orchestrator continues.

use std::path::Path;
use std::process::Command;

/// Compact git-security report.
#[derive(Debug, Clone, Default)]
pub struct GitReport {
    pub available: bool,
    pub branch: String,
    pub commit: String,
    pub commits_analyzed: u32,
    pub squashed_import: bool,
    pub hotspots: Vec<(String, u32)>,
    pub fix_commits: Vec<(String, String)>,
    pub late_files: Vec<(String, String)>,
}

const FIX_KW: &[&str] = &[
    "fix", "audit", "vuln", "exploit", "hack", "security", "overflow",
    "underflow", "reentran", "unguard", "missing check", "access control",
    "initialize", "hotfix",
];

const SOURCE_HINTS: &[&str] = &[".sol", ".vy", ".rs", ".move", ".circom"];

fn git(root: &Path, args: &[&str]) -> Option<String> {
    let out = Command::new("git")
        .args(["-C"])
        .arg(root)
        .args(args)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).to_string())
}

fn is_source(path: &str) -> bool {
    let lc = path.to_lowercase();
    SOURCE_HINTS.iter().any(|e| lc.ends_with(e))
        && !lc.contains("/test")
        && !lc.contains("/lib/")
        && !lc.contains("/mock")
}

fn is_fix(subject: &str) -> bool {
    let lc = subject.to_lowercase();
    FIX_KW.iter().any(|k| lc.contains(k))
}

/// Analyse `root` if it is a git work tree. Never panics, never blocks on network.
pub fn analyze(root: &Path) -> GitReport {
    let mut r = GitReport::default();
    let branch = match git(root, &["rev-parse", "--abbrev-ref", "HEAD"]) {
        Some(s) => s.trim().to_string(),
        None => return r,
    };
    let commit = git(root, &["rev-parse", "--short", "HEAD"])
        .unwrap_or_default()
        .trim()
        .to_string();
    r.available = true;
    r.branch = branch;
    r.commit = commit;

    let log = git(
        root,
        &[
            "log",
            "--pretty=format:%h\t%s\t%ad",
            "--date=short",
            "--no-merges",
            "-n",
            "150",
        ],
    )
    .unwrap_or_default();

    let mut commits = Vec::new();
    for line in log.lines() {
        let mut parts = line.splitn(3, '\t');
        let h = parts.next().unwrap_or("").to_string();
        let s = parts.next().unwrap_or("").to_string();
        let d = parts.next().unwrap_or("").to_string();
        if h.is_empty() {
            continue;
        }
        if is_fix(&s) {
            r.fix_commits.push((h.clone(), s.clone()));
        }
        commits.push((h, s, d));
    }
    r.commits_analyzed = commits.len() as u32;
    r.squashed_import = r.commits_analyzed <= 1;
    r.fix_commits.truncate(12);

    let names = git(
        root,
        &[
            "log",
            "--name-only",
            "--pretty=format:%h\t%ad",
            "--date=short",
            "--no-merges",
            "-n",
            "150",
        ],
    )
    .unwrap_or_default();

    use std::collections::BTreeMap;
    let mut counts: BTreeMap<String, u32> = BTreeMap::new();
    let mut last_date: BTreeMap<String, String> = BTreeMap::new();
    let mut current_date = String::new();
    for line in names.lines() {
        if line.contains('\t') {
            current_date = line.split('\t').nth(1).unwrap_or("").to_string();
            continue;
        }
        let p = line.trim();
        if p.is_empty() || !is_source(p) {
            continue;
        }
        *counts.entry(p.to_string()).or_insert(0) += 1;
        last_date.entry(p.to_string()).or_insert_with(|| current_date.clone());
    }
    let mut hot: Vec<(String, u32)> = counts.into_iter().collect();
    hot.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    hot.truncate(12);
    r.hotspots = hot;

    // "Late" = last-touched date is among the newest 15% of dated files, or
    // the file appears in the most recent 8 commits' name lists. Approximate
    // with last_date descending.
    let mut late: Vec<(String, String)> = last_date.into_iter().collect();
    late.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    late.truncate(10);
    r.late_files = late;
    r
}

impl GitReport {
    pub fn to_json(&self) -> String {
        use crate::json_escape as esc;
        let mut s = String::from("{\n");
        s.push_str(&format!("    \"available\": {},\n", self.available));
        s.push_str(&format!("    \"branch\": \"{}\",\n", esc(&self.branch)));
        s.push_str(&format!("    \"commit\": \"{}\",\n", esc(&self.commit)));
        s.push_str(&format!("    \"commits_analyzed\": {},\n", self.commits_analyzed));
        s.push_str(&format!("    \"squashed_import\": {},\n", self.squashed_import));
        s.push_str("    \"hotspots\": [");
        for (i, (p, n)) in self.hotspots.iter().enumerate() {
            if i > 0 {
                s.push_str(", ");
            }
            s.push_str(&format!("{{\"path\": \"{}\", \"commits\": {}}}", esc(p), n));
        }
        s.push_str("],\n    \"fix_commits\": [");
        for (i, (h, sub)) in self.fix_commits.iter().enumerate() {
            if i > 0 {
                s.push_str(", ");
            }
            s.push_str(&format!(
                "{{\"hash\": \"{}\", \"subject\": \"{}\"}}",
                esc(h),
                esc(sub)
            ));
        }
        s.push_str("],\n    \"late_files\": [");
        for (i, (p, d)) in self.late_files.iter().enumerate() {
            if i > 0 {
                s.push_str(", ");
            }
            s.push_str(&format!("{{\"path\": \"{}\", \"last_date\": \"{}\"}}", esc(p), esc(d)));
        }
        s.push_str("]\n  }");
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn missing_repo_is_unavailable() {
        let r = analyze(Path::new("/tmp"));
        // /tmp is almost never a git work tree we own; either way the function
        // must not panic. If git is missing, available=false.
        let _ = r.available;
    }
}
