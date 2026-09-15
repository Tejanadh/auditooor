//! Function-level entry-point census.
//!
//! `surface` ranks *files*. This ranks *functions*: every public/external
//! mutating entry, tagged permissionless vs gated, with a value-flow hint.
//! That is the shortlist the fleet actually hunts — a file score cannot tell
//! you which function an unprivileged attacker can call.

use crate::detectors::blank_comments;

/// Access class for one entrypoint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Access {
    Permissionless,
    RoleGated,
    Admin,
}

impl Access {
    pub fn as_str(&self) -> &'static str {
        match self {
            Access::Permissionless => "permissionless",
            Access::RoleGated => "role-gated",
            Access::Admin => "admin",
        }
    }
}

/// One public/external mutating function.
#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    pub file: String,
    pub contract: String,
    pub name: String,
    pub line: usize,
    pub access: Access,
    pub modifiers: Vec<String>,
    /// `in` | `out` | `both` | `none`
    pub value_flow: String,
    pub payable: bool,
}

const ADMIN_MOD: &[&str] = &[
    "onlyowner",
    "only_owner",
    "onlyadmin",
    "only_admin",
    "onlygovernance",
    "onlygovernor",
];

const ROLE_MOD: &[&str] = &[
    "onlyrole",
    "only_role",
    "onlykeeper",
    "onlyoperator",
    "onlyminter",
    "onlypauser",
    "onlyrouter",
    "requiresauth",
    "auth",
];

const ADMIN_BODY: &[&str] = &[
    "msg.sender == owner",
    "msg.sender==owner",
    "msg.sender == admin",
    "msg.sender==admin",
    "hasrole(default_admin",
    "onlyowner",
];

const ROLE_BODY: &[&str] = &[
    "hasrole(",
    "has_role",
    "msg.sender ==",
    "msg.sender==",
    "require(msg.sender",
    "isauthorized",
    "is_authorized",
    "authorized[",
];

const IN_KW: &[&str] = &["deposit", "mint", "stake", "supply", "lock", "donate", "fund"];
const OUT_KW: &[&str] = &[
    "withdraw", "redeem", "burn", "unstake", "borrow", "liquidate", "sweep",
    "drain", "claim", "harvest", "transfer", "pay",
];

fn value_flow(name: &str, body_lc: &str, payable: bool) -> String {
    let n = name.to_lowercase();
    let mut inn = payable || IN_KW.iter().any(|k| n.contains(k) || body_lc.contains(k));
    let mut out = OUT_KW.iter().any(|k| n.contains(k) || body_lc.contains(k));
    // transfer is too common in ERC20 internals — only count as out if the name says so
    // or a native call{value is present.
    if body_lc.contains("call{value") || body_lc.contains(".call{") {
        out = true;
    }
    if n.contains("transferfrom") || n.contains("safetransferfrom") {
        inn = true;
        out = true;
    }
    match (inn, out) {
        (true, true) => "both".into(),
        (true, false) => "in".into(),
        (false, true) => "out".into(),
        (false, false) => "none".into(),
    }
}

fn classify_access(header_lc: &str, body_lc: &str, modifiers: &[String]) -> Access {
    let mods_joined = modifiers.join(" ").to_lowercase();
    if ADMIN_MOD.iter().any(|m| mods_joined.contains(m) || header_lc.contains(m))
        || ADMIN_BODY.iter().any(|p| body_lc.contains(p))
    {
        return Access::Admin;
    }
    if ROLE_MOD.iter().any(|m| mods_joined.contains(m) || header_lc.contains(m))
        || ROLE_BODY.iter().any(|p| body_lc.contains(p))
    {
        return Access::RoleGated;
    }
    Access::Permissionless
}

/// Pull identifier-like modifiers sitting between `)` and `{`/`returns`.
fn extract_modifiers(header_tail: &str) -> Vec<String> {
    let mut out = Vec::new();
    for tok in header_tail.split_whitespace() {
        let t = tok.trim_matches(|c: char| !c.is_alphanumeric() && c != '_');
        if t.is_empty() {
            continue;
        }
        let tl = t.to_lowercase();
        if matches!(
            tl.as_str(),
            "external"
                | "public"
                | "internal"
                | "private"
                | "view"
                | "pure"
                | "payable"
                | "virtual"
                | "override"
                | "returns"
                | "memory"
                | "calldata"
                | "storage"
        ) {
            continue;
        }
        if tl.starts_with("returns") {
            break;
        }
        out.push(t.to_string());
    }
    out
}

fn slice_body(src: &str, header_end: usize) -> &str {
    let bytes = src.as_bytes();
    let mut i = header_end;
    while i < bytes.len() && bytes[i] as char != '{' {
        i += 1;
    }
    if i >= bytes.len() {
        return "";
    }
    let start = i + 1;
    let mut depth = 1i32;
    i += 1;
    while i < bytes.len() {
        match bytes[i] as char {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return &src[start..i];
                }
            }
            _ => {}
        }
        i += 1;
    }
    &src[start..]
}

