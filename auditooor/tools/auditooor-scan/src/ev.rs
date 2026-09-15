//! EV gate — the reward-first law as a HARD verdict, not advice.
//!
//! WEAK JOINT THIS CLOSES (field-reported): on Charm the ranking correctly said
//! "$10k cap, 4 audits, 5y live, patched = fortress", but the fleet still ran
//! because nothing STOPPED it. Spending a full multi-agent fleet on a $10k
//! picked-clean core is the exact opposite of dollars-per-run. This module turns
//! the target profile into `ABORT / SCOPE-ONLY / PROCEED` before any agent
//! spawns, so the reward-first rule is enforced by the tool, not left to nerve.
//!
//! It is deliberately pessimistic: on a bounty the base rate of a full fleet
//! landing a *payable, novel, proven* bug is low, and audits/age/crowding drive
//! it lower. Fresh code and un-audited seam value are the only real lifts.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// Negative or trivial dollars-per-run. Do not spawn the fleet.
    Abort,
    /// Marginal. Cheap Phase-0/seams recon only; escalate to a fleet ONLY if
    /// recon surfaces a concrete un-audited value seam.
    ScopeOnly,
    /// Worth a full run.
    Proceed,
}

impl Verdict {
    pub fn as_str(&self) -> &'static str {
        match self {
            Verdict::Abort => "ABORT",
            Verdict::ScopeOnly => "SCOPE-ONLY",
            Verdict::Proceed => "PROCEED",
        }
    }
}

pub struct EvInputs {
    pub cap_usd: f64,
    pub audits: u32,
    pub age_years: f64,
    pub crowded: bool,
    pub fresh_code: bool,
    pub seam_value: bool,
    /// Dollar-equivalent cost of committing a full fleet run (token+time+opportunity).
    pub fleet_cost_usd: f64,
}

pub struct EvResult {
    pub verdict: Verdict,
    pub p_find: f64,
    pub ev_usd: f64,
    pub fortress_score: f64,
    pub rationale: Vec<String>,
}

/// Base probability that a full fleet lands a payable+novel+proven bug on an
/// average in-scope target. Honest and low.
const P_BASE: f64 = 0.12;

pub fn evaluate(inp: &EvInputs) -> EvResult {
    let mut r: Vec<String> = Vec::new();
    let mut p = P_BASE;

    // Audits: each prior audit picks the core cleaner. Multiplicative decay.
    if inp.audits > 0 {
        let m = 0.7_f64.powi(inp.audits as i32);
        p *= m;
        r.push(format!("{} prior audit(s) → ×{:.2} (audited core is picked)", inp.audits, m));
    }
    // Age: older deployments have been swept more.
    if inp.age_years > 0.0 {
        let m = 1.0 / (1.0 + 0.15 * inp.age_years);
        p *= m;
        r.push(format!("{:.1}y live → ×{:.2} (older = more swept)", inp.age_years, m));
    }
    // Crowding: first-valid-submission means concurrent dups dominate rejects.
    if inp.crowded {
        p *= 0.4;
        r.push("crowded/hot program → ×0.40 (concurrent-dup risk)".into());
    }
    // Fresh code: launch/DIFF window is a bug cluster and briefly uncrowded.
    if inp.fresh_code {
        p *= 1.8;
        r.push("fresh/recently-shipped code → ×1.80 (bug cluster, short uncrowded window)".into());
    }
    // Un-audited seam holding reachable value: the sharpest uncrowded lane.
    if inp.seam_value {
        p *= 1.6;
        r.push("un-audited value seam present → ×1.60 (fortress periphery, unswept)".into());
    }
    p = p.clamp(0.005, 0.6);

    // Fortress score is a human-readable summary of the same signals.
    let fortress_score = (inp.audits as f64) * 2.0 + inp.age_years
        + if inp.crowded { 3.0 } else { 0.0 }
        - if inp.fresh_code { 4.0 } else { 0.0 }
        - if inp.seam_value { 3.0 } else { 0.0 };

    let ev = inp.cap_usd * p - inp.fleet_cost_usd;
    r.push(format!(
        "EV = cap ${:.0} × P {:.3} − fleet ${:.0} = ${:.0}",
        inp.cap_usd, p, inp.fleet_cost_usd, ev
    ));

    // ---- verdict: hard reward-first rules ----
    let is_fortress = inp.audits >= 3 && inp.age_years >= 3.0 && !inp.fresh_code && !inp.seam_value;

    let verdict = if is_fortress && inp.cap_usd < 50_000.0 {
        // The Charm profile: low cap + multi-audited + old + no fresh/seam lift.
        r.push("HARD KILL: low-cap multi-audited old core, no fresh-code / seam lift → the Charm fortress profile.".into());
        Verdict::Abort
    } else if ev <= 0.0 {
        r.push("Negative dollars-per-run.".into());
        Verdict::Abort
    } else if ev < 3.0 * inp.fleet_cost_usd || (inp.cap_usd < 25_000.0 && (inp.audits >= 2 || inp.age_years >= 3.0)) {
        r.push("Marginal EV — recon-only; escalate to a fleet ONLY if a concrete un-audited seam is found.".into());
        Verdict::ScopeOnly
    } else {
        r.push("Positive dollars-per-run clears the bar.".into());
        Verdict::Proceed
    };

    EvResult { verdict, p_find: p, ev_usd: ev, fortress_score, rationale: r }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> EvInputs {
        EvInputs { cap_usd: 100_000.0, audits: 0, age_years: 0.0, crowded: false,
            fresh_code: false, seam_value: false, fleet_cost_usd: 200.0 }
    }

    #[test]
    fn charm_fortress_aborts() {
        let inp = EvInputs { cap_usd: 10_000.0, audits: 4, age_years: 5.0, crowded: true,
            fresh_code: false, seam_value: false, fleet_cost_usd: 200.0 };
        assert_eq!(evaluate(&inp).verdict, Verdict::Abort);
    }

    #[test]
    fn fresh_high_cap_proceeds() {
        let inp = EvInputs { cap_usd: 500_000.0, fresh_code: true, ..base() };
        assert_eq!(evaluate(&inp).verdict, Verdict::Proceed);
    }

    #[test]
    fn seam_value_rescues_an_audited_target() {
        // Audited + old, but a real un-audited value seam and a decent cap.
        let inp = EvInputs { cap_usd: 250_000.0, audits: 3, age_years: 4.0,
            seam_value: true, ..base() };
        // Not the hard-kill (seam lift present) and EV should clear.
        assert_ne!(evaluate(&inp).verdict, Verdict::Abort);
    }

    #[test]
    fn low_cap_audited_is_scope_or_abort_not_proceed() {
        let inp = EvInputs { cap_usd: 20_000.0, audits: 2, age_years: 2.0, ..base() };
        assert_ne!(evaluate(&inp).verdict, Verdict::Proceed);
    }
}
