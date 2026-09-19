//! Static confirmation of a hunter's hypothesis — the GPTScan loop.
//!
//! GPTScan (ICSE'24) found that ~80% of web3 bugs are LOGIC bugs no fixed-pattern
//! tool catches, and its key move was: let the LLM propose a candidate, then run
//! STATIC CONFIRMATION on the key variable/statement to cut the false positive
//! before it costs anything. That cut its false-positive rate hard and still
//! found 9 bugs human auditors missed.
//!
//! auditooor had no such step: hunters guess, skeptics reason, the fork PoC
//! proves — and the PoC is the expensive part. This module is the cheap
//! deterministic filter in between. It answers, for one named function and one
//! hypothesis shape, whether the code actually has the dataflow the bug REQUIRES.
//!
//! It is a FILTER, not a prover. Three honest verdicts:
//!   CONFIRMED     — the code has the shape the hypothesis needs → promote, PoC it
//!   REFUTED       — the code cannot have this bug (the required shape is absent)
//!                   → kill the lead before a PoC, the way static confirmation does
//!   INCONCLUSIVE  — regex cannot decide → pass through to the human / the PoC
//!
//! A false REFUTED kills a real lead and a false CONFIRMED wastes a PoC, so every
//! check biases to INCONCLUSIVE whenever the evidence is ambiguous. The fork PoC
//! remains the only proof; this only decides what is worth proving.

/// External-call markers: the payout-relevant set (low-level calls + value
/// transfers). An interface method call `IX(a).f()` is deliberately NOT treated
/// as external here — too noisy at regex level — so the CEI check stays precise.
const EXT_CALL: &[&str] = &[
    ".call(", ".call{", ".delegatecall", ".staticcall", ".transfer(", ".send(",
    ".safetransfer(", ".safetransferfrom(", ".sendvalue(",
];

/// Solidity value/type keywords that mark a line as a *local declaration*, not a
/// storage write — used to avoid calling a local a state mutation.
const DECL_KW: &[&str] = &[
    "uint", "int", "address", "bool", "bytes", "string", "mapping", "struct",
    "memory ", "calldata ", "return ", "emit ", "require(", "assert(", "if ",
    "if(", "for", "while", "else", "//",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Confirmed,
    Refuted,
    Inconclusive,
}

impl Verdict {
    pub fn as_str(&self) -> &'static str {
        match self {
            Verdict::Confirmed => "CONFIRMED",
            Verdict::Refuted => "REFUTED",
            Verdict::Inconclusive => "INCONCLUSIVE",
        }
    }
    /// Exit code convention: REFUTED exits 3 so a pipeline can drop the lead.
    pub fn exit_code(&self) -> i32 {
        match self {
            Verdict::Refuted => 3,
            _ => 0,
        }
    }
}

pub struct Confirmation {
    pub verdict: Verdict,
    pub why: String,
    /// 1-indexed lines the verdict rests on.
    pub evidence: Vec<usize>,
}

/// Extract the body (between the matching braces) of the first function whose
/// name matches `func`. Returns (body, first_line_of_body) with lines relative
/// to the whole file so evidence line numbers are real.
pub fn function_body<'a>(src: &'a str, func: &str) -> Option<(String, usize)> {
    let needle = format!("function {func}");
    let start = src.find(&needle)?;
    let bytes = src.as_bytes();
    // walk to the opening brace of the body (skip the header + any modifiers)
    let mut i = start + needle.len();
    let mut depth_paren = 0i32;
    while i < bytes.len() {
        match bytes[i] as char {
            '(' => depth_paren += 1,
            ')' => depth_paren -= 1,
            '{' if depth_paren <= 0 => break,
            ';' if depth_paren <= 0 => return None, // declaration only
            _ => {}
        }
        i += 1;
    }
    if i >= bytes.len() {
        return None;
    }
    let body_start = i + 1;
    let mut depth = 1i32;
    let mut j = body_start;
    while j < bytes.len() {
        match bytes[j] as char {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    break;
                }
            }
            _ => {}
        }
        j += 1;
    }
    let line0 = src[..body_start].bytes().filter(|&b| b == b'\n').count() + 1;
    Some((src[body_start..j.min(src.len())].to_string(), line0))
}

fn line_has_ext_call(l: &str) -> bool {
    EXT_CALL.iter().any(|k| l.contains(k))
}

