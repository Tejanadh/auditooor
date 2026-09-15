//! High-precision pattern detectors for two payout-heavy, statically-detectable
//! bug classes: signature/crypto-scheme flaws and access-control/init flaws.
//!
//! Out-of-box rationale: the LLM scanner-swarm reasons probabilistically about
//! these. A deterministic scanner flags the exact structural pattern with near-
//! zero miss rate, then hands the LEAD to the proof pipeline. These are LEADS —
//! high-signal starting points — never findings; the PoC gate still decides.

/// One flagged lead.
#[derive(Debug, Clone, PartialEq)]
pub struct Detection {
    pub line: usize,
    pub id: String,
    pub severity: String,
    pub title: String,
    pub detail: String,
}

fn has(lower: &[String], needle: &str) -> bool {
    lower.iter().any(|l| l.contains(needle))
}

/// Blank out `//` and `/* */` comments and string-literal *contents*, replacing
/// them with spaces while preserving newlines — so line numbers stay exact and
/// no keyword is ever matched inside a comment or string. This is what gives the
/// detectors precision without needing an AST.
pub fn blank_comments(src: &str) -> String {
    #[derive(PartialEq)]
    enum S {
        Code,
        Line,
        Block,
        Str(u8),
    }
    let b = src.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(b.len());
    let mut st = S::Code;
    let mut i = 0;
    while i < b.len() {
        let c = b[i];
        let n = if i + 1 < b.len() { b[i + 1] } else { 0 };
        match st {
            S::Code => {
                if c == b'/' && n == b'/' {
                    st = S::Line;
                    out.push(b' ');
                    out.push(b' ');
                    i += 2;
                } else if c == b'/' && n == b'*' {
                    st = S::Block;
                    out.push(b' ');
                    out.push(b' ');
                    i += 2;
                } else if c == b'"' || c == b'\'' {
                    st = S::Str(c);
                    out.push(c);
                    i += 1;
                } else {
                    out.push(c);
                    i += 1;
                }
            }
            S::Line => {
                out.push(if c == b'\n' {
                    st = S::Code;
                    b'\n'
                } else {
                    b' '
                });
                i += 1;
            }
            S::Block => {
                if c == b'*' && n == b'/' {
                    st = S::Code;
                    out.push(b' ');
                    out.push(b' ');
                    i += 2;
                } else {
                    out.push(if c == b'\n' { b'\n' } else { b' ' });
                    i += 1;
                }
            }
            S::Str(q) => {
                if c == b'\\' && i + 1 < b.len() {
                    out.push(b' ');
                    out.push(b' ');
                    i += 2;
                } else if c == q {
                    st = S::Code;
                    out.push(c);
                    i += 1;
                } else {
                    out.push(if c == b'\n' { b'\n' } else { b' ' });
                    i += 1;
                }
            }
        }
    }
    String::from_utf8(out).unwrap_or_else(|_| src.to_string())
}

