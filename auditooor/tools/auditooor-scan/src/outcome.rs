//! Outcome ledger — the only instrument that measures lead-to-payout.
//!
//! Every other number in this tool is recall or clean-corpus FP: measured against
//! labeled code. None of it can tell you the in-the-wild precision (dead leads per
//! real bug) or calibrate `human_EV`'s P(model an un-modeled assumption). Only real
//! submits landing or bouncing do that — and this is where they get recorded. It
//! fills by hunting, not by building. Append one row per real outcome; the stats
//! turn a pile of vibes into the two numbers the rest of the system is missing.

use std::fs;
use std::path::Path;

/// Append one outcome row (TSV) to the ledger, creating it if needed.
/// Fields: iso_ts, protocol, mechanism, sink, lane, status, payout_usd, notes.
pub fn record(
    ledger: &Path,
    ts: &str,
    protocol: &str,
    mechanism: &str,
    sink: &str,
    lane: &str,
    status: &str,
    payout_usd: u64,
    notes: &str,
) -> std::io::Result<()> {
    use std::io::Write;
    if let Some(p) = ledger.parent() {
        let _ = fs::create_dir_all(p);
    }
    let mut f = fs::OpenOptions::new().create(true).append(true).open(ledger)?;
    let clean = |s: &str| s.replace(['\t', '\n'], " ");
    writeln!(
        f,
        "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
        clean(ts),
        clean(protocol),
        clean(mechanism),
        clean(sink),
        clean(lane),
        clean(status),
        payout_usd,
        clean(notes)
    )
}

/// Aggregate stats over the ledger — the calibration the vibe-terms need.
#[derive(Debug, Default, PartialEq)]
pub struct Stats {
    pub submitted: u32,
    pub accepted: u32,
    pub rejected: u32,
    pub duplicate: u32,
    pub no_response: u32,
    pub total_payout: u64,
    pub machine_lane: u32,
    pub human_lane: u32,
    pub machine_accepted: u32,
    pub human_accepted: u32,
}

impl Stats {
    /// In-the-wild precision proxy: accepted / (all resolved submissions).
    /// The number DeFiHackLabs recall can never give you.
    pub fn in_wild_precision(&self) -> f64 {
        let resolved = self.accepted + self.rejected + self.duplicate;
        if resolved == 0 { 0.0 } else { self.accepted as f64 / resolved as f64 }
    }
    /// Per-lane acceptance — calibrates machine_EV vs human_EV over time.
    pub fn machine_hit_rate(&self) -> f64 {
        if self.machine_lane == 0 { 0.0 } else { self.machine_accepted as f64 / self.machine_lane as f64 }
    }
    pub fn human_hit_rate(&self) -> f64 {
        if self.human_lane == 0 { 0.0 } else { self.human_accepted as f64 / self.human_lane as f64 }
    }
}

/// Parse the ledger into aggregate stats. Missing file = empty stats.
pub fn stats(ledger: &Path) -> Stats {
    let mut s = Stats::default();
    let body = match fs::read_to_string(ledger) {
        Ok(b) => b,
        Err(_) => return s,
    };
    for line in body.lines() {
        let f: Vec<&str> = line.split('\t').collect();
        if f.len() < 8 {
            continue;
        }
        let lane = f[4];
        let status = f[5];
        let payout: u64 = f[6].parse().unwrap_or(0);
        s.submitted += 1;
        s.total_payout += payout;
        let accepted = status == "accepted";
        match status {
            "accepted" => s.accepted += 1,
            "rejected" => s.rejected += 1,
            "duplicate" => s.duplicate += 1,
            "no_response" => s.no_response += 1,
            _ => {}
        }
        match lane {
            "machine" => {
                s.machine_lane += 1;
                if accepted { s.machine_accepted += 1; }
            }
            "human" => {
                s.human_lane += 1;
                if accepted { s.human_accepted += 1; }
            }
            _ => {}
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_and_aggregates() {
        let dir = std::env::temp_dir().join(format!("auditooor-outcome-{}", std::process::id()));
        let _ = fs::create_dir_all(&dir);
        let led = dir.join("outcomes.tsv");
        let _ = fs::remove_file(&led);
        record(&led, "2026-08-28", "Acme", "share inflation", "totalSupply", "human", "accepted", 250_000, "seam").unwrap();
        record(&led, "2026-08-28", "Acme", "rounding", "redeem", "machine", "rejected", 0, "dup risk").unwrap();
        record(&led, "2026-08-28", "Beta", "reentrancy", "withdraw", "machine", "duplicate", 0, "").unwrap();
        let s = stats(&led);
        assert_eq!(s.submitted, 3);
        assert_eq!(s.accepted, 1);
        assert_eq!(s.total_payout, 250_000);
        // 1 accepted / 3 resolved
        assert!((s.in_wild_precision() - 1.0 / 3.0).abs() < 1e-9);
        assert_eq!(s.human_lane, 1);
        assert!((s.human_hit_rate() - 1.0).abs() < 1e-9);
        assert!((s.machine_hit_rate() - 0.0).abs() < 1e-9);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn missing_ledger_is_empty() {
        let s = stats(Path::new("/nonexistent/outcomes.tsv"));
        assert_eq!(s.submitted, 0);
        assert_eq!(s.in_wild_precision(), 0.0);
    }
}
