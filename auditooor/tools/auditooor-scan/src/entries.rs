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
    // `if (msg.sender != X) revert` is as much a guard as `require(msg.sender == X)`.
    "msg.sender !=",
    "msg.sender!=",
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

/// Any `onlySomething` modifier is a gate, whatever the project called it.
///
/// WEAK JOINT THIS CLOSES (field-reported, Fluid hunt 2026-09-19): the access
/// class was a fixed allowlist (onlyOwner/onlyRole/onlyKeeper/...), so
/// project-specific gates — `onlyRebalancer`, `onlyMultisig`, `onlyPauseAuth` —
/// fell through to `permissionless`. That put 74 fully-guarded functions on the
/// impact map of a live hunt. An impact map that says "permissionless" about a
/// guarded function is worse than no impact map: it aims the whole of Phase 2 at
/// nothing, and Abort B stops being able to abort.
fn gate_kind(m: &str) -> Option<Access> {
    let ml = m.to_lowercase();
    let namey = ml.strip_prefix("only").or_else(|| ml.strip_prefix("only_"));
    let is_gate = namey.is_some()
        || ml.contains("auth")
        || ml.contains("restricted")
        || ml.contains("permission")
        || ml.contains("guard")
        || ml.contains("hasrole")
        || ml.contains("requires");
    if !is_gate {
        return None;
    }
    let owner_ish = ["owner", "admin", "gov", "multisig", "timelock", "dao"]
        .iter()
        .any(|k| ml.contains(k));
    Some(if owner_ish { Access::Admin } else { Access::RoleGated })
}

/// Detect a guard written through a sender alias.
///
/// `ROLE_BODY` matches the literal `msg.sender ==` / `!=` forms. OpenZeppelin —
/// and anything using `Context` — writes the same guard as:
///
/// ```solidity
/// address caller = _msgSender();
/// if (caller != authority()) revert AccessManagedUnauthorized(caller);
/// ```
///
/// Found on the census stress corpus (2026-09-19): `AccessManaged.setAuthority`
/// censused as permissionless while being owner-gated. This resolves one level
/// of aliasing — the local a sender was assigned to — and requires a revert on
/// the same line, so an ordinary `_msgSender()` read (an ERC20 transfer) is not
/// mistaken for a guard.
/// The identifier adjacent to a comparison operator.
///
/// `side` is the text before (`from_right = false`) or after (`from_right = true`)
/// the operator. Handles the no-arg call form, because `pendingOwner() != sender`
/// / `owner() == msg.sender` is the most common guard shape in Solidity and a
/// scanner that stops at the `)` loses the identifier entirely.
fn adjacent_ident(side: &str, from_right: bool) -> String {
    let ident = |it: &mut dyn Iterator<Item = char>| -> String {
        it.take_while(|c| c.is_alphanumeric() || *c == '_' || *c == '.').collect()
    };
    if from_right {
        let mut it = side.chars().skip_while(|c| c.is_whitespace());
        ident(&mut it)
    } else {
        let rev: Vec<char> = side.chars().rev().skip_while(|c| c.is_whitespace()).collect();
        // strip a trailing `()` of a no-arg call before reading the name
        let rest: Vec<char> = if rev.len() >= 2 && rev[0] == ')' && rev[1] == '(' {
            rev[2..].to_vec()
        } else {
            rev
        };
        let mut it = rest.into_iter();
        let t = ident(&mut it);
        t.chars().rev().collect()
    }
}