/// Signature / crypto-scheme detectors (SIG-01..SIG-05).
pub fn detect_signature(src: &str) -> Vec<Detection> {
    // Raw (unblanked) view: SIG-06 must see selector strings like "permit(...)"
    // that the blanked view intentionally wipes.
    let raw_lower: Vec<String> = src.lines().map(|l| l.to_lowercase()).collect();
    let src = blank_comments(src);
    let lower: Vec<String> = src.lines().map(|l| l.to_lowercase()).collect();
    let mut out = Vec::new();

    let uses_ecrecover = has(&lower, "ecrecover(");
    let uses_oz_ecdsa = has(&lower, "ecdsa.recover") || has(&lower, "ecdsa.tryrecover");
    // Malleability guard signals: an explicit high-s reject or OZ ECDSA.
    let has_malleability_guard = uses_oz_ecdsa
        || has(&lower, "secp256k1")
        || has(&lower, "0x7fffffffffffffffffffffffffffffff5d576e7357a4501ddfe92f46681b20a0")
        || has(&lower, "highs")
        || lower.iter().any(|l| l.contains("s >") && l.contains("0x7fff"));
    let has_chainid = has(&lower, "block.chainid") || has(&lower, "chainid()");
    let has_nonce = has(&lower, "nonce");
    // A pure recovery/crypto library is not responsible for nonce/replay — its caller is.
    let is_library = has(&lower, "library ");
    // Only flag domain-separator bugs when the file actually CONSTRUCTS the domain
    // (has the EIP712Domain typehash), not when it merely calls an inherited getter
    // like `_domainSeparatorV4()` — that construction lives in the parent (EIP712.sol).
    let constructs_domain = has(&lower, "eip712domain");
    let has_permit = has(&lower, "function permit");
    let has_deadline = has(&lower, "deadline") || has(&lower, "expiry");

    let mut nonce_flagged = false;
    for (i, l) in lower.iter().enumerate() {
        if !l.contains("ecrecover(") {
            continue;
        }
        if !has_malleability_guard {
            out.push(Detection {
                line: i + 1,
                id: "SIG-01".into(),
                severity: "HIGH".into(),
                title: "Signature malleability (raw ecrecover, no high-s reject)".into(),
                detail: "For (v,r,s) a second valid sig exists via s'=n-s, v'=1-v. Without a high-s reject or OpenZeppelin ECDSA, a used signature can be replayed in malleated form. Verify replay tracking keys on the digest, not the raw signature.".into(),
            });
        }
        // Zero-address recovery: ecrecover returns address(0) on bad input.
        let lo = i.saturating_sub(4);
        let hi = (i + 4).min(lower.len().saturating_sub(1));
        let guards_zero = lower[lo..=hi].iter().any(|w| w.contains("address(0)") || w.contains("== 0)"));
        if !guards_zero {
            out.push(Detection {
                line: i + 1,
                id: "SIG-02".into(),
                severity: "HIGH".into(),
                title: "Missing zero-address check on ecrecover result".into(),
                detail: "ecrecover fails silently to address(0). Without `signer != address(0)`, a malformed signature authorizes actions as the zero address.".into(),
            });
        }
        if !has_nonce && !nonce_flagged && !is_library {
            out.push(Detection {
                line: i + 1,
                id: "SIG-03".into(),
                severity: "CRITICAL".into(),
                title: "No nonce near signature verification (replay)".into(),
                detail: "Signature verification with no nonce mapping/increment in the file: the same signature can be submitted repeatedly. Confirm one-time consumption.".into(),
            });
            nonce_flagged = true;
        }
    }

    if constructs_domain && !has_chainid {
        let line = lower.iter().position(|l| l.contains("eip712domain")).map(|i| i + 1).unwrap_or(1);
        out.push(Detection {
            line,
            id: "SIG-04".into(),
            severity: "HIGH".into(),
            title: "EIP-712 domain separator without block.chainid (cross-chain/fork replay)".into(),
            detail: "Domain separator lacks chainId (or caches it without recompute). After a fork/chain split, or across a sibling deployment, the same signed message verifies where it should not.".into(),
        });
    }
    if (has_permit || (uses_ecrecover && constructs_domain)) && !has_deadline {
        let line = lower.iter().position(|l| l.contains("permit")).map(|i| i + 1).unwrap_or(1);
        out.push(Detection {
            line,
            id: "SIG-05".into(),
            severity: "MEDIUM".into(),
            title: "Signed approval without deadline/expiry".into(),
            detail: "No deadline/expiry bound: a leaked or intercepted signature stays valid forever. EIP-2612 requires a deadline check.".into(),
        });
    }

    // SIG-06 — phantom permit ("billion-dollar no-op"): permit() invoked via a
    // low-level call. On a token that lacks permit but has a fallback, the call
    // silently succeeds, the approval never happens, and funds are pulled anyway.
    // Scanned on RAW source (the selector lives in a string) with a small window,
    // since the low-level call and the "permit(...)" selector often span lines.
    for (i, l) in raw_lower.iter().enumerate() {
        let low_level = l.contains("abi.encodewithsignature")
            || l.contains("abi.encodewithselector")
            || l.contains(".call(");
        if !low_level {
            continue;
        }
        let hi = (i + 3).min(raw_lower.len().saturating_sub(1));
        if raw_lower[i..=hi].iter().any(|w| w.contains("permit(")) {
            out.push(Detection {
                line: i + 1,
                id: "SIG-06".into(),
                severity: "HIGH".into(),
                title: "Phantom permit: permit() called via low-level call".into(),
                detail: "A permit invoked through a low-level call succeeds silently on a token that has a fallback but no permit (a no-op). The approval never occurs; verify the token implements EIP-2612 and check code.length, or use SafeERC20.".into(),
            });
            break; // one lead per file is enough
        }
    }
    out
}

