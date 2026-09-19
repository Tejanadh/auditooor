//! Protocol-specific known-issues register — the novelty pillar's missing layer.
//!
//! `corpus.rs` holds GLOBAL bug classes (ALM MEV, first-depositor, FoT…). It
//! cannot know that *this* protocol's Dec-2024 audit already reported *this*
//! division-by-zero. That gap is what got the Cork run killed: the tool found
//! and fully PoC'd a real bug, then it died as known-issue #213 — after the PoC
//! was already forged. Every token past "this is a redeemEarlyLv DoS" was waste.
//!
//! Idea adapted from J4X-Security/K.I.T (MIT): build a structured register of a
//! protocol's already-known findings from its real audit reports, each keyed by
//! root cause / affected surface / mechanism / sink / impact, then CHECK every
//! candidate against it *before* a PoC is forged. The LLM does the extraction
//! (PDFs, contest pages) — this module owns the deterministic part: a stable
//! store and a precise, two-factor match, so a KNOWN verdict is cheap and a
//! false KNOWN (which would kill a real finding) is rare.
//!
//! Register format: append-only JSONL, one finding per line, at
//! `$LEDGER/known/<protocol>.jsonl`. Pure std, hand-parsed like the rest.

use std::path::Path;

#[derive(Debug, Clone, Default)]
pub struct KnownFinding {
    pub id: String,
    pub protocol: String,
    pub title: String,
    /// Free-text root cause — the tokens that make a match precise.
    pub root_cause: String,
    /// Contract / function / area the finding lives in.
    pub surface: String,
    pub mechanism: String,
    pub sink: String,
    pub impact: String,
    /// Where it came from (audit firm + date, contest, known-issues list).
    pub source: String,
}

fn esc(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', " ")
}

fn field(obj: &str, key: &str) -> String {
    let pat = format!("\"{key}\":");
    let i = match obj.find(&pat) {
        Some(i) => i + pat.len(),
        None => return String::new(),
    };
    let rest = obj[i..].trim_start();
    let rest = match rest.strip_prefix('"') {
        Some(r) => r,
        None => return String::new(),
    };
    let mut out = String::new();
    let mut chars = rest.chars();
    while let Some(c) = chars.next() {
        match c {
            '\\' => out.extend(chars.next()),
            '"' => break,
            _ => out.push(c),
        }
    }
    out
}

impl KnownFinding {
    pub fn to_json(&self) -> String {
        format!(
            "{{\"id\":\"{}\",\"protocol\":\"{}\",\"title\":\"{}\",\"root_cause\":\"{}\",\"surface\":\"{}\",\"mechanism\":\"{}\",\"sink\":\"{}\",\"impact\":\"{}\",\"source\":\"{}\"}}",
            esc(&self.id), esc(&self.protocol), esc(&self.title), esc(&self.root_cause),
            esc(&self.surface), esc(&self.mechanism), esc(&self.sink), esc(&self.impact), esc(&self.source)
        )
    }

    pub fn from_json(line: &str) -> Option<Self> {
        if !line.contains("\"title\":") {
            return None;
        }
        Some(KnownFinding {
            id: field(line, "id"),
            protocol: field(line, "protocol"),
            title: field(line, "title"),
            root_cause: field(line, "root_cause"),
            surface: field(line, "surface"),
            mechanism: field(line, "mechanism"),
            sink: field(line, "sink"),
            impact: field(line, "impact"),
            source: field(line, "source"),
        })
    }

    /// Everything matchable, lowercased into one haystack.
    fn haystack(&self) -> String {
        format!(
            "{} {} {} {} {} {}",
            self.title, self.root_cause, self.surface, self.mechanism, self.sink, self.impact
        )
        .to_lowercase()
    }
}

pub fn register_path(ledger_dir: &Path, protocol: &str) -> std::path::PathBuf {
    let slug: String = protocol
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect();
    ledger_dir.join("known").join(format!("{slug}.jsonl"))
}

