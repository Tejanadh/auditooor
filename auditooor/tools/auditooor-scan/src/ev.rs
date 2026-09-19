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
    /// Cost of the LITE pipeline. `None` = `fleet_cost_usd * LITE_COST_FRACTION`.
    pub lite_cost_usd: Option<f64>,
    /// In-scope source lines, when known. A fleet cannot out-read three focused
    /// hunters on a small target — it just re-reads the same code.
    pub target_nsloc: Option<u32>,
}

/// Which pipeline the run should actually buy. The verdict says "is this target
/// worth anything"; the mode says "how much machine do you get to spend on it".
/// LITE is the default because on nearly every profile the marginal bugs a full
/// fleet adds are worth less than the tokens it burns to add them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Do not hunt.
    Abort,
    /// Phase 0/1 recon + <=3 hunters on the focus set. The default.
    Lite,
    /// Full fleet + coverage wave + skeptics. Has to be *earned*.
    Deep,
}

impl Mode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Mode::Abort => "ABORT",
            Mode::Lite => "LITE",
            Mode::Deep => "DEEP",
        }
    }
}

pub struct EvResult {
    pub verdict: Verdict,
    pub mode: Mode,
    pub p_find: f64,
    /// P(payable bug) for the LITE pipeline — a fraction of the fleet's reach.
    pub p_find_lite: f64,
    pub ev_usd: f64,
    pub ev_lite_usd: f64,
    pub fortress_score: f64,
    pub rationale: Vec<String>,
}

/// Base probability that a full fleet lands a payable+novel+proven bug on an
/// average in-scope target. Honest and low.
const P_BASE: f64 = 0.12;

/// Share of the fleet's reach that 3 focused hunters on the ranked focus set
/// retain. Calibration note: across the recorded runs in `benchmark/CALIBRATION.md`
/// every candidate the full fleet produced came from the top-ranked files the
/// LITE focus set already contains, so this is generous to DEEP, not to LITE.
const LITE_REACH: f64 = 0.55;

/// Default dollar-equivalent cost of a LITE run relative to a fleet run.
pub const LITE_COST_FRACTION: f64 = 0.12;

/// Below this many in-scope lines, DEEP buys nothing: fifteen agents and three
/// agents read the same files, so the fleet's extra "reach" is fifteen opinions
/// on one small surface rather than more surface covered.
///
/// Field-reported (Fluid periphery, 2026-09-19): the gate returned `mode: DEEP`
/// on a 2k-line scope purely because the cap was $500k and the code was fresh.
/// Cap size is not scope size.
pub const DEEP_MIN_NSLOC: u32 = 3_000;

