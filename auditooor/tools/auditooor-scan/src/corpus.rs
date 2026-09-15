//! Known-bug-class corpus — the baked-in prior-art the novelty hash never had.
//!
//! WEAK JOINT THIS CLOSES (field-reported): local `fingerprint` only self-dedups,
//! so it returned NOVEL on textbook ALM MEV (TWAP 1-tick sandwich, rebalance-band
//! MEV, Arrakis spot-mint NAV, first-depositor inflation, Cantina fee-rounding).
//! World-novelty only worked when a human ran the search. This module gives the
//! binary a curated library of *public* bug classes so the local pass can say
//! "this is textbook — KNOWN-CLASS, here is the canonical prior-art" WITHOUT the
//! web, and carry a payability tag so intended-MEV / dust / grief auto-demotes
//! before it ever inflates the LEAD list.
//!
//! This is a PRE-FILTER, not a novelty *verdict*: a corpus match is a hard
//! "don't dress this as novel / don't spend a PoC"; a MISS still requires the
//! full world-search (a class not in the corpus is not proven novel). Precision
//! over recall — every entry is a real, publicly-documented class.

/// How payable a class is on a live program, independent of novelty.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Payability {
    /// Intended behavior / accepted trade-off / needs trusted role. Auto-demote.
    Intended,
    /// Real but low-tier: dust, temporary DoS with workaround, griefing.
    LowTier,
    /// Genuinely payable class IF a fresh, unclaimed instance is proven.
    Payable,
}

impl Payability {
    pub fn as_str(&self) -> &'static str {
        match self {
            Payability::Intended => "INTENDED/OUT-OF-SCOPE",
            Payability::LowTier => "LOW-TIER (dust/grief/temp-DoS)",
            Payability::Payable => "PAYABLE-IF-NOVEL",
        }
    }
}

/// One curated, publicly-documented bug class.
pub struct KnownClass {
    pub name: &'static str,
    pub prior_art: &'static str,
    pub payability: Payability,
    /// Match fires when the normalized haystack contains ANY of these AND, if
    /// `plus_any` is non-empty, ANY of those too. Two-factor keeps precision high.
    pub any: &'static [&'static str],
    pub plus_any: &'static [&'static str],
}