/// Access-control / initialization detectors (AC-01..AC-02).
pub fn detect_access_control(src: &str) -> Vec<Detection> {
    let src = blank_comments(src);
    let lower: Vec<String> = src.lines().map(|l| l.to_lowercase()).collect();
    let mut out = Vec::new();

    let has_disable_initializers = has(&lower, "_disableinitializers");
    // Upgradeable context: an uninitialized-proxy bug needs an implementation with
    // NO constructor (or one inheriting an Initializable/UUPS base). A contract with
    // a real constructor + immutables (e.g. a Uni-v4 position manager) is not it.
    let has_constructor = has(&lower, "constructor(");
    let inherits_upgradeable =
        has(&lower, "initializable") || has(&lower, "upgradeable") || has(&lower, "uups");
    let looks_upgradeable = !has_constructor || inherits_upgradeable;
    // Real initializer = a function named EXACTLY `initialize(` (not initializePool /
    // initializeStdChains), external/public, and not an internal/pure library packer.
    let mut real_initialize = false;

    for (i, l) in lower.iter().enumerate() {
        if l.contains("function initialize(") {
            let hi = (i + 3).min(lower.len().saturating_sub(1));
            let window = &lower[i..=hi];
            let is_entry = window.iter().any(|w| w.contains("external") || w.contains("public"));
            let is_readonly = window
                .iter()
                .any(|w| w.contains(" view") || w.contains(" pure") || w.contains("internal") || w.contains("private"));
            if is_entry && !is_readonly {
                real_initialize = true;
                let guarded = window.iter().any(|w| {
                    w.contains("initializer") || w.contains("reinitializer") || w.contains("oninitializing")
                });
                if !guarded && looks_upgradeable {
                    out.push(Detection {
                        line: i + 1,
                        id: "AC-01".into(),
                        severity: "CRITICAL".into(),
                        title: "Unprotected initializer (front-run / re-initialization)".into(),
                        detail: "An external initialize() with no `initializer` modifier, in an upgradeable/constructor-less contract, can be called by anyone before the deployer (front-run to seize ownership) or again after deployment. Confirm the modifier and atomic deploy+init.".into(),
                    });
                }
            }
        }
        if l.contains("_authorizeupgrade") {
            let hi = (i + 3).min(lower.len().saturating_sub(1));
            let window = &lower[i..=hi];
            // Only an IMPLEMENTED body can be unguarded. An abstract declaration
            // (`_authorizeUpgrade(address) internal virtual;`, as in OZ's UUPS base)
            // has no body and is meant to be overridden — not a bug.
            let has_body = window.iter().any(|w| w.contains('{'));
            let guarded = window.iter().any(|w| {
                w.contains("onlyowner") || w.contains("only_owner") || w.contains("onlyrole")
                    || w.contains("access") || w.contains("require(") || w.contains("_checkowner")
            });
            if has_body && !guarded {
                out.push(Detection {
                    line: i + 1,
                    id: "AC-02".into(),
                    severity: "CRITICAL".into(),
                    title: "_authorizeUpgrade without access guard (anyone upgrades)".into(),
                    detail: "The UUPS upgrade hook must revert for all but the upgrader. No onlyOwner/role/require here means any caller can point the proxy at malicious code.".into(),
                });
            }
        }
    }

    // Upgradeable implementation left constructor-initializable. Gate on a REAL
    // external initializer AND upgradeable context — not any `initialize*` substring,
    // and not a contract that already has a constructor + immutables.
    if real_initialize && looks_upgradeable && !has_disable_initializers {
        out.push(Detection {
            line: 1,
            id: "AC-03".into(),
            severity: "HIGH".into(),
            title: "Upgradeable contract without _disableInitializers() in constructor".into(),
            detail: "An implementation with an external initialize() but no `_disableInitializers()` in its constructor can be initialized directly (CPIMP-class). Add it to the constructor.".into(),
        });
    }
    out
}

