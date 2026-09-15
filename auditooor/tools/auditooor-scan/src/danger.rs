//! Danger-keyword census. Lifted from CritFindsAudit v2 Turn 2 — a cheap grep
//! that tells the fleet which files are hot before anyone reads source.
//! Hits are LEADs, never findings.

use crate::json_escape as esc;
use std::path::Path;

const KEYWORDS: &[&str] = &[
    "delegatecall",
    "selfdestruct",
    "tx.origin",
    "ecrecover",
    "abi.encodePacked",
    "call{value",
    "call{gas",
    ".send(",
    ".transfer(",
    "unchecked",
    "assembly",
    "tstore",
    "tload",
    "create2",
    "getReserves",
    "latestRoundData",
    "initialize",
    "_authorizeUpgrade",
    "upgradeToAndCall",
    "permit",
    "transferOwnership",
    "renounceOwnership",
    "swapCallback",
    "flashLoan",
    "onFlashLoan",
    "emergencyWithdraw",
    "receive()",
    "fallback()",
    "onlyOwner",
    "mulDown",
    "mulUp",
    "divDown",
    "divUp",
];

#[derive(Debug, Clone)]
pub struct Hit {
    pub file: String,
    pub line: usize,
    pub keyword: String,
}

pub fn scan(root: &Path) -> Vec<Hit> {
    let mut out = Vec::new();
    for f in crate::walk_files(root) {
        let src = match std::fs::read_to_string(&f) {
            Ok(s) => s,
            Err(_) => continue,
        };
        let rel = f.strip_prefix(root).unwrap_or(&f).to_string_lossy().to_string();
        let blanked = crate::detectors::blank_comments(&src);
        for (i, line) in blanked.lines().enumerate() {
            for kw in KEYWORDS {
                if line.contains(kw) {
                    out.push(Hit {
                        file: rel.clone(),
                        line: i + 1,
                        keyword: (*kw).into(),
                    });
                }
            }
        }
    }
    out
}

pub fn to_json(hits: &[Hit]) -> String {
    use std::collections::BTreeMap;
    let mut per_file: BTreeMap<String, u32> = BTreeMap::new();
    for h in hits {
        *per_file.entry(h.file.clone()).or_insert(0) += 1;
    }
    let mut hot: Vec<(String, u32)> = per_file.into_iter().collect();
    hot.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    hot.truncate(8);
    let mut s = String::from("{\n    \"hit_count\": ");
    s.push_str(&hits.len().to_string());
    s.push_str(",\n    \"hot_files\": [");
    for (i, (p, n)) in hot.iter().enumerate() {
        if i > 0 {
            s.push_str(", ");
        }
        s.push_str(&format!("{{\"path\": \"{}\", \"hits\": {}}}", esc(p), n));
    }
    s.push_str("],\n    \"hits\": [");
    for (i, h) in hits.iter().take(80).enumerate() {
        if i > 0 {
            s.push_str(", ");
        }
        s.push_str(&format!(
            "{{\"file\": \"{}\", \"line\": {}, \"keyword\": \"{}\"}}",
            esc(&h.file),
            h.line,
            esc(&h.keyword)
        ));
    }
    s.push_str("]\n  }");
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_vault_hits_call_value() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures");
        let hits = scan(&root);
        assert!(hits.iter().any(|h| h.keyword.contains("call{value") || h.file.contains("OpenVault")));
    }
}