/// Above this fortress score the code is swept: no cap is large enough to make a
/// full fleet the right purchase, because the marginal bugs it would find have
/// already been found by the audits that produced the score.
pub const FORTRESS_DEEP_CEILING: f64 = 6.0;

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

    // ---- mode: what pipeline the EV actually pays for ----
    let lite_cost = inp.lite_cost_usd.unwrap_or(inp.fleet_cost_usd * LITE_COST_FRACTION);
    let p_lite = p * LITE_REACH;
    let ev_lite = inp.cap_usd * p_lite - lite_cost;
    // DEEP is bought only when the *marginal* reach it adds clears 3x its
    // marginal cost. Anything else is LITE: same gates, a fraction of the burn.
    let marginal_gain = inp.cap_usd * (p - p_lite);
    let marginal_cost = (inp.fleet_cost_usd - lite_cost).max(1.0);
    // A fleet only pays on code that has not already been swept. Dollars alone
    // are not enough: a $250k cap on a 4x-audited, crowded, 2-year-old core
    // clears any marginal-dollar test while being exactly the profile where a
    // full fleet has historically found nothing (benchmark/CALIBRATION.md).
    // Caught by dogfooding v0.8 target selection.
    let swept = fortress_score >= FORTRESS_DEEP_CEILING;
    let too_small = inp.target_nsloc.map(|n| n < DEEP_MIN_NSLOC).unwrap_or(false);
    let mode = match verdict {
        Verdict::Abort => Mode::Abort,
        _ if too_small => {
            r.push(format!(
                "LITE forced: {} in-scope lines (< {}). A fleet re-reads the same files — buy depth with a bigger scope, not more agents.",
                inp.target_nsloc.unwrap_or(0),
                DEEP_MIN_NSLOC
            ));
            Mode::Lite
        }
        _ if swept => {
            r.push(format!(
                "LITE forced: fortress_score {:.1} >= {:.1} — picked-clean code, a fleet adds reach nobody can use.",
                fortress_score, FORTRESS_DEEP_CEILING
            ));
            Mode::Lite
        }
        _ if marginal_gain > 3.0 * marginal_cost && verdict == Verdict::Proceed => {
            r.push(format!(
                "DEEP earned: marginal reach ${:.0} > 3x marginal cost ${:.0}.",
                marginal_gain, marginal_cost
            ));
            Mode::Deep
        }
        _ => {
            r.push(format!(
                "LITE (default): EV_lite ${:.0} at ${:.0} vs fleet ${:.0} at ${:.0} — the fleet's extra reach does not pay for itself.",
                ev_lite, lite_cost, ev, inp.fleet_cost_usd
            ));
            Mode::Lite
        }
    };

    EvResult {
        verdict,
        mode,
        p_find: p,
        p_find_lite: p_lite,
        ev_usd: ev,
        ev_lite_usd: ev_lite,
        fortress_score,
        rationale: r,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> EvInputs {
        EvInputs { cap_usd: 100_000.0, audits: 0, age_years: 0.0, crowded: false,
            fresh_code: false, seam_value: false, fleet_cost_usd: 200.0, lite_cost_usd: None,
            target_nsloc: None }
    }

    #[test]
    fn charm_fortress_aborts() {
        let inp = EvInputs { cap_usd: 10_000.0, audits: 4, age_years: 5.0, crowded: true,
            fresh_code: false, seam_value: false, fleet_cost_usd: 200.0, lite_cost_usd: None,
            target_nsloc: None };
        assert_eq!(evaluate(&inp).verdict, Verdict::Abort);
        assert_eq!(evaluate(&inp).mode, Mode::Abort);
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
    fn default_mode_is_lite_on_an_ordinary_target() {
        // $100k cap, 4 audits, 1.5y, crowded: real EV, but the fleet's extra
        // reach is worth less than the tokens it costs.
        let inp = EvInputs { cap_usd: 100_000.0, audits: 4, age_years: 1.5, crowded: true, ..base() };
        assert_eq!(evaluate(&inp).mode, Mode::Lite);
    }

    #[test]
    fn a_huge_cap_does_not_buy_a_fleet_on_swept_code() {
        // $3M cap, 5 audits, 2y live, crowded: EV is real, but this is exactly
        // the profile where fleets have found nothing. LITE, not DEEP.
        let inp = EvInputs { cap_usd: 3_000_000.0, audits: 5, age_years: 2.0, crowded: true, ..base() };
        let res = evaluate(&inp);
        assert_eq!(res.verdict, Verdict::Proceed);
        assert_eq!(res.mode, Mode::Lite);
    }

    #[test]
    fn deep_is_earned_only_by_a_big_uncrowded_cap() {
        let inp = EvInputs { cap_usd: 1_000_000.0, fresh_code: true, seam_value: true, ..base() };
        assert_eq!(evaluate(&inp).mode, Mode::Deep);
    }

    #[test]
    fn a_small_scope_cannot_buy_a_fleet() {
        // Fluid periphery: $500k cap, fresh, un-audited seam — EV said DEEP on a
        // ~2k-line scope. Cap size is not scope size.
        let big = EvInputs { cap_usd: 500_000.0, audits: 1, age_years: 1.0, crowded: true,
            seam_value: true, target_nsloc: Some(30_000), ..base() };
        assert_eq!(evaluate(&big).mode, Mode::Deep);
        let small = EvInputs { target_nsloc: Some(2_000), ..big };
        assert_eq!(evaluate(&small).mode, Mode::Lite);
    }

    #[test]
    fn lite_ev_is_reported_and_positive_when_proceeding() {
        let res = evaluate(&EvInputs { cap_usd: 500_000.0, fresh_code: true, ..base() });
        assert!(res.ev_lite_usd > 0.0);
        assert!(res.p_find_lite < res.p_find);
    }

    #[test]
    fn low_cap_audited_is_scope_or_abort_not_proceed() {
        let inp = EvInputs { cap_usd: 20_000.0, audits: 2, age_years: 2.0, ..base() };
        assert_ne!(evaluate(&inp).verdict, Verdict::Proceed);
    }
}