/// Run all detectors over one file's source — SIG + AC (known CWE patterns) plus
/// ACC (accounting anomalies, the detective layer). Folding ACC in here means
/// `surface`'s detector-bonus and every caller see it, not just the CLI.
pub fn scan_all(src: &str) -> Vec<Detection> {
    let mut v = detect_signature(src);
    v.extend(detect_access_control(src));
    v.extend(crate::accounting::detect_accounting(src));
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flags_raw_ecrecover_malleability_and_zeroaddr() {
        let src = "function claim(bytes32 h, uint8 v, bytes32 r, bytes32 s) external {\n  address signer = ecrecover(h, v, r, s);\n  balances[signer] += 1;\n}";
        let d = scan_all(src);
        let ids: Vec<&str> = d.iter().map(|x| x.id.as_str()).collect();
        assert!(ids.contains(&"SIG-01"), "malleability: {ids:?}");
        assert!(ids.contains(&"SIG-02"), "zero-addr: {ids:?}");
        assert!(ids.contains(&"SIG-03"), "replay/nonce: {ids:?}");
    }

    #[test]
    fn oz_ecdsa_and_nonce_and_zerocheck_are_clean() {
        let src = "using ECDSA for bytes32;\nfunction claim(...) external {\n  address signer = h.recover(sig);\n  require(signer != address(0));\n  require(nonces[signer]++ == nonce);\n}";
        let d = detect_signature(src);
        assert!(d.iter().all(|x| x.id != "SIG-01"), "OZ ECDSA should clear malleability");
        assert!(d.iter().all(|x| x.id != "SIG-03"), "nonce present should clear replay");
    }

    #[test]
    fn flags_domain_separator_without_chainid() {
        let src = "bytes32 public DOMAIN_SEPARATOR = keccak256(abi.encode(EIP712DOMAIN_TYPEHASH, name, address(this)));";
        let d = detect_signature(src);
        assert!(d.iter().any(|x| x.id == "SIG-04"));
    }

    #[test]
    fn flags_unprotected_initializer_and_missing_disable() {
        let src = "contract V {\n  function initialize(address o) external {\n    owner = o;\n  }\n}";
        let d = detect_access_control(src);
        let ids: Vec<&str> = d.iter().map(|x| x.id.as_str()).collect();
        assert!(ids.contains(&"AC-01"), "unprotected init: {ids:?}");
        assert!(ids.contains(&"AC-03"), "missing _disableInitializers: {ids:?}");
    }

    #[test]
    fn comments_do_not_poison_detectors() {
        // A comment mentioning "nonce" / "_disableInitializers" must NOT clear the
        // real leads. Precision fix: comments are blanked before matching.
        let src = "contract V {\n  // TODO: add nonce and _disableInitializers later\n  function initialize(address o) external { owner = o; }\n  function claim(bytes32 h, uint8 v, bytes32 r, bytes32 s) external {\n    address signer = ecrecover(h, v, r, s);\n    balances[signer] += 1;\n  }\n}";
        let d = scan_all(src);
        let ids: Vec<&str> = d.iter().map(|x| x.id.as_str()).collect();
        assert!(ids.contains(&"SIG-03"), "replay must still fire despite comment: {ids:?}");
        assert!(ids.contains(&"AC-03"), "missing-disable must still fire: {ids:?}");
        assert!(ids.contains(&"AC-01"), "unprotected init must fire: {ids:?}");
    }

    #[test]
    fn guarded_initializer_is_clean() {
        let src = "function initialize(address o) external initializer {\n  owner = o;\n}\nconstructor() { _disableInitializers(); }";
        let d = detect_access_control(src);
        assert!(d.iter().all(|x| x.id != "AC-01"));
        assert!(d.iter().all(|x| x.id != "AC-03"));
    }
}