fn aliased_sender_guard(body_lc: &str, params: &[String]) -> bool {
    let mut aliases: Vec<String> = vec!["msg.sender".into(), "_msgsender()".into()];
    for line in body_lc.lines() {
        if let Some(eq) = line.find('=') {
            let rhs = &line[eq + 1..];
            if rhs.contains("msg.sender") || rhs.contains("_msgsender()") {
                let lhs = line[..eq].trim();
                if let Some(name) = lhs.split_whitespace().last() {
                    let n = name.trim_matches(|c: char| !c.is_alphanumeric() && c != '_');
                    if !n.is_empty() && n != "msg" {
                        aliases.push(n.to_string());
                    }
                }
            }
        }
    }
    // The alias must be an *operand* of the comparison, not merely present on the
    // line. `if (allowance[owner][spender] != max)` mentions `owner` and is not a
    // sender check; over-matching it falsely gated ERC20 transferFrom and, worse,
    // Fluid's permissionless `operate` — a false gate now aborts a good target.
    let operand_is_alias = |side: &str, from_right: bool| -> bool {
        let norm = |x: &str| x.replace(['(', ')'], "").trim().to_string();
        let tok = norm(&adjacent_ident(side, from_right));
        !tok.is_empty() && aliases.iter().any(|a| norm(a) == tok)
    };

    // A sender comparison is only an AUTHORIZATION gate when the other side is
    // stored state (an owner, an authority, a constant). Corpus findings
    // (2026-09-19):
    //   ERC20Wrapper.depositFor  `if (sender == address(this)) revert` — a
    //     sanity check; anyone may call it.
    //   ERC3009.receiveWithAuthorization  compares the sender to a *parameter*;
    //     an attacker simply passes their own address.
    // Calling either a gate is a false gate, and a false gate aborts a live
    // target that was worth hunting.
    let not_authority = |x: &str| -> bool {
        let t = x.replace(['(', ')'], "").trim().to_string();
        t.is_empty()
            || t.starts_with("address")
            || t.starts_with('0')
            || t.chars().all(|c| c.is_ascii_digit())
            || params.iter().any(|p| p.to_lowercase() == t)
    };
    let other_side = |side: &str, from_right: bool| -> String { adjacent_ident(side, from_right) };
    for line in body_lc.lines() {
        let enforces = line.contains("revert") || line.contains("require(") || line.contains("if (");
        if !enforces {
            continue;
        }
        for op in ["!=", "=="] {
            if let Some(i) = line.find(op) {
                let (l, r) = (&line[..i], &line[i + 2..]);
                let gate = (operand_is_alias(l, false) && !not_authority(&other_side(r, true)))
                    || (operand_is_alias(r, true) && !not_authority(&other_side(l, false)));
                if gate {
                    return true;
                }
            }
        }
    }
    false
}

/// Identifier names of a function's own parameters — a sender compared against
/// one of these is not an authorization gate, since the caller chooses it.
fn param_names(header: &str) -> Vec<String> {
    let open = match header.find('(') {
        Some(i) => i,
        None => return Vec::new(),
    };
    let close = header[open..].find(')').map(|i| open + i).unwrap_or(header.len());
    header[open + 1..close]
        .split(',')
        .filter_map(|chunk| {
            chunk
                .split_whitespace()
                .last()
                .map(|t| t.trim_matches(|c: char| !c.is_alphanumeric() && c != '_').to_lowercase())
        })
        .filter(|t| !t.is_empty())
        .collect()
}