/// Does this line look like a STATE write (not a local decl, not a comparison)?
fn looks_like_state_write(l: &str) -> bool {
    let t = l.trim();
    if DECL_KW.iter().any(|k| t.starts_with(k)) {
        return false;
    }
    // an assignment that is not `==`, `!=`, `<=`, `>=`
    if let Some(p) = t.find('=') {
        let after = t.as_bytes().get(p + 1).copied().unwrap_or(b' ') as char;
        let before = if p > 0 { t.as_bytes()[p - 1] as char } else { ' ' };
        let is_cmp = after == '=' || matches!(before, '=' | '!' | '<' | '>');
        if is_cmp {
            return false;
        }
        // a leading identifier/index target, e.g. `x =`, `bal[a] =`, `s.x +=`
        let lhs = t[..p].trim();
        return lhs.chars().next().map(|c| c.is_alphabetic() || c == '_').unwrap_or(false);
    }
    false
}

/// CEI / checks-effects-interactions ordering. The shape behind reentrancy and a
/// large family of stale-state bugs: a state write that happens AFTER an external
/// call in the same function.
pub fn check_cei(body: &str, line0: usize) -> Confirmation {
    let lines: Vec<&str> = body.lines().collect();
    let lc: Vec<String> = lines.iter().map(|l| l.to_lowercase()).collect();
    let last_ext = lc.iter().rposition(|l| line_has_ext_call(l));
    let first_ext = lc.iter().position(|l| line_has_ext_call(l));
    let Some(first_ext) = first_ext else {
        return Confirmation {
            verdict: Verdict::Inconclusive,
            why: "no external call in this function — CEI ordering is not the relevant shape here.".into(),
            evidence: vec![],
        };
    };
    // any state write strictly after the FIRST external call is the risky shape
    let mut writes_after = Vec::new();
    for (i, l) in lc.iter().enumerate() {
        if i > first_ext && looks_like_state_write(l) {
            writes_after.push(line0 + i);
        }
    }
    let _ = last_ext;
    if !writes_after.is_empty() {
        Confirmation {
            verdict: Verdict::Confirmed,
            why: format!(
                "state write(s) occur AFTER an external call (line {}) — the CEI-violation shape reentrancy/stale-read needs is present. Worth a reentrancy PoC.",
                line0 + first_ext
            ),
            evidence: {
                let mut e = vec![line0 + first_ext];
                e.extend(writes_after);
                e
            },
        }
    } else {
        Confirmation {
            verdict: Verdict::Refuted,
            why: format!(
                "external call at line {} is followed by no state write — the function follows checks-effects-interactions. No CEI-reentrancy path here.",
                line0 + first_ext
            ),
            evidence: vec![line0 + first_ext],
        }
    }
}

/// Unchecked low-level call return. `(bool ok,) = x.call(...)` whose `ok` is never
/// required, or a bare `x.call(...)` whose success is dropped.
pub fn check_unchecked_return(body: &str, line0: usize) -> Confirmation {
    let lines: Vec<&str> = body.lines().collect();
    let lc: Vec<String> = lines.iter().map(|l| l.to_lowercase()).collect();
    let mut call_lines = Vec::new();
    for (i, l) in lc.iter().enumerate() {
        if l.contains(".call(") || l.contains(".call{") || l.contains(".delegatecall") {
            call_lines.push(i);
        }
    }
    if call_lines.is_empty() {
        return Confirmation {
            verdict: Verdict::Inconclusive,
            why: "no low-level call in this function.".into(),
            evidence: vec![],
        };
    }
    for &i in &call_lines {
        let l = &lc[i];
        // captured into a bool? look for `bool <name>` on the assignment LHS
        let captured = l.contains("bool ") || l.starts_with('(') || l.contains(") =");
        if !captured {
            // bare call, return dropped
            return Confirmation {
                verdict: Verdict::Confirmed,
                why: format!("low-level call at line {} drops its success return — unchecked-call shape present.", line0 + i),
                evidence: vec![line0 + i],
            };
        }
        // captured — is the success flag ever checked (require/if) in the body?
        let checked = lc.iter().enumerate().any(|(j, m)| {
            j > i && (m.contains("require(") || m.contains("if (") || m.contains("if(")) &&
            (m.contains("success") || m.contains("ok") || m.contains("!") )
        });
        if !checked {
            return Confirmation {
                verdict: Verdict::Confirmed,
                why: format!("low-level call at line {} captures success but never checks it — unchecked-call shape present.", line0 + i),
                evidence: vec![line0 + i],
            };
        }
    }
    Confirmation {
        verdict: Verdict::Refuted,
        why: "every low-level call in this function checks its success return.".into(),
        evidence: call_lines.iter().map(|i| line0 + i).collect(),
    }
}

