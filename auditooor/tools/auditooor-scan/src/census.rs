//! Repo census: toolchain, nSLOC, test/fuzz presence, protocol type.
//!
//! Pashov x-ray spends an LLM turn + a 9k bash script on this. We do it in the
//! same `xray` process so posture can see "no stateful fuzz" as a hunt signal.

use std::fs;
use std::path::{Path, PathBuf};

/// Deterministic census of a target tree.
#[derive(Debug, Clone, Default)]
pub struct Census {
    pub toolchain: String,
    pub nsloc: u32,
    pub test_files: u32,
    pub test_functions: u32,
    pub foundry_invariant: u32,
    pub stateless_fuzz: u32,
    pub echidna: u32,
    pub medusa: u32,
    pub fork_tests: u32,
    pub certora: u32,
    pub protocol_types: Vec<String>,
    /// Config present — we do not run slither (optional, if the hunter has it).
    pub slither_config: bool,
}

fn skip_dir(name: &str) -> bool {
    matches!(
        name,
        "node_modules" | "lib" | "libs" | "out" | "cache" | "artifacts" | "target" | ".git"
            | "dist" | "build" | "coverage" | "typechain" | "typechain-types" | "forge-std"
    )
}

fn walk_all(root: &Path) -> Vec<PathBuf> {
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
                if !skip_dir(&name) {
                    stack.push(p);
                }
            } else {
                out.push(p);
            }
        }
    }
    out
}

fn nsloc_file(src: &str) -> u32 {
    let mut n = 0u32;
    let mut in_block = false;
    for line in src.lines() {
        let t = line.trim();
        if in_block {
            if t.contains("*/") {
                in_block = false;
            }
            continue;
        }
        if t.starts_with("/*") {
            if !t.contains("*/") {
                in_block = true;
            }
            continue;
        }
        if t.is_empty() || t.starts_with("//") {
            continue;
        }
        n += 1;
    }
    n
}

fn count_occ(hay: &str, needle: &str) -> u32 {
    hay.matches(needle).count() as u32
}

/// Infer protocol types from concatenated lowercased source + entry names.
pub fn infer_types(blob_lc: &str) -> Vec<String> {
    let mut t = Vec::new();
    let vault = (blob_lc.contains("deposit") && blob_lc.contains("withdraw"))
        || blob_lc.contains("totalassets")
        || blob_lc.contains("convertsassets")
        || blob_lc.contains("previewdeposit");
    let lending = blob_lc.contains("liquidate")
        || (blob_lc.contains("borrow") && blob_lc.contains("repay"))
        || blob_lc.contains("collateral");
    let amm = blob_lc.contains("function swap")
        || blob_lc.contains("addliquidity")
        || blob_lc.contains("getamountout");
    let staking = blob_lc.contains("function stake") || blob_lc.contains("earned(");
    let escrow = blob_lc.contains("function lock") || blob_lc.contains("function release");
    if vault {
        t.push("vault".into());
    }
    if lending {
        t.push("lending".into());
    }
    if amm {
        t.push("amm".into());
    }
    if staking {
        t.push("staking".into());
    }
    if escrow {
        t.push("escrow".into());
    }
    if t.is_empty() {
        t.push("unknown".into());
    }
    t
}

pub fn analyze(root: &Path) -> Census {
    let mut c = Census::default();
    if root.join("foundry.toml").is_file() {
        c.toolchain = "foundry".into();
    } else if root.join("hardhat.config.js").is_file()
        || root.join("hardhat.config.ts").is_file()
    {
        c.toolchain = "hardhat".into();
    } else if root.join("Anchor.toml").is_file() {
        c.toolchain = "anchor".into();
    } else {
        c.toolchain = "unknown".into();
    }

    let mut blob = String::new();
    for f in crate::walk_files(root) {
        if let Ok(src) = fs::read_to_string(&f) {
            c.nsloc += nsloc_file(&src);
            blob.push_str(&src);
            blob.push('\n');
        }
    }
    c.protocol_types = infer_types(&blob.to_lowercase());
    c.slither_config = root.join("slither.config.json").is_file()
        || root.join(".solace").is_file()
        || root.join("slither.json").is_file();

    for f in walk_all(root) {
        let name = f.file_name().and_then(|n| n.to_str()).unwrap_or("");
        let path_lc = f.to_string_lossy().to_lowercase();
        let ext = f.extension().and_then(|e| e.to_str()).unwrap_or("");
        let in_test = path_lc.contains("/test") || path_lc.contains("\\test");
        if in_test && matches!(ext, "sol" | "js" | "ts" | "mjs" | "cjs") {
            c.test_files += 1;
        }
        if name == "echidna.yaml" || name.starts_with("echidna") && name.ends_with(".yaml") {
            c.echidna += 1;
        }
        if name == "medusa.json" {
            c.medusa += 1;
        }
        if name.ends_with(".spec") || name.ends_with(".cvl") {
            c.certora += 1;
        }
        if ext != "sol" && ext != "js" && ext != "ts" {
            continue;
        }
        let src = match fs::read_to_string(&f) {
            Ok(s) => s,
            Err(_) => continue,
        };
        c.test_functions += count_occ(&src, "function test") + count_occ(&src, "\nit(");
        c.foundry_invariant += count_occ(&src, "function invariant_");
        c.stateless_fuzz += count_occ(&src, "function testFuzz");
        c.echidna += count_occ(&src, "function echidna_");
        c.medusa += count_occ(&src, "function property_");
        if src.contains("createSelectFork") || src.contains("createFork") || src.contains("vm.createFork")
        {
            c.fork_tests += 1;
        }
    }
    c
}

impl Census {
    pub fn to_json(&self) -> String {
        use crate::json_escape as esc;
        let types: Vec<String> = self
            .protocol_types
            .iter()
            .map(|t| format!("\"{}\"", esc(t)))
            .collect();
        format!(
            "{{\n    \"toolchain\": \"{}\",\n    \"nsloc\": {},\n    \"test_files\": {},\n    \"test_functions\": {},\n    \"foundry_invariant\": {},\n    \"stateless_fuzz\": {},\n    \"echidna\": {},\n    \"medusa\": {},\n    \"fork_tests\": {},\n    \"certora\": {},\n    \"slither_config\": {},\n    \"protocol_types\": [{}]\n  }}",
            esc(&self.toolchain),
            self.nsloc,
            self.test_files,
            self.test_functions,
            self.foundry_invariant,
            self.stateless_fuzz,
            self.echidna,
            self.medusa,
            self.fork_tests,
            self.certora,
            self.slither_config,
            types.join(", ")
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vault_type_from_deposit_withdraw() {
        let t = infer_types("function deposit() function withdraw() totalassets");
        assert!(t.contains(&"vault".to_string()));
    }

    #[test]
    fn nsloc_skips_comments() {
        let src = "pragma solidity ^0.8.0;\n// hi\n\ncontract C {}\n";
        assert_eq!(nsloc_file(src), 2);
    }
}