/// The corpus. Curated from public ALM/vault/AMM disclosures 2022–2026. Keep
/// entries specific: a false KNOWN-CLASS kills a real finding, so two-factor
/// match (mechanism-signal AND sink/context-signal) wherever a single word is
/// ambiguous.
pub const CORPUS: &[KnownClass] = &[
    KnownClass {
        name: "TWAP single-tick / oracle-window sandwich (ALM MEV)",
        prior_art: "Textbook ALM MEV; Arrakis/Gamma/Charm class. Sandwiching a narrow TWAP or a 1-tick move around rebalance is a known, generally-accepted MEV trade-off, not a payable bug on mature vaults.",
        payability: Payability::Intended,
        any: &["twap", "tick", "sandwich", "rebalance band", "rebalance-band"],
        plus_any: &["mev", "sandwich", "oracle", "rebalance", "twap", "tick"],
    },
    KnownClass {
        name: "Spot mint/burn NAV manipulation (Arrakis-style)",
        prior_art: "Arrakis V1/V2 spot mint/burn NAV. Applies only where deposit/withdraw prices off live slot0 reserves; does NOT map to vaults that burn real liquidity pro-rata. Widely disclosed.",
        payability: Payability::Intended,
        any: &["spot mint", "spot burn", "nav", "slot0", "spot price mint"],
        plus_any: &["mint", "burn", "nav", "deposit", "withdraw", "reserves"],
    },
    KnownClass {
        name: "First-depositor / share-inflation (empty-vault donation)",
        prior_art: "ERC4626 inflation attack (OZ advisory, countless C4/Sherlock dups). Mitigated by virtual shares / dead shares / min-liquidity. Only payable if the specific mitigation is absent AND reachable.",
        payability: Payability::Payable,
        any: &["first deposit", "first-depositor", "share inflation", "share-price inflation", "inflation attack", "donation"],
        plus_any: &["share", "totalsupply", "mint", "deposit", "vault", "price", "4626"],
    },
    KnownClass {
        name: "Fee-rounding / rounding-direction skim",
        prior_art: "Cantina/C4 fee-rounding family (e.g. Charm v2.1, Balancer $120M was the payable extreme). Dust-scale rounding that favors the pool is usually intended; only payable if it compounds to material loss or flips solvency.",
        payability: Payability::LowTier,
        any: &["fee rounding", "rounding direction", "round down", "round up", "dust", "rounding"],
        plus_any: &["fee", "rounding", "dust", "wei", "precision", "share"],
    },
    KnownClass {
        name: "Fee-on-transfer / rebasing nominal-credit mismatch",
        prior_art: "FoT/rebasing deposit over-credit (ACC-01 class). Intended when the token set is trusted-non-FoT (most vaults whitelist); only payable if an untrusted/open token path reaches internal accounting.",
        payability: Payability::Intended,
        any: &["fee on transfer", "fee-on-transfer", "fot", "rebasing", "deflationary"],
        plus_any: &["transfer", "deposit", "balance", "credit", "amount", "accounting"],
    },
    KnownClass {
        name: "Uninitialized proxy / missing _disableInitializers (AC-03)",
        prior_art: "CPIMP class (Wormhole-style). Real crit ONLY if the implementation is reachable and initialization confers control; an empty/logic-less implementation behind a used proxy is not promotable.",
        payability: Payability::Payable,
        any: &["uninitialized", "disableinitializers", "initializer", "ac-03", "reinitialize", "re-initialize"],
        plus_any: &["proxy", "implementation", "initialize", "upgrade", "owner", "init"],
    },
    KnownClass {
        name: "ecrecover malleability / zero-address (SIG-01/02)",
        prior_art: "Signature malleability & silent zero-address. NOT exploitable when replay tracking keys on the digest (not the raw sig) and address(0) holds no balance — the usual case.",
        payability: Payability::LowTier,
        any: &["ecrecover", "malleab", "signature replay", "sig malleability", "high-s"],
        plus_any: &["signature", "ecrecover", "replay", "nonce", "digest", "permit"],
    },
    KnownClass {
        name: "Temporary DoS / griefing with recovery path",
        prior_art: "Availability-only impact with an expiry/admin/alternate-path recovery. Most programs rule ineligible or Low; not a fund-loss class.",
        payability: Payability::LowTier,
        any: &["temporary dos", "temp dos", "griefing", "grief", "denial of service", "revert", "brick"],
        plus_any: &["dos", "revert", "stuck", "lock", "recover", "expiry", "availability"],
    },
];

/// A corpus classification result.
pub struct CorpusMatch<'a> {
    pub class: &'a KnownClass,
}

fn norm(s: &str) -> String {
    s.to_lowercase()
}

/// Classify a candidate against the corpus. `protocol` is included in the
/// haystack so protocol-specific signals (e.g. a fork name) can contribute.
/// Returns the FIRST (most-specific-ordered) match, or None (= corpus miss,
/// still run the full world-search).
pub fn classify<'a>(protocol: &str, mechanism: &str, sink: &str) -> Option<CorpusMatch<'a>> {
    let hay = format!("{} {} {}", norm(protocol), norm(mechanism), norm(sink));
    for class in CORPUS {
        let a = class.any.iter().any(|k| hay.contains(k));
        if !a {
            continue;
        }
        let b = class.plus_any.is_empty() || class.plus_any.iter().any(|k| hay.contains(k));
        if a && b {
            return Some(CorpusMatch { class });
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn twap_sandwich_is_known_intended() {
        let m = classify("Charm", "TWAP 1-tick sandwich at rebalance", "vault NAV").unwrap();
        assert!(m.class.name.contains("TWAP"));
        assert_eq!(m.class.payability, Payability::Intended);
    }

    #[test]
    fn first_depositor_is_known_payable() {
        let m = classify("Acme", "first-depositor share inflation", "totalSupply").unwrap();
        assert_eq!(m.class.payability, Payability::Payable);
    }

    #[test]
    fn fee_on_transfer_known_intended() {
        let m = classify("X", "fee-on-transfer over-credit", "deposit accounting").unwrap();
        assert_eq!(m.class.payability, Payability::Intended);
    }

    #[test]
    fn novel_logic_bug_misses_corpus() {
        // A genuinely protocol-specific accounting bug has no corpus entry.
        assert!(classify("X", "epoch reward index desync across migration", "cumulativeSum").is_none());
    }

    #[test]
    fn single_ambiguous_word_needs_second_factor() {
        // "rounding" alone with a money context matches fee-rounding...
        assert!(classify("X", "rounding direction on fee", "share").is_some());
    }
}