/// Guard-between: is there a `require`/`if...revert` gating the function before
/// its first value-moving sink? Confirms/refutes an "unguarded sink" hypothesis
/// at statement granularity (finer than the entry-level access class).
pub fn check_guard(body: &str, line0: usize) -> Confirmation {
    let lc = body.to_lowercase();
    let line_at = |off: usize| line0 + lc[..off.min(lc.len())].bytes().filter(|&b| b == b'\n').count();
    let sink_kw = [
        ".transfer(", ".safetransfer(", ".safetransferfrom(", ".call{", "_mint(",
        "_burn(", ".send(", "sendvalue",
    ];
    let sink = sink_kw.iter().filter_map(|k| lc.find(k)).min();
    let Some(sink) = sink else {
        return Confirmation {
            verdict: Verdict::Inconclusive,
            why: "no value-moving sink in this function body.".into(),
            evidence: vec![],
        };
    };
    // A guard is a require(...) or a revert/if-revert appearing BEFORE the sink,
    // measured by character offset so a single-line body works too.
    let before = &lc[..sink];
    let guard = before.rfind("require(").or_else(|| before.rfind("revert"));
    if let Some(g) = guard {
        Confirmation {
            verdict: Verdict::Refuted,
            why: format!("a guard at line {} precedes the value sink at line {} — the sink is not unguarded.", line_at(g), line_at(sink)),
            evidence: vec![line_at(g), line_at(sink)],
        }
    } else {
        Confirmation {
            verdict: Verdict::Confirmed,
            why: format!("value sink at line {} has no require/revert guard before it in this body — unguarded-sink shape present (confirm the caller is unprivileged via `entries`).", line_at(sink)),
            evidence: vec![line_at(sink)],
        }
    }
}

pub fn run(check: &str, body: &str, line0: usize) -> Confirmation {
    match check {
        "cei" => check_cei(body, line0),
        "unchecked-return" | "unchecked" => check_unchecked_return(body, line0),
        "guard" => check_guard(body, line0),
        _ => Confirmation {
            verdict: Verdict::Inconclusive,
            why: format!("unknown check '{check}' — use cei | unchecked-return | guard."),
            evidence: vec![],
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const REENTRANT: &str = r#"
    function withdraw(uint256 amount) external {
        require(balances[msg.sender] >= amount);
        (bool ok, ) = msg.sender.call{value: amount}("");
        require(ok);
        balances[msg.sender] -= amount;
    }
    "#;

    const SAFE_CEI: &str = r#"
    function withdraw(uint256 amount) external {
        require(balances[msg.sender] >= amount);
        balances[msg.sender] -= amount;
        (bool ok, ) = msg.sender.call{value: amount}("");
        require(ok);
    }
    "#;

    #[test]
    fn cei_violation_is_confirmed() {
        let (b, l) = function_body(REENTRANT, "withdraw").unwrap();
        let r = check_cei(&b, l);
        assert_eq!(r.verdict, Verdict::Confirmed, "{}", r.why);
    }

    #[test]
    fn correct_cei_ordering_is_refuted() {
        let (b, l) = function_body(SAFE_CEI, "withdraw").unwrap();
        let r = check_cei(&b, l);
        assert_eq!(r.verdict, Verdict::Refuted, "{}", r.why);
        assert_eq!(r.verdict.exit_code(), 3);
    }

    #[test]
    fn unchecked_bare_call_is_confirmed() {
        let src = r#"function f() external { target.call(data); }"#;
        let (b, l) = function_body(src, "f").unwrap();
        assert_eq!(check_unchecked_return(&b, l).verdict, Verdict::Confirmed);
    }

    #[test]
    fn checked_call_is_refuted() {
        let (b, l) = function_body(REENTRANT, "withdraw").unwrap();
        assert_eq!(check_unchecked_return(&b, l).verdict, Verdict::Refuted);
    }

    #[test]
    fn unguarded_sink_confirmed_guarded_refuted() {
        let open = r#"function pay(address to, uint256 a) external { to.transfer(a); }"#;
        let (b, l) = function_body(open, "pay").unwrap();
        assert_eq!(check_guard(&b, l).verdict, Verdict::Confirmed);

        let guarded = r#"function pay(address to, uint256 a) external { require(msg.sender == owner); to.transfer(a); }"#;
        let (b2, l2) = function_body(guarded, "pay").unwrap();
        assert_eq!(check_guard(&b2, l2).verdict, Verdict::Refuted);
    }

    #[test]
    fn missing_function_is_none() {
        assert!(function_body(REENTRANT, "nope").is_none());
    }
}
