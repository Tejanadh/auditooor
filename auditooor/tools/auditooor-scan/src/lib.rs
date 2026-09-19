//! auditooor-scan core library.
//!
//! Fast, deterministic, pure-std pre-filter for the auditooor audit stack.
//! Three jobs the LLM should never spend tokens on:
//!   1. `classify_ecosystem` / `walk_files` — route a repo to the right engine.
//!   2. `scan_file` / money-proximity scoring — compute the impact map, ranked.
//!   3. `fingerprint` + ledger — real dedup for the novelty gate.

use std::fs;
use std::path::{Path, PathBuf};

pub mod harness;
pub mod detectors;
pub mod accounting;
pub mod novelty;
pub mod corpus;
pub mod ev;
pub mod seams;
pub mod outcome;
pub mod entries;
pub mod git;
pub mod xray;
pub mod census;
pub mod props;
pub mod pack;
pub mod deltas;
pub mod danger;
pub mod known;

#[cfg(feature = "solar")]
pub mod ast;

// ---------------------------------------------------------------------------
// Ecosystem detection
// ---------------------------------------------------------------------------

/// Classify a source file into an ecosystem the router understands.
/// Returns `None` for files with no security-relevant ecosystem.
pub fn classify_ecosystem(path: &Path, content: &str) -> Option<String> {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    let lc = content.to_lowercase();
    match ext {
        "sol" => Some("evm".to_string()),
        "vy" | "vyi" => Some("vyper".to_string()),
        "circom" => Some("zk".to_string()),
        "move" => Some("move".to_string()),
        "rs" => {
            // Rust splits three ways: Solana program, ZK circuit, or generic.
            if lc.contains("declare_id!")
                || lc.contains("#[program]")
                || lc.contains("anchor_lang")
                || lc.contains("solana_program")
            {
                Some("solana".to_string())
            } else if lc.contains("plonky2")
                || lc.contains("halo2")
                || lc.contains("arkworks")
                || lc.contains("ark_")
                || lc.contains("bellman")
                || lc.contains("circuit")
            {
                Some("zk".to_string())
            } else {
                Some("rust".to_string())
            }
        }
        _ => None,
    }
}

/// Directories never worth scanning (tests, deps, build artifacts).
const EXCLUDED_DIRS: &[&str] = &[
    "test", "tests", "lib", "libs", "mock", "mocks", "node_modules", "out",
    "cache", "artifacts", "target", ".git", "script", "scripts", "interfaces",
    "dist", "build", "coverage", ".github",
];

const SOURCE_EXTS: &[&str] = &["sol", "vy", "vyi", "rs", "move", "circom"];

fn is_excluded_dir(name: &str) -> bool {
    EXCLUDED_DIRS.contains(&name)
}

fn is_test_file(name: &str) -> bool {
    let lc = name.to_lowercase();
    lc.ends_with(".t.sol")
        || lc.contains("test")
        || lc.contains("mock")
        || lc.ends_with("_test.rs")
        || lc.ends_with("_tests.rs")
}

/// Recursively collect in-scope source files under `root`, applying the same
/// exclude discipline the crit* engines use.
pub fn walk_files(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let entries = match fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let p = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            if p.is_dir() {
                if !is_excluded_dir(&name) {
                    stack.push(p);
                }
            } else {
                let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("");
                if SOURCE_EXTS.contains(&ext) && !is_test_file(&name) {
                    out.push(p);
                }
            }
        }
    }
    out.sort();
    out
}

// ---------------------------------------------------------------------------
// Money-proximity impact map
// ---------------------------------------------------------------------------

/// A single value-moving site inside a file.
#[derive(Debug, Clone, PartialEq)]
pub struct ValueSite {
    pub line: usize,
    pub keyword: String,
    /// Heuristic: no obvious sender/role guard within a small window.
    pub permissionless_hint: bool,
    pub weight: u32,
    /// How deep/branchy the path around this value site is. Per the research
    /// (arXiv 2411.17343) complexity is a *complementary* signal — it multiplies
    /// money-proximity, it does not rank on its own. zhero: "live bugs are simple,
    /// they hide in complex paths."
    pub path_complexity: u32,
}

