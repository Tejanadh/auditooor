//! Same-function storage deltas. This is the mechanical half of Pashov x-ray v2
//! invariant synthesis: `Δ(A)=+x` paired with `Δ(B)=-x` in one body is a
//! conservation candidate; a function that writes only one side is the gap.

use crate::detectors::blank_comments;
use crate::json_escape as esc;

#[derive(Debug, Clone)]
pub struct Delta {
    pub function: String,
    pub var: String,
    pub dir: i8,
    pub line: usize,
}

#[derive(Debug, Clone)]
pub struct ConservationGap {
    pub function: String,
    pub plus: Vec<String>,
    pub minus: Vec<String>,
    /// true if this function writes only one side of a pair seen elsewhere.
    pub one_sided: bool,
}

fn ident_before(line: &str, op_at: usize) -> Option<String> {
    let prefix = line[..op_at].trim_end();
    let ident: String = prefix
        .chars()
        .rev()
        .take_while(|c| c.is_alphanumeric() || *c == '_' || *c == ']' || *c == '[' || *c == '.')
        .collect::<String>()
        .chars()
        .rev()
        .collect();
    let ident = ident
        .split(|c| c == '[' || c == '.')
        .next()
        .unwrap_or("")
        .trim()
        .to_string();
    if ident.is_empty() || ident.chars().next()?.is_ascii_digit() {
        None
    } else {
        Some(ident)
    }
}

/// Extract += / -= / ++ / -- writes from Solidity (comments blanked).
pub fn scan_deltas(src: &str) -> Vec<Delta> {
    let blanked = blank_comments(src);
    let mut out = Vec::new();
    let mut func = String::from("?");
    for (i, raw) in blanked.lines().enumerate() {
        let t = raw.trim();
        if let Some(rest) = t.strip_prefix("function ") {
            let name: String = rest
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            if !name.is_empty() {
                func = name;
            }
        }
        let bytes = t.as_bytes();
        let mut j = 0;
        while j + 1 < bytes.len() {
            if bytes[j] == b'+' && bytes[j + 1] == b'=' {
                if let Some(v) = ident_before(t, j) {
                    out.push(Delta { function: func.clone(), var: v, dir: 1, line: i + 1 });
                }
                j += 2;
                continue;
            }
            if bytes[j] == b'-' && bytes[j + 1] == b'=' {
                if let Some(v) = ident_before(t, j) {
                    out.push(Delta { function: func.clone(), var: v, dir: -1, line: i + 1 });
                }
                j += 2;
                continue;
            }
            j += 1;
        }
    }
    out
}

/// Group deltas per function; flag bodies that only increment or only decrement
/// when the file as a whole has both directions on related vars.
pub fn conservation_gaps(deltas: &[Delta]) -> Vec<ConservationGap> {
    use std::collections::BTreeMap;
    let mut by_fn: BTreeMap<String, (Vec<String>, Vec<String>)> = BTreeMap::new();
    for d in deltas {
        let e = by_fn.entry(d.function.clone()).or_default();
        if d.dir > 0 {
            e.0.push(d.var.clone());
        } else {
            e.1.push(d.var.clone());
        }
    }
    let mut out = Vec::new();
    for (f, (plus, minus)) in by_fn {
        let one_sided = plus.is_empty() ^ minus.is_empty();
        if plus.is_empty() && minus.is_empty() {
            continue;
        }
        out.push(ConservationGap { function: f, plus, minus, one_sided });
    }
    out
}

pub fn to_json(deltas: &[Delta], gaps: &[ConservationGap]) -> String {
    let mut s = String::from("{\n    \"writes\": [");
    for (i, d) in deltas.iter().take(80).enumerate() {
        if i > 0 {
            s.push_str(", ");
        }
        s.push_str(&format!(
            "{{\"fn\": \"{}\", \"var\": \"{}\", \"dir\": {}, \"line\": {}}}",
            esc(&d.function),
            esc(&d.var),
            d.dir,
            d.line
        ));
    }
    s.push_str("],\n    \"gaps\": [");
    for (i, g) in gaps.iter().enumerate() {
        if i > 0 {
            s.push_str(", ");
        }
        let plus: Vec<String> = g.plus.iter().map(|v| format!("\"{}\"", esc(v))).collect();
        let minus: Vec<String> = g.minus.iter().map(|v| format!("\"{}\"", esc(v))).collect();
        s.push_str(&format!(
            "{{\"fn\": \"{}\", \"plus\": [{}], \"minus\": [{}], \"one_sided\": {}}}",
            esc(&g.function),
            plus.join(", "),
            minus.join(", "),
            g.one_sided
        ));
    }
    s.push_str("]\n  }");
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deposit_withdraw_are_opposite_deltas() {
        let src = include_str!("../fixtures/OpenVault.sol");
        let ds = scan_deltas(src);
        assert!(ds.iter().any(|d| d.function == "deposit" && d.dir == 1));
        assert!(ds.iter().any(|d| d.function == "withdraw" && d.dir == -1));
        let gaps = conservation_gaps(&ds);
        assert!(gaps.iter().any(|g| g.function == "deposit" && g.one_sided));
    }
}