pub fn add(ledger_dir: &Path, f: &KnownFinding) -> std::io::Result<()> {
    let p = register_path(ledger_dir, &f.protocol);
    if let Some(parent) = p.parent() {
        std::fs::create_dir_all(parent)?;
    }
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new().create(true).append(true).open(&p)?;
    writeln!(file, "{}", f.to_json())
}

pub fn load(ledger_dir: &Path, protocol: &str) -> Vec<KnownFinding> {
    let p = register_path(ledger_dir, protocol);
    std::fs::read_to_string(p)
        .map(|s| s.lines().filter_map(KnownFinding::from_json).collect())
        .unwrap_or_default()
}

/// Split into distinctive word tokens (drops short/common noise).
fn tokens(s: &str) -> Vec<String> {
    const STOP: &[&str] = &[
        "the", "and", "for", "with", "that", "this", "from", "into", "when", "can",
        "not", "are", "was", "any", "all", "via", "out", "its", "may", "but",
        "a", "an", "of", "in", "to", "is", "on", "by", "or", "be", "as", "it",
    ];
    s.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.len() >= 3 && !STOP.contains(w))
        .map(|w| w.to_string())
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// Matched a prior finding strongly enough to kill before PoC.
    Known,
    /// Enough overlap to demand a manual read, not enough to auto-kill.
    Review,
    /// No meaningful overlap with the register.
    Novel,
}

impl Verdict {
    pub fn as_str(&self) -> &'static str {
        match self {
            Verdict::Known => "KNOWN",
            Verdict::Review => "REVIEW",
            Verdict::Novel => "NOVEL",
        }
    }
}

pub struct MatchResult {
    pub verdict: Verdict,
    pub score: f64,
    pub best: Option<KnownFinding>,
    pub why: String,
}

/// Two-factor match, mirroring corpus.rs precision discipline:
///   factor A — the candidate's mechanism/sink/surface names overlap the entry's
///   factor B — the root-cause vocabulary overlaps
/// A KNOWN verdict needs BOTH to be strong, because a false KNOWN kills a real,
/// payable finding. REVIEW fires when one factor is strong (a human should read
/// the cited report), and everything else is NOVEL.
pub fn check(
    register: &[KnownFinding],
    mechanism: &str,
    sink: &str,
    root_cause: &str,
    surface: &str,
) -> MatchResult {
    let cand_ident: Vec<String> = tokens(&format!("{mechanism} {sink} {surface}"));
    let cand_cause: Vec<String> = tokens(root_cause);

    let overlap = |a: &[String], b: &[String]| -> f64 {
        if a.is_empty() || b.is_empty() {
            return 0.0;
        }
        let hits = a.iter().filter(|t| b.contains(t)).count();
        hits as f64 / a.len().min(b.len()) as f64
    };

    let mut best: Option<KnownFinding> = None;
    let mut best_score = 0.0f64;
    let mut best_a = 0.0;
    let mut best_b = 0.0;

    for e in register {
        let entry_ident = tokens(&format!("{} {} {}", e.mechanism, e.sink, e.surface));
        let entry_cause = tokens(&format!("{} {}", e.title, e.root_cause));
        let a = overlap(&cand_ident, &entry_ident);
        let b = overlap(&cand_cause, &entry_cause);
        // Also credit an exact mechanism or sink string equality — the strongest
        // possible identifier signal.
        let exact = (!mechanism.is_empty() && e.mechanism.eq_ignore_ascii_case(mechanism))
            || (!sink.is_empty() && e.sink.eq_ignore_ascii_case(sink));
        let a = if exact { a.max(0.9) } else { a };
        let score = 0.5 * a + 0.5 * b;
        if score > best_score {
            best_score = score;
            best_a = a;
            best_b = b;
            best = Some(e.clone());
        }
    }

    let verdict = if best_a >= 0.5 && best_b >= 0.34 {
        Verdict::Known
    } else if best_a >= 0.5 || best_b >= 0.5 {
        Verdict::Review
    } else {
        Verdict::Novel
    };

    let why = match (&verdict, &best) {
        (Verdict::Known, Some(e)) => format!(
            "matches prior finding [{}] \"{}\" ({}) — ident {:.0}% / root-cause {:.0}%. DEAD-DUP: do not forge a PoC.",
            e.id, e.title, e.source, best_a * 100.0, best_b * 100.0
        ),
        (Verdict::Review, Some(e)) => format!(
            "partial overlap with [{}] \"{}\" ({}) — read that finding before proving this one.",
            e.id, e.title, e.source
        ),
        _ => "no meaningful overlap with the protocol register — novel against known findings (still run the world-search).".into(),
    };

    MatchResult { verdict, score: best_score, best, why }
}