/// Per-file scan result: money-proximity plus complexity-weighted risk.
#[derive(Debug, Clone)]
pub struct FileScan {
    pub path: String,
    pub ecosystem: String,
    pub entrypoints: u32,
    pub value_sites: Vec<ValueSite>,
    /// Pure money-proximity (entrypoints + weighted value moves). Backward-compatible.
    pub score: u32,
    /// Total decision/branch tokens in the file — a size+complexity proxy.
    pub complexity: u32,
    /// Count of SIG/AC detector leads in this file (EVM only) — flat-class coverage.
    pub detector_hits: u32,
    /// The ranking key (additive): money-proximity + path-complexity + detector leads.
    pub risk_score: u32,
}

/// High-severity value-movement keywords (weight 5). Matched case-insensitively.
const HIGH_KW: &[&str] = &[
    "transferfrom", "safetransfer", "transfer", "mint", "burn", "withdraw",
    "redeem", "liquidate", "delegatecall", "selfdestruct", "call{value",
    ".call{", "flashloan", "sweep", "borrow", "repay", "settransfer",
];

/// Lower-severity value/flow keywords (weight 2).
const LOW_KW: &[&str] = &[
    "deposit", "swap", "claim", "send", "migrate", "stake", "unstake",
    "harvest", "collect", "rebalance", "settle",
];

/// Guard tokens that, if near a value site, lower its risk (drop permissionless hint).
const GUARD_KW: &[&str] = &[
    "onlyowner", "only_owner", "msg.sender ==", "msg.sender==",
    "require(msg.sender", "assert msg.sender", "has_role", "hasrole",
    "has_one", "onlyrole", "only_role", "accesscontrol", "signer",
    "is_authorized", "authorized", "restricted", "onlyadmin", "only_admin",
];

/// Entrypoint signals per ecosystem, matched on a lowercased line.
fn is_entrypoint_line(ecosystem: &str, line_lc: &str) -> bool {
    match ecosystem {
        "evm" => {
            line_lc.contains("function ")
                && (line_lc.contains(" external") || line_lc.contains(" public"))
        }
        "vyper" => line_lc.contains("@external") || line_lc.contains("@payable"),
        "solana" => line_lc.trim_start().starts_with("pub fn "),
        "move" => {
            line_lc.contains("public entry fun")
                || line_lc.contains("entry fun")
                || line_lc.contains("public fun")
        }
        "zk" | "rust" => {
            line_lc.trim_start().starts_with("pub fn ")
                || line_lc.contains("#[external")
        }
        _ => false,
    }
}

