//! Accounting-anomaly detectors — the "detective" layer.
//!
//! SIG/AC recognise known CWE *patterns*. These flag structural *accounting
//! mismatches* — the shapes that become real economic bugs but have no CWE name:
//! crediting a nominal amount instead of the measured balance delta (fee-on-
//! transfer), consuming a raw `balanceOf(this)` in share/price math (donation/
//! inflation), and sibling divergence (one entrypoint missing a guard its twin
//! has). These point at THE function, the way a human reading `depositToken` does
//! — but they are LEADS with false positives (a trusted non-FoT token makes ACC-01
//! intended), not verdicts. They put the right function on the list; the human/PoC
//! decides.

use crate::detectors::{blank_comments, Detection};

/// Iterate `(name, body)` for each function, bodies lowercased and comment/string
/// blanked. Body is the text between the header's `{` and its matching `}`.
fn iter_functions(src: &str) -> Vec<(String, String)> {
    let blanked = blank_comments(src);
    let b = blanked.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while let Some(rel) = blanked[i..].find("function ") {
        let start = i + rel + "function ".len();
        // name = up to '('
        let name_end = blanked[start..].find('(').map(|p| start + p).unwrap_or(start);
        let name = blanked[start..name_end].trim().to_string();
        // find the header's opening '{' (or ';' for an abstract decl)
        let mut j = name_end;
        let mut body_open = None;
        while j < b.len() {
            match b[j] as char {
                '{' => { body_open = Some(j); break; }
                ';' => break,
                _ => {}
            }
            j += 1;
        }
        let open = match body_open {
            Some(o) => o,
            None => { i = name_end + 1; continue; }
        };
        // match braces to find the body close
        let mut depth = 0i32;
        let mut k = open;
        let mut close = open;
        while k < b.len() {
            match b[k] as char {
                '{' => depth += 1,
                '}' => { depth -= 1; if depth == 0 { close = k; break; } }
                _ => {}
            }
            k += 1;
        }
        let body = blanked[open..=close.min(b.len() - 1)].to_lowercase();
        if !name.is_empty() {
            out.push((name, body));
        }
        i = close + 1;
    }
    out
}

/// A value-credit operation, across eras/spellings: modern `+=`/`_mint`, and the
/// SafeMath-era `safeAdd`/`.add(` that 2016 code (EtherDelta) uses. Body is lowercased.
fn credits_value(body: &str) -> bool {
    ["+=", "_mint(", "safeadd(", "safe_add(", ".add("]
        .iter()
        .any(|k| body.contains(k))
}

fn guard_count(body: &str) -> u32 {
    let mut n = 0;
    for g in ["require(", "revert", "onlyowner", "onlyrole", "_checkowner", "msg.sender ==", "msg.sender=="] {
        n += body.matches(g).count() as u32;
    }
    n
}

/// Longest shared name stem for grouping siblings (deposit / depositFor / depositTo).
fn stem(name: &str) -> String {
    let n = name.trim_start_matches('_').to_lowercase();
    // strip common suffixes so deposit/depositFor/depositTo/emergencyWithdraw cluster
    for suf in ["for", "to", "from", "with", "internal", "unsafe"] {
        if let Some(p) = n.strip_suffix(suf) {
            if p.len() >= 4 { return p.to_string(); }
        }
    }
    for pre in ["emergency", "unsafe", "force", "admin"] {
        if let Some(p) = n.strip_prefix(pre) {
            if p.len() >= 4 { return p.to_string(); }
        }
    }
    n
}

