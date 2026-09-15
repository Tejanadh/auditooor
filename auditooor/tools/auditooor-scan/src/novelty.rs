//! World-novelty lookup planner.
//!
//! The `fingerprint` ledger only catches SELF-duplication. World-novelty — is this
//! bug already public, in a prior audit, in the program's known-issues, or already
//! reported by someone else? — is the hardest pillar and the #1 live-reject reason.
//!
//! This binary is std-only and cannot call the web. What it CAN do is generate the
//! exact, targeted prior-art query set + a worksheet, so the LLM layer executes a
//! disciplined world-novelty search (via WebSearch) instead of hand-waving it.

/// One targeted prior-art query for a specific venue.
#[derive(Debug, Clone, PartialEq)]
pub struct NoveltyQuery {
    pub venue: String,
    pub query: String,
}

fn norm(s: &str) -> String {
    s.trim().split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Generate the prior-art query set for a candidate. `fork_family` (e.g.
/// "Uniswap V2", "Compound") enables the fork-inheritance check — a bug present
/// unchanged in the upstream is almost certainly already claimed.
pub fn generate_queries(
    protocol: &str,
    mechanism: &str,
    sink: &str,
    fork_family: Option<&str>,
) -> Vec<NoveltyQuery> {
    let p = norm(protocol);
    let m = norm(mechanism);
    let s = norm(sink);
    let mut q = vec![
        NoveltyQuery {
            venue: "disclosed-reports".into(),
            query: format!("{p} {m} vulnerability report immunefi OR code4rena OR sherlock"),
        },
        NoveltyQuery {
            venue: "protocol-audits".into(),
            query: format!("{p} audit report {m} finding"),
        },
        NoveltyQuery {
            venue: "github-issues-prs".into(),
            query: format!("{p} {m} {s} issue OR pull request fix"),
        },
        NoveltyQuery {
            venue: "mechanism-prior-art".into(),
            query: format!("{m} smart contract vulnerability writeup CVE"),
        },
        NoveltyQuery {
            venue: "hack-history".into(),
            query: format!("{p} exploit hack {m} {s}"),
        },
    ];
    if let Some(f) = fork_family {
        let f = norm(f);
        q.push(NoveltyQuery {
            venue: "fork-inheritance".into(),
            query: format!("{f} fork {m} known issue (present unchanged upstream = likely claimed)"),
        });
    }
    q
}

/// Emit the query set + the manual-check worksheet as JSON. `self_dedup` is the
/// local ledger verdict passed in by the caller. `corpus` is the baked-in
/// known-bug-class verdict: `Some((verdict, name, payability, prior_art))` on a
/// hit, `None` on a corpus miss.
#[allow(clippy::too_many_arguments)]
pub fn to_json(
    protocol: &str,
    mechanism: &str,
    sink: &str,
    fingerprint: &str,
    self_dedup: &str,
    corpus: Option<(&str, &str, &str, &str)>,
    queries: &[NoveltyQuery],
    esc: impl Fn(&str) -> String,
) -> String {
    let mut out = String::from("{\n");
    out.push_str(&format!("  \"protocol\": \"{}\",\n", esc(protocol)));
    out.push_str(&format!("  \"mechanism\": \"{}\",\n", esc(mechanism)));
    out.push_str(&format!("  \"sink\": \"{}\",\n", esc(sink)));
    out.push_str(&format!("  \"fingerprint\": \"{}\",\n", esc(fingerprint)));
    out.push_str(&format!("  \"self_dedup\": \"{}\",\n", esc(self_dedup)));
    match corpus {
        Some((verdict, name, payability, prior_art)) => {
            out.push_str(&format!("  \"corpus_verdict\": \"{}\",\n", esc(verdict)));
            out.push_str(&format!("  \"corpus_class\": \"{}\",\n", esc(name)));
            out.push_str(&format!("  \"corpus_payability\": \"{}\",\n", esc(payability)));
            out.push_str(&format!("  \"corpus_prior_art\": \"{}\",\n", esc(prior_art)));
        }
        None => {
            out.push_str("  \"corpus_verdict\": \"CORPUS-MISS\",\n");
            out.push_str("  \"corpus_note\": \"No known-class match. NOT proof of novelty — run the full world-search below.\",\n");
        }
    }
    out.push_str("  \"note\": \"self_dedup + corpus are LOCAL only. A corpus hit is a hard 'do not dress as novel / check payability'. A miss still requires running every query below via WebSearch before treating this as world-novel.\",\n");
    out.push_str("  \"queries\": [\n");
    for (i, nq) in queries.iter().enumerate() {
        if i > 0 {
            out.push_str(",\n");
        }
        out.push_str(&format!(
            "    {{\"venue\": \"{}\", \"query\": \"{}\"}}",
            esc(&nq.venue),
            esc(&nq.query)
        ));
    }
    out.push_str("\n  ],\n");
    out.push_str("  \"manual_checks\": [\n");
    out.push_str("    \"Read the program's own known-issues / previously-reported section (a match here = auto-close).\",\n");
    out.push_str("    \"Read every prior audit report of this protocol for the same mechanism.\",\n");
    out.push_str("    \"Check the protocol changelog/commits for a fix that post-dates the deployed version.\",\n");
    out.push_str("    \"If a fork: confirm the bug is not the upstream's already-known/accepted behavior.\"\n");
    out.push_str("  ]\n}");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_targeted_queries() {
        let q = generate_queries("Acme Vault", "share inflation", "totalSupply", None);
        assert_eq!(q.len(), 5);
        assert!(q.iter().any(|x| x.venue == "disclosed-reports" && x.query.contains("Acme Vault")));
        assert!(q.iter().all(|x| x.query.contains("share inflation")));
    }

    #[test]
    fn fork_family_adds_inheritance_check() {
        let q = generate_queries("SomeFork", "rounding direction", "redeem", Some("Uniswap V2"));
        assert_eq!(q.len(), 6);
        assert!(q.iter().any(|x| x.venue == "fork-inheritance" && x.query.contains("Uniswap V2")));
    }

    #[test]
    fn normalizes_whitespace() {
        let q = generate_queries("  Acme   Vault ", "  x ", "y", None);
        assert!(q[0].query.contains("Acme Vault"));
    }
}
