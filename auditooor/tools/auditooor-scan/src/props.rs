//! Mechanical invariant seeds. Not findings — starting points for `/auditooor FUZZ`.
//!
//! Pashov fizz asks five LLM agents to invent properties from scratch. We first
//! emit the properties the *code shape* already implies (pairs, aggregates,
//! lifted guards) so the agents spend tokens on protocol-specific economics,
//! not rediscovering that deposit↔withdraw exists.

use crate::entries::{Access, Entry};
use crate::json_escape as esc;

const PAIRS: &[(&str, &str, &str)] = &[
    ("deposit", "withdraw", "round-trip: withdraw after deposit returns ≤ in, absent documented fees"),
    ("mint", "burn", "supply: minted − burned conserves totalSupply"),
    ("stake", "unstake", "round-trip: unstake after stake returns ≤ staked, absent documented yield"),
    ("borrow", "repay", "debt: repay cannot under-account vs borrow at this rounding step"),
    ("lock", "unlock", "lock: unlocked ≤ locked for the same id/user"),
    ("open", "close", "position: close cannot extract more than open posted"),
    ("supply", "redeem", "round-trip: redeem after supply returns ≤ in"),
    ("addliquidity", "removeliquidity", "AMM: remove after add returns ≤ in, absent fees"),
];

/// One seed the synthesizer must keep, rewrite, or drop with a reason.
#[derive(Debug, Clone)]
pub struct PropSeed {
    pub id: String,
    pub kind: String,
    pub statement: String,
    pub evidence: String,
    pub guarantee: String,
}

fn names_of(entries: &[Entry]) -> Vec<String> {
    entries.iter().map(|e| e.name.to_lowercase()).collect()
}

fn has_fn(names: &[String], stem: &str) -> bool {
    names.iter().any(|n| n.contains(stem))
}

/// Seeds from an entry census + concatenated source (lowercased) + per-file deltas.
pub fn seeds(entries: &[Entry], src_lc: &str, deltas: &[crate::deltas::Delta]) -> Vec<PropSeed> {
    let mut out = Vec::new();
    let names = names_of(entries);
    let mut n = 1u32;

    for (a, b, stmt) in PAIRS {
        if has_fn(&names, a) && has_fn(&names, b) {
            let perm = entries.iter().any(|e| {
                e.access == Access::Permissionless
                    && (e.name.to_lowercase().contains(a) || e.name.to_lowercase().contains(b))
            });
            out.push(PropSeed {
                id: format!("P-{n:02}"),
                kind: "pair".into(),
                statement: stmt.to_string(),
                evidence: format!("{a}↔{b} both present; permissionless={perm}"),
                guarantee: "EXPLORATORY".into(),
            });
            n += 1;
        }
    }

    const AGG: &[&str] = &[
        "totalsupply",
        "totalassets",
        "totalborrowed",
        "totaldebt",
        "totalstaked",
        "cash",
        "shares",
    ];
    for ag in AGG {
        if src_lc.contains(ag) {
            out.push(PropSeed {
                id: format!("P-{n:02}"),
                kind: "aggregate".into(),
                statement: format!("{ag} == Σ parts across every write site (flag any function that writes one side only)"),
                evidence: format!("identifier `{ag}` in source"),
                guarantee: "EXPLORATORY".into(),
            });
            n += 1;
        }
    }

    let perm_value = entries.iter().any(|e| {
        e.access == Access::Permissionless && e.value_flow != "none"
    });
    // One-sided delta functions: the x-ray v2 signal. A body that only credits
    // or only debits, while a sibling does the opposite, is a conservation gap.
    let gaps = crate::deltas::conservation_gaps(deltas);
    for g in gaps.iter().filter(|g| g.one_sided).take(8) {
        out.push(PropSeed {
            id: format!("P-{n:02}"),
            kind: "delta-gap".into(),
            statement: format!(
                "{} writes only [{}] / [{}] — check the sibling path credits the other side",
                g.function,
                g.plus.join(","),
                g.minus.join(",")
            ),
            evidence: format!("one-sided Δ in {}", g.function),
            guarantee: "EXPLORATORY".into(),
        });
        n += 1;
    }

    if perm_value {
        out.push(PropSeed {
            id: format!("P-{n:02}"),
            kind: "conservation".into(),
            statement: "absent documented yield, aggregate value out ≤ aggregate value in at the system boundary".into(),
            evidence: "permissionless value-flow entry exists".into(),
            guarantee: "SHOULD-HOLD".into(),
        });
        n += 1;
        out.push(PropSeed {
            id: format!("P-{n:02}"),
            kind: "authority".into(),
            statement: "no unprivileged sequence grants a role/allowance the actor was not given".into(),
            evidence: "permissionless entries present".into(),
            guarantee: "SHOULD-HOLD".into(),
        });
    }

    // Guard-lift: a require on a named bound that may not hold at every write site.
    for (pat, stmt) in [
        ("require(amount >=", "every live position/amount ≥ the documented minimum after any sequence"),
        ("require(_fee", "fee stays inside its setter bound at every write site"),
        ("require(ltv", "LTV/collateral ratio holds after this protocol's rounding, not just at the require callsite"),
    ] {
        if src_lc.contains(pat) {
            n += 1;
            out.push(PropSeed {
                id: format!("P-{n:02}"),
                kind: "guard-lift".into(),
                statement: stmt.into(),
                evidence: format!("saw `{pat}` — check ALL write sites"),
                guarantee: "EXPLORATORY".into(),
            });
        }
    }
    out
}

pub fn to_json(seeds: &[PropSeed]) -> String {
    let mut s = String::from("[\n");
    for (i, p) in seeds.iter().enumerate() {
        if i > 0 {
            s.push_str(",\n");
        }
        s.push_str(&format!(
            "    {{\"id\": \"{}\", \"kind\": \"{}\", \"guarantee\": \"{}\", \"statement\": \"{}\", \"evidence\": \"{}\"}}",
            esc(&p.id),
            esc(&p.kind),
            esc(&p.guarantee),
            esc(&p.statement),
            esc(&p.evidence)
        ));
    }
    s.push_str("\n  ]");
    s
}

pub fn to_markdown(seeds: &[PropSeed]) -> String {
    let mut s = String::from("# PROPERTIES (mechanical seeds)\n\n");
    s.push_str("Generated by `auditooor-scan`. `[ ]` = not yet implemented in the harness.\n");
    s.push_str("Discovery agents may add `P-NNx` IDs; do not reuse these numbers.\n\n");
    for p in seeds {
        s.push_str(&format!(
            "- [ ] `{id}` ({kind}, {g}) {stmt} — _{ev}_\n",
            id = p.id,
            kind = p.kind,
            g = p.guarantee,
            stmt = p.statement,
            ev = p.evidence
        ));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entries::scan_entries;

    #[test]
    fn open_vault_emits_roundtrip_and_conservation() {
        let src = include_str!("../fixtures/OpenVault.sol");
        let ents = scan_entries("OpenVault.sol", src);
        let ds = crate::deltas::scan_deltas(src);
        let ps = seeds(&ents, &src.to_lowercase(), &ds);
        assert!(ps.iter().any(|p| p.kind == "pair"));
        assert!(ps.iter().any(|p| p.kind == "conservation"));
        assert!(ps.iter().any(|p| p.guarantee == "SHOULD-HOLD"));
    }
}