fn classify_access(header_lc: &str, body_lc: &str, modifiers: &[String], params: &[String]) -> Access {
    // A named gate modifier wins over every keyword list below.
    let mut gated: Option<Access> = None;
    for m in modifiers {
        match gate_kind(m) {
            Some(Access::Admin) => return Access::Admin,
            Some(a) => gated = Some(a),
            None => {}
        }
    }
    if let Some(a) = gated {
        return a;
    }
    // Only the extracted modifier list and the body are evidence. Substring-
    // matching the raw header was a false-gate factory: a return value or
    // parameter named `authority` contains "auth", and `gate` now exits non-zero
    // on ABORT, so a false gate silently kills a good target.
    let _ = header_lc;
    let mods_joined = modifiers.join(" ").to_lowercase();
    if ADMIN_MOD.iter().any(|m| mods_joined.contains(m))
        || ADMIN_BODY.iter().any(|p| body_lc.contains(p))
    {
        return Access::Admin;
    }
    if ROLE_MOD.iter().any(|m| mods_joined.contains(m))
        || ROLE_BODY.iter().any(|p| body_lc.contains(p))
        || aliased_sender_guard(body_lc, params)
    {
        // An inline `msg.sender != TEAM_MULTISIG` guard is an admin gate, not a
        // role gate — classify by what the body compares against, the same way
        // `gate_kind` classifies by what the modifier is named.
        let owner_ish = ["multisig", "owner", "admin", "governance", "governor", "timelock", "dao"]
            .iter()
            .any(|k| body_lc.contains(k));
        return if owner_ish { Access::Admin } else { Access::RoleGated };
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
        // `returns` ends the modifier list. This must be checked BEFORE the skip
        // list below, which also contains "returns" and would `continue` past it
        // — that bug let the whole return tuple be collected as modifiers
        // (`operate` came back with mods ["reentrancy","uint256","memVar3_"]).
        // Harmless while modifiers only ranked attention; a false gate now, since
        // a return value named `authority` would read as a guard and `gate`
        // exits non-zero on ABORT.
        if tl == "returns" {
            break;
        }
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

/// One declared type and the byte range of its body.
struct TypeBlock {
    name: String,
    is_interface: bool,
    start: usize,
    end: usize,
}

/// Find every `contract` / `abstract contract` / `interface` / `library` block
/// with its brace range, so a function can be attributed to the type that
/// actually *encloses* it rather than to the last declaration seen above it.
///
/// The old "last `contract ` keyword wins" rule named `Constants.rebalance` for
/// a function defined in `FluidBufferRateHandler`, because `abstract contract
/// Constants` also contains the substring `contract `.
fn type_blocks(src: &str) -> Vec<TypeBlock> {
    let bytes = src.as_bytes();
    let mut out = Vec::new();
    for (kw, is_interface) in [("interface ", true), ("contract ", false), ("library ", false)] {
        let mut from = 0usize;
        while let Some(rel) = src[from..].find(kw) {
            let at = from + rel;
            from = at + kw.len();
            // must start a word (avoid `abstractcontract`, identifiers, strings)
            if at > 0 {
                let prev = bytes[at - 1] as char;
                if prev.is_alphanumeric() || prev == '_' {
                    continue;
                }
            }
            let name: String = src[from..]
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            if name.is_empty() {
                continue;
            }
            // walk to the opening brace of the body, then match it
            let mut i = from + name.len();
            while i < bytes.len() && bytes[i] as char != '{' && bytes[i] as char != ';' {
                i += 1;
            }
            if i >= bytes.len() || bytes[i] as char == ';' {
                continue;
            }
            let start = i;
            let mut depth = 0i32;
            while i < bytes.len() {
                match bytes[i] as char {
                    '{' => depth += 1,
                    '}' => {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    }
                    _ => {}
                }
                i += 1;
            }
            out.push(TypeBlock { name, is_interface, start, end: i.min(bytes.len()) });
        }
    }
    out
}

/// The innermost type block containing `at`.
fn enclosing_type(blocks: &[TypeBlock], at: usize) -> Option<&TypeBlock> {
    blocks
        .iter()
        .filter(|b| at > b.start && at < b.end)
        .min_by_key(|b| b.end - b.start)
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
    let blocks = type_blocks(&blanked);
    let mut i = 0usize;

    while let Some(rel) = blanked[i..].find("function ") {
        let kw = i + rel;
        let encl = enclosing_type(&blocks, kw);
        let contract = encl.map(|b| b.name.clone()).unwrap_or_else(|| "Unknown".into());
        let in_interface = encl.map(|b| b.is_interface).unwrap_or(false);
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
        // A bodiless function is a *declaration*, not an entry point: interface
        // members and abstract stubs describe the callee, not this contract.
        let declaration_only = bytes[end] as char == ';';
        let header = &blanked[start..end];
        i = end + 1;
        if declaration_only || in_interface {
            continue;
        }

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
        let params = param_names(&header_lc);
        let access = classify_access(&header_lc, &body_lc, &modifiers, &params);
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
mod project_gate_tests {
    use super::*;

    fn census() -> Vec<Entry> {
        let src = std::fs::read_to_string(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures/ProjectGated.sol"),
        )
        .unwrap();
        scan_entries("ProjectGated.sol", &src)
    }

    #[test]
    fn interface_members_are_not_entry_points() {
        // Regression, Fluid hunt 2026-09-19: bodiless interface members were
        // counted as callable entry points on the hunted contract.
        let e = census();
        assert!(
            !e.iter().any(|x| x.name == "updateFeeAndRevenueCut" || x.name == "setDexFee"),
            "interface declarations must not appear in the census: {:?}",
            e.iter().map(|x| &x.name).collect::<Vec<_>>()
        );
    }

    #[test]
    fn functions_are_attributed_to_the_enclosing_contract() {
        // Regression: `abstract contract Constants` above the real contract used
        // to claim every function below it.
        let e = census();
        let r = e.iter().find(|x| x.name == "rebalance").expect("rebalance missing");
        assert_eq!(r.contract, "FluidRateHandler");
    }

    #[test]
    fn project_specific_only_modifiers_are_gates() {
        // Regression: onlyRebalancer/onlyMultisig are not in any allowlist, and
        // used to classify as permissionless — which put 74 guarded functions on
        // a live hunt's impact map.
        let e = census();
        let by = |n: &str| e.iter().find(|x| x.name == n).unwrap().access.clone();
        assert_eq!(by("rebalance"), Access::RoleGated);
        assert_eq!(by("setRate"), Access::Admin);
        assert_eq!(by("setPauseContract"), Access::Admin, "inline msg.sender != guard");
        assert_eq!(by("donate"), Access::Permissionless, "this one really is open");
    }

    #[test]
    fn returns_tuple_names_are_not_modifiers() {
        // Regression: `returns` was in the skip list and `continue`d past the
        // break, so the return tuple was collected as modifiers. With `gate`
        // exiting non-zero on ABORT, a return value named `authority` would
        // abort a perfectly good target.
        let e = census();
        let swap = e.iter().find(|x| x.name == "swap").expect("swap missing");
        assert_eq!(swap.access, Access::Permissionless, "mods were {:?}", swap.modifiers);
        assert!(
            !swap.modifiers.iter().any(|m| m == "authority" || m == "onlyOut"),
            "return names leaked into modifiers: {:?}",
            swap.modifiers
        );
        let reb = e.iter().find(|x| x.name == "rebalance").unwrap();
        assert_eq!(reb.modifiers, vec!["onlyRebalancer".to_string()]);
    }

    #[test]
    fn sender_aliased_into_a_local_is_still_a_guard() {
        // Corpus regression (OpenZeppelin AccessManaged.setAuthority): the guard
        // is `caller = _msgSender(); if (caller != authority()) revert`.
        let e = census();
        let g = e.iter().find(|x| x.name == "setAuthority").unwrap();
        assert_ne!(g.access, Access::Permissionless, "aliased sender guard missed");
        // ...but merely reading the sender is not a guard.
        let open = e.iter().find(|x| x.name == "selfRegister").unwrap();
        assert_eq!(open.access, Access::Permissionless, "reading _msgSender() is not a gate");
    }

    #[test]
    fn no_arg_call_on_the_left_of_the_comparison_is_a_guard() {
        // Corpus regression (OpenZeppelin Ownable2Step.acceptOwnership):
        // `if (pendingOwner() != sender) revert`. A scanner that stops at the
        // `)` of the call loses the identifier and calls this permissionless.
        let e = census();
        let a = e.iter().find(|x| x.name == "acceptOwnership").unwrap();
        assert_ne!(a.access, Access::Permissionless, "call-form guard missed");
    }

    #[test]
    fn exactly_one_function_here_is_permissionless() {
        let e = census();
        let open: Vec<_> = e
            .iter()
            .filter(|x| x.access == Access::Permissionless)
            .map(|x| x.name.as_str())
            .collect();
        assert_eq!(open, vec!["donate", "swap", "selfRegister"]);
    }
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