fn line_of(src: &str, byte: usize) -> usize {
    src[..byte.min(src.len())].bytes().filter(|&b| b == b'\n').count() + 1
}

fn next_contract_name(src: &str, from: usize) -> Option<(usize, String)> {
    let rel = src[from..].find("contract ")?;
    let start = from + rel + "contract ".len();
    let name: String = src[start..]
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .collect();
    if name.is_empty() {
        None
    } else {
        Some((from + rel, name))
    }
}

/// Census every mutating public/external function in a Solidity (or similar) file.
pub fn scan_entries(file: &str, src: &str) -> Vec<Entry> {
    let blanked = blank_comments(src);
    let bytes = blanked.as_bytes();
    let mut out = Vec::new();
    let mut contract = String::from("Unknown");
    let mut i = 0usize;
    if let Some((_, name)) = next_contract_name(&blanked, 0) {
        contract = name;
    }

    while let Some(rel) = blanked[i..].find("function ") {
        let kw = i + rel;
        if let Some((at, name)) = next_contract_name(&blanked, i) {
            if at < kw {
                contract = name;
            }
        }
        let start = kw + "function ".len();
        let mut j = start;
        let mut depth = 0i32;
        let mut header_end = None;
        while j < bytes.len() {
            match bytes[j] as char {
                '(' => depth += 1,
                ')' => depth -= 1,
                '{' | ';' if depth <= 0 => {
                    header_end = Some(j);
                    break;
                }
                _ => {}
            }
            j += 1;
        }
        let end = match header_end {
            Some(e) => e,
            None => break,
        };
        let header = &blanked[start..end];
        i = end + 1;

        let paren = match header.find('(') {
            Some(p) => p,
            None => continue,
        };
        let name = header[..paren].trim().to_string();
        if name.is_empty() || !name.chars().all(|c| c.is_alphanumeric() || c == '_') {
            continue;
        }
        let close = match header.find(')') {
            Some(c) => c,
            None => continue,
        };
        let tail = header[close + 1..].to_lowercase();
        let is_entry = tail.contains("external") || tail.contains("public");
        let is_readonly = tail.contains("view") || tail.contains("pure");
        if !is_entry || is_readonly {
            continue;
        }
        let payable = tail.contains("payable");
        let modifiers = extract_modifiers(&header[close + 1..]);
        let body = slice_body(&blanked, end);
        let body_lc = body.to_lowercase();
        let header_lc = header.to_lowercase();
        let access = classify_access(&header_lc, &body_lc, &modifiers);
        out.push(Entry {
            file: file.to_string(),
            contract: contract.clone(),
            name,
            line: line_of(&blanked, kw),
            access,
            modifiers,
            value_flow: value_flow(&header[..paren], &body_lc, payable),
            payable,
        });
    }
    out
}

/// Scan every in-scope Solidity file under `root`.
pub fn scan_repo_entries(root: &std::path::Path) -> Vec<Entry> {
    let mut all = Vec::new();
    for f in crate::walk_files(root) {
        if f.extension().and_then(|e| e.to_str()) != Some("sol") {
            continue;
        }
        let src = match std::fs::read_to_string(&f) {
            Ok(s) => s,
            Err(_) => continue,
        };
        let rel = f.strip_prefix(root).unwrap_or(&f).to_string_lossy().to_string();
        all.extend(scan_entries(&rel, &src));
    }
    all.sort_by(|a, b| {
        let rank = |e: &Entry| match e.access {
            Access::Permissionless => 0,
            Access::RoleGated => 1,
            Access::Admin => 2,
        };
        rank(a)
            .cmp(&rank(b))
            .then(a.file.cmp(&b.file))
            .then(a.line.cmp(&b.line))
    });
    all
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_vault_deposit_is_permissionless() {
        let src = include_str!("../fixtures/OpenVault.sol");
        let ents = scan_entries("OpenVault.sol", src);
        let dep = ents.iter().find(|e| e.name == "deposit").unwrap();
        assert_eq!(dep.access, Access::Permissionless);
        assert_eq!(dep.value_flow, "in");
        let drain = ents.iter().find(|e| e.name == "drainTo").unwrap();
        assert_eq!(drain.access, Access::Permissionless);
        assert_eq!(drain.value_flow, "out");
    }

    #[test]
    fn guarded_sweep_is_admin() {
        let src = include_str!("../fixtures/GuardedVault.sol");
        let ents = scan_entries("GuardedVault.sol", src);
        let sweep = ents.iter().find(|e| e.name == "sweep").unwrap();
        assert_eq!(sweep.access, Access::Admin);
        assert_eq!(sweep.value_flow, "out");
    }
}