/// Render the register as a Phase-0 "already known — DO NOT CHASE" brief,
/// suitable to hand to `pack --brief`.
pub fn brief(register: &[KnownFinding], protocol: &str) -> String {
    let mut s = format!("# Already known — DO NOT CHASE ({protocol})\n\n");
    if register.is_empty() {
        s.push_str("(register empty — build it from the program's audit reports before hunting)\n");
        return s;
    }
    s.push_str("Extracted from prior audits/known-issues. A candidate matching any class below is a duplicate — it pays zero and a PoC for it is pure waste.\n\n");
    for e in register {
        s.push_str(&format!(
            "- **{}** ({}) — {}. surface: {}. [{}]\n",
            if e.title.is_empty() { "(untitled)" } else { &e.title },
            e.source,
            if e.root_cause.is_empty() { "no root cause recorded" } else { &e.root_cause },
            if e.surface.is_empty() { "?" } else { &e.surface },
            e.id
        ));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cork_register() -> Vec<KnownFinding> {
        vec![
            KnownFinding {
                id: "cork-213".into(),
                protocol: "cork".into(),
                title: "redeemEarlyLv can be permanently DoS'd".into(),
                root_cause: "attacker locks redemption by leaving a dust position that reverts the early redeem path".into(),
                surface: "LvVault.redeemEarlyLv".into(),
                mechanism: "redeemEarlyLv-DoS".into(),
                sink: "LvVault".into(),
                impact: "redemption temporarily bricked".into(),
                source: "Cork known-issues #213".into(),
            },
            KnownFinding {
                id: "cork-audit-7".into(),
                protocol: "cork".into(),
                title: "first-depositor share inflation in PSM".into(),
                root_cause: "empty vault donation inflates share price".into(),
                surface: "Psm.deposit".into(),
                mechanism: "share-inflation".into(),
                sink: "Psm".into(),
                impact: "second depositor loses funds".into(),
                source: "Cork audit 2025".into(),
            },
        ]
    }

    #[test]
    fn the_cork_213_duplicate_is_caught_before_poc() {
        // The exact scenario that cost a full PoC forge: the candidate the tool
        // found autonomously must resolve to KNOWN against the register.
        let reg = cork_register();
        let r = check(
            &reg,
            "redeemEarlyLv-DoS",
            "LvVault",
            "attacker bricks early redemption by leaving a dust remainder that reverts",
            "LvVault.redeemEarlyLv",
        );
        assert_eq!(r.verdict, Verdict::Known, "{}", r.why);
        assert_eq!(r.best.unwrap().id, "cork-213");
    }

    #[test]
    fn an_unrelated_candidate_stays_novel() {
        let reg = cork_register();
        let r = check(
            &reg,
            "oracle-staleness",
            "PriceFeed",
            "chainlink round not checked for staleness before use in liquidation",
            "Liquidator.liquidate",
        );
        assert_eq!(r.verdict, Verdict::Novel, "{}", r.why);
    }

    #[test]
    fn exact_mechanism_match_is_known_even_with_terse_root_cause() {
        let reg = cork_register();
        let r = check(&reg, "share-inflation", "Psm", "empty vault donation share price", "Psm.deposit");
        assert_eq!(r.verdict, Verdict::Known, "{}", r.why);
    }

    #[test]
    fn jsonl_roundtrips() {
        let f = &cork_register()[0];
        let back = KnownFinding::from_json(&f.to_json()).unwrap();
        assert_eq!(back.id, f.id);
        assert_eq!(back.mechanism, f.mechanism);
        assert_eq!(back.root_cause, f.root_cause);
    }
}