/// Run the accounting-anomaly detectors over a file.
pub fn detect_accounting(src: &str) -> Vec<Detection> {
    let fns = iter_functions(src);
    let mut out = Vec::new();

    for (name, body) in &fns {
        let nlc = name.to_lowercase();
        let has_transferfrom = body.contains("transferfrom(") || body.contains(".safetransferfrom(");
        let measures_balance = body.contains("balanceof(address(this))") || body.contains("balanceof(this)");
        let credits_state = credits_value(body);

        // ACC-01 — fee-on-transfer / nominal-amount credit.
        if has_transferfrom && !measures_balance && credits_state {
            out.push(Detection {
                line: 0,
                id: "ACC-01".into(),
                severity: "HIGH".into(),
                title: format!("Fee-on-transfer: {name}() credits nominal amount, not measured balance"),
                detail: "The function pulls tokens via transferFrom then credits the *amount argument* to internal accounting, with no balanceOf(address(this)) before/after measurement. A fee-on-transfer, deflationary, or rebasing token delivers less than `amount`, so the contract over-credits — the deposit/accounting mismatch that drains the pool. Confirm whether the token set is trusted-non-FoT (then intended) or open.".into(),
            });
        }

        // ACC-02 — raw balanceOf(this) consumed in share/price math (donation/inflation).
        let uses_raw_balance = measures_balance
            || body.contains("balanceof(address(this))")
            || body.contains(".balanceof(address(this))");
        let in_math = body.contains(" / ") || body.contains("/=") || body.contains("* totalsupply")
            || body.contains("totalsupply") && body.contains("/");
        if uses_raw_balance && in_math {
            out.push(Detection {
                line: 0,
                id: "ACC-02".into(),
                severity: "MEDIUM".into(),
                title: format!("Raw balance consumer: {name}() prices on balanceOf(this)"),
                detail: "Share/price math reads the contract's live token balance directly. A direct token donation (transfer, not deposit) inflates that balance without minting shares — the first-depositor / inflation-attack surface. Prefer an internal accounted total over balanceOf.".into(),
            });
        }

        // ACC-03 — inflow-less credit (ETH printer / mint without receipt).
        // A printer only CREDITS. A ledger MOVE (transfer, batch transfer, internal
        // settlement like EtherDelta tradeBalances) debits one side and credits the
        // other in the same body — net zero, not a print. Skip when the body debits,
        // and skip the token's own named mechanics as belt-and-suspenders.
        let is_token_fn = matches!(
            nlc.as_str(),
            "transfer" | "transferfrom" | "_transfer" | "_mint" | "_burn" | "_update"
                | "batchtransfer" | "batchtransferfrom" | "safebatchtransferfrom" | "safetransferfrom"
        );
        let has_debit = body.contains("-=")
            || body.contains("safesub(")
            || body.contains("safe_sub(")
            || body.contains(".sub(");
        let increases = credits_value(body);
        let has_inflow = body.contains("msg.value") || has_transferfrom;
        let credits_caller = body.contains("[msg.sender]") || body.contains("_mint(msg.sender");
        if increases && credits_caller && !has_inflow && !is_token_fn && !has_debit {
            out.push(Detection {
                line: 0,
                id: "ACC-03".into(),
                severity: "HIGH".into(),
                title: format!("Inflow-less credit: {name}() increases a balance with no value received"),
                detail: "The function credits the caller (mapping += / _mint) but takes no msg.value and no transferFrom in the same body — an internal balance minted without a matching inflow. Confirm the value arrives another way; if not, this prints claims against the pool.".into(),
            });
        }
    }

    // ACC-04 — sibling divergence: entrypoints sharing a stem where one is far less
    // guarded than its twin (the "one path forgot the check" class).
    use std::collections::HashMap;
    let mut groups: HashMap<String, Vec<(String, u32)>> = HashMap::new();
    for (name, body) in &fns {
        groups.entry(stem(name)).or_default().push((name.clone(), guard_count(body)));
    }
    // Only the "shortcut forgot the check" shape — a privileged-sounding variant
    // that dropped its base sibling's guard. Precise: audited code guards its
    // transfer/grantRole siblings differently, but does not ship emergency* twins
    // that silently drop a check.
    const SHORTCUT: &[&str] = &["emergency", "force", "admin", "unsafe", "quick", "instant", "direct", "skip"];
    for (_stem, members) in &groups {
        if members.len() < 2 {
            continue;
        }
        let max_g = members.iter().map(|(_, g)| *g).max().unwrap_or(0);
        for (name, g) in members {
            let is_shortcut = SHORTCUT.iter().any(|s| name.to_lowercase().contains(s));
            if max_g >= 1 && *g == 0 && is_shortcut {
                let twin = members.iter().find(|(n, gg)| n != name && *gg == max_g).map(|(n, _)| n.clone()).unwrap_or_default();
                out.push(Detection {
                    line: 0,
                    id: "ACC-04".into(),
                    severity: "MEDIUM".into(),
                    title: format!("Sibling divergence: {name}() has no guard; its twin {twin}() does"),
                    detail: "Two entrypoints share a stem, but this one carries no require/role/sender guard while its sibling does. The classic 'one code path forgot the check' bug. Confirm the missing guard is intentional, not an omission.".into(),
                });
            }
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flags_fee_on_transfer_credit() {
        let src = r#"
            contract Vault {
                function deposit(uint256 amount) external {
                    token.transferFrom(msg.sender, address(this), amount);
                    balances[msg.sender] += amount;
                }
            }
        "#;
        let d = detect_accounting(src);
        assert!(d.iter().any(|x| x.id == "ACC-01"), "expected ACC-01: {:?}", d.iter().map(|x| &x.id).collect::<Vec<_>>());
    }

    #[test]
    fn safe_balance_delta_pattern_is_clean() {
        let src = r#"
            contract Vault {
                function deposit(uint256 amount) external {
                    uint256 before = token.balanceOf(address(this));
                    token.transferFrom(msg.sender, address(this), amount);
                    uint256 received = token.balanceOf(address(this)) - before;
                    balances[msg.sender] += received;
                }
            }
        "#;
        let d = detect_accounting(src);
        assert!(!d.iter().any(|x| x.id == "ACC-01"), "measured-delta deposit must be clean");
    }

    #[test]
    fn flags_sibling_divergence() {
        let src = r#"
            contract V {
                function withdraw(uint256 a) external { require(shares[msg.sender] >= a); pay(a); }
                function emergencyWithdraw(uint256 a) external { pay(a); }
            }
        "#;
        let d = detect_accounting(src);
        assert!(d.iter().any(|x| x.id == "ACC-04"), "expected sibling divergence: {:?}", d);
    }

    #[test]
    fn flags_etherdelta_safeadd_deposit() {
        // The real EtherDelta v2 depositToken shape: safeAdd, not +=. Must still fire.
        let src = r#"
            contract EtherDelta {
                function depositToken(address token, uint amount) {
                    if (!Token(token).transferFrom(msg.sender, this, amount)) throw;
                    tokens[token][msg.sender] = safeAdd(tokens[token][msg.sender], amount);
                }
            }
        "#;
        let d = detect_accounting(src);
        assert!(d.iter().any(|x| x.id == "ACC-01"), "safeAdd deposit must fire ACC-01: {:?}", d.iter().map(|x| &x.id).collect::<Vec<_>>());
    }

    #[test]
    fn token_transfer_does_not_fire_acc03() {
        // StandardToken.transfer moves balances by design — not an inflow-less printer.
        let src = r#"
            contract StandardToken {
                function transfer(address to, uint amount) returns (bool) {
                    balances[msg.sender] = safeSub(balances[msg.sender], amount);
                    balances[to] = safeAdd(balances[to], amount);
                    return true;
                }
            }
        "#;
        let d = detect_accounting(src);
        assert!(!d.iter().any(|x| x.id == "ACC-03"), "token transfer must not fire ACC-03: {:?}", d);
    }

    #[test]
    fn flags_inflow_less_credit() {
        let src = r#"
            contract V {
                function claim(uint256 a) external { balances[msg.sender] += a; }
            }
        "#;
        let d = detect_accounting(src);
        assert!(d.iter().any(|x| x.id == "ACC-03"));
    }
}