/// Match a keyword list against a lowercased line; return the matched keyword.
fn first_match<'a>(line_lc: &str, kws: &'a [&'a str]) -> Option<&'a str> {
    kws.iter().copied().find(|kw| line_lc.contains(kw))
}

/// Branch / decision tokens — the cyclomatic-complexity proxy.
const BRANCH_KW: &[&str] = &[
    "if(", "if (", "require(", "assert(", "for(", "for (", "while(", "while (",
    "} else", "else if", "&&", "||", "revert", "catch", "assembly",
];

/// External-call markers — each hop out of the contract adds attack surface/depth.
const EXT_CALL_KW: &[&str] = &[".call", ".delegatecall", ".staticcall", ".call{"];

fn count_matches(line_lc: &str, kws: &[&str]) -> u32 {
    kws.iter().map(|kw| line_lc.matches(kw).count() as u32).sum()
}

/// Per-line brace-nesting depth (depth *entering* each line).
fn nesting_depths(lower: &[String]) -> Vec<u32> {
    let mut out = Vec::with_capacity(lower.len());
    let mut d: i32 = 0;
    for l in lower {
        out.push(d.max(0) as u32);
        let opens = l.matches('{').count() as i32;
        let closes = l.matches('}').count() as i32;
        d += opens - closes;
    }
    out
}

/// Scan one file's content into a `FileScan`. Deterministic and allocation-light.
pub fn scan_file(path: &str, ecosystem: &str, content: &str) -> FileScan {
    let lines: Vec<&str> = content.lines().collect();
    let lower: Vec<String> = lines.iter().map(|l| l.to_lowercase()).collect();

    let depths = nesting_depths(&lower);
    let mut entrypoints = 0u32;
    let mut complexity = 0u32;
    let mut value_sites = Vec::new();

    for (i, ll) in lower.iter().enumerate() {
        if is_entrypoint_line(ecosystem, ll) {
            entrypoints += 1;
        }
        complexity += count_matches(ll, BRANCH_KW);
        let (kw, weight) = if let Some(k) = first_match(ll, HIGH_KW) {
            (Some(k), 5u32)
        } else if let Some(k) = first_match(ll, LOW_KW) {
            (Some(k), 2u32)
        } else {
            (None, 0)
        };
        if let Some(k) = kw {
            // Window around the site for guards, branchiness and external hops.
            let lo = i.saturating_sub(6);
            let hi = (i + 6).min(lower.len().saturating_sub(1));
            let window = &lower[lo..=hi];
            let guarded = window
                .iter()
                .any(|w| GUARD_KW.iter().any(|g| w.contains(g)));
            let branchiness: u32 = window.iter().map(|w| count_matches(w, BRANCH_KW)).sum();
            let ext_hops: u32 = window.iter().map(|w| count_matches(w, EXT_CALL_KW)).sum();
            // path_complexity: how deep and branchy the money-touch is buried.
            // Capped so complexity complements money-proximity, never swamps it.
            let pc = (depths[i] * 2 + branchiness + ext_hops * 2).min(20);
            value_sites.push(ValueSite {
                line: i + 1,
                keyword: k.to_string(),
                permissionless_hint: !guarded,
                weight,
                path_complexity: pc,
            });
        }
    }

    // Money-proximity (backward-compatible): weight + permissionless bonus + entrypoints.
    let mut score = entrypoints;
    for vs in &value_sites {
        score += vs.weight + if vs.permissionless_hint { 3 } else { 0 };
    }

    // High-precision detector leads (SIG/AC) folded into ranking. This is the fix
    // for the flat-bug blind spot: an unprotected initializer has NO value-keyword
    // site and near-zero complexity, so money-proximity would bury it — yet it is
    // the #1 loss class. A CRITICAL/HIGH lead forces the file to the top regardless.
    let (detector_bonus, detector_hits) = if ecosystem == "evm" {
        let ds = crate::detectors::scan_all(content);
        let bonus: u32 = ds
            .iter()
            .map(|d| match d.severity.as_str() {
                "CRITICAL" => 12,
                "HIGH" => 7,
                _ => 3,
            })
            .sum();
        (bonus, ds.len() as u32)
    } else {
        (0, 0)
    };

    // Risk is ADDITIVE, not multiplicative: money-proximity + path-complexity bonus
    // + detector leads. Additive (not a product) so a flat file is never zeroed by
    // complexity=0; complexity complements money-proximity, and detector leads cover
    // the flat classes (AC/SIG) that carry no value-keyword signal at all.
    let mut risk_score = entrypoints + detector_bonus;
    for vs in &value_sites {
        risk_score += vs.weight + if vs.permissionless_hint { 3 } else { 0 } + vs.path_complexity;
    }

    FileScan {
        detector_hits,
        path: path.to_string(),
        ecosystem: ecosystem.to_string(),
        entrypoints,
        value_sites,
        score,
        complexity,
        risk_score,
    }
}

/// Repo-level maturity verdict → the tactic to hunt with.
/// zhero: sloppy codebases → low-hanging fruit; clean/complex → deepest paths.
pub fn repo_maturity(scans: &[FileScan]) -> (u32, &'static str) {
    let total_complexity: u32 = scans.iter().map(|s| s.complexity).sum();
    let files = scans.len().max(1) as u32;
    let avg = total_complexity / files;
    let tactic = if avg >= 12 {
        "MATURE/COMPLEX — hunt the deepest, branchiest value paths (highest risk_score); the bug is a simple missing check nobody read that far to find"
    } else if avg >= 4 {
        "MIXED — start at top risk_score, but sweep the flat entrypoints for a missing guard too"
    } else {
        "SLOPPY/SIMPLE — hunt low-hanging fruit: unguarded entrypoints, missing checks, obvious access-control gaps"
    };
    (total_complexity, tactic)
}

/// Scan a whole repo: returns per-file scans sorted by score descending.
pub fn scan_repo(root: &Path) -> Vec<FileScan> {
    let mut scans = Vec::new();
    for f in walk_files(root) {
        let content = match fs::read_to_string(&f) {
            Ok(c) => c,
            Err(_) => continue,
        };
        if let Some(eco) = classify_ecosystem(&f, &content) {
            let rel = f.strip_prefix(root).unwrap_or(&f).to_string_lossy().to_string();
            scans.push(scan_file(&rel, &eco, &content));
        }
    }
    scans.sort_by(|a, b| b.risk_score.cmp(&a.risk_score).then(a.path.cmp(&b.path)));
    scans
}

// ---------------------------------------------------------------------------
// Novelty-gate fingerprinting
// ---------------------------------------------------------------------------

/// FNV-1a 64-bit — stable, dependency-free, good enough for dedup keys.
fn fnv1a64(s: &str) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

fn normalize(s: &str) -> String {
    s.trim().to_lowercase().split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Stable candidate fingerprint from (mechanism, value-sink, entrypoint).
/// Two candidates with the same normalized triple collide by design — that is
/// the dedup signal the novelty gate keys on.
pub fn fingerprint(mechanism: &str, sink: &str, entrypoint: &str) -> String {
    let key = format!(
        "{}|{}|{}",
        normalize(mechanism),
        normalize(sink),
        normalize(entrypoint)
    );
    format!("{:016x}", fnv1a64(&key))
}

/// Result of checking a fingerprint against a ledger file.
#[derive(Debug, PartialEq)]
pub enum LedgerStatus {
    Novel,
    Duplicate,
}

/// Check whether `fp` already appears in the ledger at `ledger_path`.
/// A missing ledger file is treated as empty (everything novel).
pub fn ledger_check(ledger_path: &Path, fp: &str) -> LedgerStatus {
    match fs::read_to_string(ledger_path) {
        Ok(body) => {
            if body.lines().any(|l| l.split_whitespace().next() == Some(fp)) {
                LedgerStatus::Duplicate
            } else {
                LedgerStatus::Novel
            }
        }
        Err(_) => LedgerStatus::Novel,
    }
}

/// Append a fingerprint + label to the ledger, creating it if needed.
pub fn ledger_add(ledger_path: &Path, fp: &str, label: &str) -> std::io::Result<()> {
    use std::io::Write;
    if let Some(parent) = ledger_path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let mut f = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(ledger_path)?;
    writeln!(f, "{}\t{}", fp, label.replace('\n', " "))
}

// ---------------------------------------------------------------------------
// Minimal JSON emission (no serde — stays offline / dependency-free)
// ---------------------------------------------------------------------------

/// Escape a string for embedding in JSON.
pub fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn detects_solidity() {
        assert_eq!(
            classify_ecosystem(Path::new("A.sol"), "pragma solidity ^0.8;"),
            Some("evm".to_string())
        );
    }

    #[test]
    fn detects_vyper_and_move_and_circom() {
        assert_eq!(classify_ecosystem(Path::new("a.vy"), "@external\ndef f():"), Some("vyper".into()));
        assert_eq!(classify_ecosystem(Path::new("m.move"), "module x {}"), Some("move".into()));
        assert_eq!(classify_ecosystem(Path::new("c.circom"), "template T(){}"), Some("zk".into()));
    }

    #[test]
    fn rust_splits_three_ways() {
        assert_eq!(classify_ecosystem(Path::new("p.rs"), "declare_id!(\"x\");"), Some("solana".into()));
        assert_eq!(classify_ecosystem(Path::new("z.rs"), "use plonky2::field;"), Some("zk".into()));
        assert_eq!(classify_ecosystem(Path::new("u.rs"), "fn helper() {}"), Some("rust".into()));
    }

    #[test]
    fn ignores_non_source() {
        assert_eq!(classify_ecosystem(Path::new("README.md"), "hello"), None);
    }

    #[test]
    fn permissionless_value_site_scores_higher() {
        let guarded = "function f() external {\n  require(msg.sender == owner);\n  token.transfer(a, b);\n}";
        let open = "function f() external {\n  token.transfer(a, b);\n}";
        let g = scan_file("g.sol", "evm", guarded);
        let o = scan_file("o.sol", "evm", open);
        assert!(o.score > g.score, "open {} should beat guarded {}", o.score, g.score);
        assert!(o.value_sites[0].permissionless_hint);
        assert!(!g.value_sites[0].permissionless_hint);
    }

    #[test]
    fn counts_entrypoints() {
        let src = "function a() external {}\nfunction b() public {}\nfunction c() internal {}";
        let s = scan_file("x.sol", "evm", src);
        assert_eq!(s.entrypoints, 2);
    }

    #[test]
    fn fingerprint_is_stable_and_normalized() {
        let a = fingerprint("Rounding Direction", "vault.shares", "deposit()");
        let b = fingerprint("  rounding   direction ", "VAULT.SHARES", "Deposit()");
        assert_eq!(a, b, "normalization must collapse case/whitespace");
        assert_eq!(a.len(), 16);
        let c = fingerprint("reentrancy", "vault.shares", "deposit()");
        assert_ne!(a, c);
    }

    #[test]
    fn json_escape_handles_specials() {
        assert_eq!(json_escape("a\"b\\c\n"), "a\\\"b\\\\c\\n");
    }
}
