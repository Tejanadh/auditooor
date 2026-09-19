//! auditooor-scan CLI.
//!
//! Subcommands:
//!   detect <dir>                        → ecosystem inventory (JSON)
//!   surface <dir> [--top N]             → money-proximity impact map (JSON)
//!   xray <dir> [--top N]                → one-shot recon pack (JSON)
//!   entries <dir>                       → function-level entry census (JSON)
//!   fingerprint --mechanism M --sink S --entrypoint E [--ledger P] [--add] [--label L]
//!
//! Everything is JSON on stdout so the orchestrator can consume it directly.

use auditooor_scan::*;
use std::collections::BTreeMap;
use std::path::Path;
use std::process::exit;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: auditooor-scan <detect|surface|xray|pack|gate|entries|ev|novelty|known|confirm|harness|outcome|...> ...");
        exit(2);
    }
    match args[1].as_str() {
        "detect" => cmd_detect(&args[2..]),
        "surface" => cmd_surface(&args[2..]),
        "xray" => cmd_xray(&args[2..]),
        "pack" => cmd_pack(&args[2..]),
        "danger" => cmd_danger(&args[2..]),
        "entries" => cmd_entries(&args[2..]),
        "fingerprint" => cmd_fingerprint(&args[2..]),
        "harness" => cmd_harness(&args[2..]),
        "detectors" => cmd_detectors(&args[2..]),
        "novelty" => cmd_novelty(&args[2..]),
        "seams" => cmd_seams(&args[2..]),
        "outcome" => cmd_outcome(&args[2..]),
        "ev" => cmd_ev(&args[2..]),
        "gate" => cmd_gate(&args[2..]),
        "known" => cmd_known(&args[2..]),
        "confirm" => cmd_confirm(&args[2..]),
        "--version" | "-V" => println!("auditooor-scan {}", env!("CARGO_PKG_VERSION")),
        other => {
            eprintln!("unknown subcommand: {other}");
            exit(2);
        }
    }
}

fn flag<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
    args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).map(|s| s.as_str())
}

fn has_flag(args: &[String], name: &str) -> bool {
    args.iter().any(|a| a == name)
}

fn cmd_detect(args: &[String]) {
    let dir = args.iter().find(|a| !a.starts_with("--")).map(|s| s.as_str()).unwrap_or(".");
    let root = Path::new(dir);
    let scans = scan_repo(root);
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for s in &scans {
        *counts.entry(s.ecosystem.clone()).or_insert(0) += 1;
    }
    let mut out = String::from("{\n");
    out.push_str(&format!("  \"root\": \"{}\",\n", json_escape(&root.display().to_string())));
    out.push_str("  \"ecosystems\": {");
    let ecos: Vec<String> = counts
        .iter()
        .map(|(k, v)| format!("\"{}\": {}", json_escape(k), v))
        .collect();
    out.push_str(&ecos.join(", "));
    out.push_str("},\n");
    out.push_str(&format!("  \"file_count\": {},\n", scans.len()));
    let (total_complexity, tactic) = repo_maturity(&scans);
    out.push_str(&format!("  \"total_complexity\": {},\n", total_complexity));
    out.push_str(&format!("  \"tactic\": \"{}\",\n", json_escape(tactic)));
    // Route recommendation.
    let mut routes = Vec::new();
    for k in counts.keys() {
        let engine = match k.as_str() {
            "evm" => "critfindsaudit",
            "solana" => "critsolaudit",
            "zk" => "critzkaudit",
            "vyper" => "references/vyper-adapter.md",
            "move" => "references/move-adapter.md",
            _ => "manual",
        };
        routes.push(format!("\"{}\": \"{}\"", json_escape(k), engine));
    }
    out.push_str("  \"routes\": {");
    out.push_str(&routes.join(", "));
    out.push_str("}\n}");
    println!("{out}");
}

fn cmd_xray(args: &[String]) {
    let dir = args.iter().find(|a| !a.starts_with("--")).map(|s| s.as_str()).unwrap_or(".");
    let top: usize = flag(args, "--top").and_then(|v| v.parse().ok()).unwrap_or(20);
    let json = auditooor_scan::xray::generate(Path::new(dir), top);
    if let Some(out) = flag(args, "--out") {
        let agents = flag(args, "--agents").map(Path::new);
        let opts = pack_opts(args);
        match auditooor_scan::pack::write_pack(Path::new(dir), Path::new(out), &json, agents, &opts) {
            Ok(files) => {
                eprintln!("wrote recon pack -> {out} ({})", files.join(", "));
                print!("{json}");
            }
            Err(e) => {
                eprintln!("cannot write pack {out}: {e}");
                exit(1);
            }
        }
    } else {
        print!("{json}");
    }
}

/// Pack sizing. LITE is the default: 3 headline roles, top-8 focus set.
/// `--deep` buys every role and every file; `--roles`/`--focus` override either.
fn pack_opts(args: &[String]) -> auditooor_scan::pack::PackOpts {
    use auditooor_scan::pack::PackOpts;
    let mut opts = if has_flag(args, "--deep") { PackOpts::deep() } else { PackOpts::default() };
    if let Some(r) = flag(args, "--roles") {
        opts.roles = if r == "all" {
            None
        } else {
            Some(r.split(',').map(|x| x.trim().to_string()).filter(|x| !x.is_empty()).collect())
        };
    }
    if let Some(f) = flag(args, "--focus").and_then(|v| v.parse::<usize>().ok()) {
        opts.focus_files = f;
    }
    if has_flag(args, "--full-source") {
        opts.write_full_source = true;
    }
    if let Some(bf) = flag(args, "--brief") {
        match std::fs::read_to_string(bf) {
            Ok(t) => opts.brief = Some(t),
            Err(e) => {
                eprintln!("cannot read --brief {bf}: {e}");
                exit(2);
            }
        }
    }
    opts
}

fn cmd_pack(args: &[String]) {
    if !has_flag(args, "--out") {
        eprintln!("usage: auditooor-scan pack <dir> --out <recon-dir> [--agents DIR] [--deep] [--roles a,b,c|all] [--focus N] [--brief FILE] [--full-source]");
        eprintln!("  default (LITE): 3 headline bundles, top-8 focus set inlined as focus.md, rest a path manifest.");
        eprintln!("  LITE writes NO source.md on purpose — the whole tree on disk is a file the orchestrator wanders into.");
        eprintln!("  --brief FILE: Phase-0 prior-art brief, prepended to every bundle as \"already known, do not chase\".");
        exit(2);
    }
    cmd_xray(args);
}

fn cmd_danger(args: &[String]) {
    let dir = args.iter().find(|a| !a.starts_with("--")).map(|s| s.as_str()).unwrap_or(".");
    let hits = auditooor_scan::danger::scan(Path::new(dir));
    print!("{}", auditooor_scan::danger::to_json(&hits));
}

fn cmd_entries(args: &[String]) {
    use auditooor_scan::entries::scan_repo_entries;
    let dir = args.iter().find(|a| !a.starts_with("--")).map(|s| s.as_str()).unwrap_or(".");
    let ents = scan_repo_entries(Path::new(dir));
    let mut out = String::from("[\n");
    for (i, e) in ents.iter().enumerate() {
        if i > 0 {
            out.push_str(",\n");
        }
        let mods: Vec<String> = e
            .modifiers
            .iter()
            .map(|m| format!("\"{}\"", json_escape(m)))
            .collect();
        out.push_str(&format!(
            "  {{\"file\": \"{}\", \"contract\": \"{}\", \"name\": \"{}\", \"line\": {}, \"access\": \"{}\", \"value_flow\": \"{}\", \"payable\": {}, \"modifiers\": [{}]}}",
            json_escape(&e.file),
            json_escape(&e.contract),
            json_escape(&e.name),
            e.line,
            e.access.as_str(),
            json_escape(&e.value_flow),
            e.payable,
            mods.join(", ")
        ));
    }
    out.push_str("\n]");
    println!("{out}");
}

fn cmd_surface(args: &[String]) {
    let dir = args.iter().find(|a| !a.starts_with("--")).map(|s| s.as_str()).unwrap_or(".");
    let top: usize = flag(args, "--top").and_then(|v| v.parse().ok()).unwrap_or(20);
    let root = Path::new(dir);
    let scans = scan_repo(root);

    let mut out = String::from("[\n");
    for (idx, s) in scans.iter().take(top).enumerate() {
        if idx > 0 {
            out.push_str(",\n");
        }
        out.push_str("  {\n");
        out.push_str(&format!("    \"path\": \"{}\",\n", json_escape(&s.path)));
        out.push_str(&format!("    \"ecosystem\": \"{}\",\n", json_escape(&s.ecosystem)));
        out.push_str(&format!("    \"risk_score\": {},\n", s.risk_score));
        out.push_str(&format!("    \"money_score\": {},\n", s.score));
        out.push_str(&format!("    \"complexity\": {},\n", s.complexity));
        out.push_str(&format!("    \"detector_hits\": {},\n", s.detector_hits));
        out.push_str(&format!("    \"entrypoints\": {},\n", s.entrypoints));
        let perm = s.value_sites.iter().filter(|v| v.permissionless_hint).count();
        out.push_str(&format!("    \"permissionless_value_sites\": {},\n", perm));
        out.push_str("    \"value_sites\": [");
        let sites: Vec<String> = s
            .value_sites
            .iter()
            .take(12)
            .map(|v| {
                format!(
                    "{{\"line\": {}, \"keyword\": \"{}\", \"permissionless\": {}, \"path_complexity\": {}}}",
                    v.line,
                    json_escape(&v.keyword),
                    v.permissionless_hint,
                    v.path_complexity
                )
            })
            .collect();
        out.push_str(&sites.join(", "));
        out.push_str("]\n  }");
    }
    out.push_str("\n]");
    println!("{out}");
}

fn cmd_outcome(args: &[String]) {
    use auditooor_scan::outcome::{record, stats};
    let default_ledger = format!(
        "{}/.claude/auditooor/outcomes.tsv",
        std::env::var("HOME").unwrap_or_else(|_| ".".into())
    );
    let ledger = flag(args, "--ledger").unwrap_or(&default_ledger);
    let lp = Path::new(ledger);

    if has_flag(args, "--stats") {
        let s = stats(lp);
        println!(
            "{{\"submitted\": {}, \"accepted\": {}, \"rejected\": {}, \"duplicate\": {}, \"no_response\": {}, \"total_payout_usd\": {}, \"in_wild_precision\": {:.3}, \"machine_hit_rate\": {:.3}, \"human_hit_rate\": {:.3}}}",
            s.submitted, s.accepted, s.rejected, s.duplicate, s.no_response, s.total_payout,
            s.in_wild_precision(), s.machine_hit_rate(), s.human_hit_rate()
        );
        return;
    }

    let protocol = flag(args, "--protocol").unwrap_or("");
    let mechanism = flag(args, "--mechanism").unwrap_or("");
    let sink = flag(args, "--sink").unwrap_or("");
    let lane = flag(args, "--lane").unwrap_or("machine");
    let status = flag(args, "--status").unwrap_or("");
    if protocol.is_empty() || status.is_empty() {
        eprintln!("usage: auditooor-scan outcome --protocol P --status submitted|accepted|rejected|duplicate|no_response [--mechanism M --sink S --lane machine|human --payout N --notes ..] | --stats");
        exit(2);
    }
    let payout: u64 = flag(args, "--payout").and_then(|v| v.parse().ok()).unwrap_or(0);
    let notes = flag(args, "--notes").unwrap_or("");
    let today = utc_date();
    let ts = flag(args, "--ts").unwrap_or(&today);
    match record(lp, ts, protocol, mechanism, sink, lane, status, payout, notes) {
        Ok(()) => eprintln!("recorded {status} outcome for {protocol} -> {ledger}"),
        Err(e) => {
            eprintln!("cannot write {ledger}: {e}");
            exit(1);
        }
    }
}

fn cmd_seams(args: &[String]) {
    use auditooor_scan::seams::analyze_all;
    let dir = args.iter().find(|a| !a.starts_with("--")).map(|s| s.as_str()).unwrap_or(".");
    let mut contracts: Vec<(String, String)> = Vec::new();
    for f in auditooor_scan::walk_files(Path::new(dir)) {
        if f.extension().and_then(|e| e.to_str()) != Some("sol") {
            continue;
        }
        let src = match std::fs::read_to_string(&f) {
            Ok(s) => s,
            Err(_) => continue,
        };
        let name = src.lines().find_map(|l| {
            l.trim().strip_prefix("contract ").map(|r| {
                r.split(|c: char| c.is_whitespace() || c == '{').next().unwrap_or("C").to_string()
            })
        });
        if let Some(name) = name {
            contracts.push((name, src));
        }
    }
    let scores = analyze_all(&contracts);
    let mut out = String::from("[\n");
    for (i, s) in scores.iter().enumerate() {
        if i > 0 {
            out.push_str(",\n");
        }
        let reasons: Vec<String> = s.reasons.iter().map(|r| format!("\"{}\"", json_escape(r))).collect();
        out.push_str(&format!(
            "  {{\"contract\": \"{}\", \"classification\": \"{}\", \"periphery_score\": {}, \"local_imports\": {}, \"external_calls\": {}, \"state_slots\": {}, \"reasons\": [{}]}}",
            json_escape(&s.contract),
            json_escape(&s.classification),
            s.periphery_score,
            s.local_imports,
            s.external_calls,
            s.state_slots,
            reasons.join(", ")
        ));
    }
    out.push_str("\n]");
    println!("{out}");
}

fn cmd_novelty(args: &[String]) {
    use auditooor_scan::novelty::{generate_queries, to_json};
    let protocol = flag(args, "--protocol").unwrap_or("");
    let mechanism = flag(args, "--mechanism").unwrap_or("");
    let sink = flag(args, "--sink").unwrap_or("");
    if protocol.is_empty() || mechanism.is_empty() || sink.is_empty() {
        eprintln!("novelty requires --protocol, --mechanism and --sink [--fork-family F] [--ledger P]");
        exit(2);
    }
    let fork_family = flag(args, "--fork-family");
    // Local self-dedup, reusing the fingerprint + ledger machinery.
    let fp = fingerprint(mechanism, sink, protocol);
    let default_ledger = format!(
        "{}/.claude/auditooor/ledger.tsv",
        std::env::var("HOME").unwrap_or_else(|_| ".".into())
    );
    let ledger = flag(args, "--ledger").unwrap_or(&default_ledger);
    let self_dedup = match ledger_check(Path::new(ledger), &fp) {
        LedgerStatus::Novel => "NOVEL_LOCALLY",
        LedgerStatus::Duplicate => "SELF_DUPLICATE",
    };
    // Baked-in known-bug-class corpus: catch textbook classes (ALM MEV, FoT,
    // first-depositor, fee-rounding, AC-03...) BEFORE the manual world-search,
    // and carry a payability tag so intended/dust classes auto-demote.
    use auditooor_scan::corpus::{classify, Payability};
    let corpus_match = classify(protocol, mechanism, sink);
    let corpus_tuple = corpus_match.as_ref().map(|m| {
        let verdict = match m.class.payability {
            // Intended/low-tier known classes are a hard kill for a PoC spend.
            Payability::Intended => "KNOWN-CLASS / KILL (intended-or-oos)",
            Payability::LowTier => "KNOWN-CLASS / DEMOTE (low-tier)",
            // Payable-if-novel: known class, but a fresh unclaimed instance can pay.
            Payability::Payable => "KNOWN-CLASS / GATE (payable only if novel instance)",
        };
        (verdict, m.class.name, m.class.payability.as_str(), m.class.prior_art)
    });
    let queries = generate_queries(protocol, mechanism, sink, fork_family);
    println!("{}", to_json(protocol, mechanism, sink, &fp, self_dedup, corpus_tuple, &queries, json_escape));
}

/// Today's date as `YYYY-MM-DD`, UTC, pure std. The ledger is the only
/// instrument that measures lead-to-payout over time; a column that says "now"
/// on every row measures nothing.
fn utc_date() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let days = secs.div_euclid(86_400);
    // Howard Hinnant's civil_from_days.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02}")
}

fn cmd_ev(args: &[String]) {
    use auditooor_scan::ev::{evaluate, EvInputs};
    let num = |name: &str, default: f64| -> f64 {
        flag(args, name).and_then(|s| s.parse::<f64>().ok()).unwrap_or(default)
    };
    if flag(args, "--cap").is_none() {
        eprintln!("ev requires --cap <max_bounty_usd> [--audits N] [--age-years F] [--crowded] [--fresh-code] [--seam-value] [--fleet-cost USD] [--lite-cost USD] [--nsloc N]");
        eprintln!("  prints verdict (ABORT/SCOPE-ONLY/PROCEED) and mode (ABORT/LITE/DEEP) — mode is what you spend");
        exit(2);
    }
    let inp = EvInputs {
        cap_usd: num("--cap", 0.0),
        audits: num("--audits", 0.0) as u32,
        age_years: num("--age-years", 0.0),
        crowded: has_flag(args, "--crowded"),
        fresh_code: has_flag(args, "--fresh-code"),
        seam_value: has_flag(args, "--seam-value"),
        fleet_cost_usd: num("--fleet-cost", 200.0),
        lite_cost_usd: flag(args, "--lite-cost").and_then(|s| s.parse::<f64>().ok()),
        target_nsloc: flag(args, "--nsloc").and_then(|s| s.parse::<u32>().ok()),
    };
    let res = evaluate(&inp);
    let mut out = String::from("{\n");
    out.push_str(&format!("  \"verdict\": \"{}\",\n", res.verdict.as_str()));
    out.push_str(&format!("  \"mode\": \"{}\",\n", res.mode.as_str()));
    out.push_str(&format!("  \"ev_usd\": {:.0},\n", res.ev_usd));
    out.push_str(&format!("  \"ev_lite_usd\": {:.0},\n", res.ev_lite_usd));
    out.push_str(&format!("  \"p_find\": {:.4},\n", res.p_find));
    out.push_str(&format!("  \"p_find_lite\": {:.4},\n", res.p_find_lite));
    out.push_str(&format!("  \"fortress_score\": {:.1},\n", res.fortress_score));
    out.push_str("  \"rationale\": [\n");
    for (i, line) in res.rationale.iter().enumerate() {
        if i > 0 { out.push_str(",\n"); }
        out.push_str(&format!("    \"{}\"", json_escape(line)));
    }
    out.push_str("\n  ]\n}");
    println!("{}", out);
}

/// Abort B, as an exit code.
///
/// The abort used to be a paragraph in SKILL.md that the orchestrator had to
/// remember to apply to the right field of a JSON blob. On the Fluid hunt
/// (2026-09-19) it did not fire and three hunters were spent proving an impact
/// map was fiction. A gate you can forget to read is not a gate — this one exits
/// 3 on ABORT, so a `&&`-chained pipeline stops whether or not anyone is paying
/// attention.
fn cmd_gate(args: &[String]) {
    let dir = args.iter().find(|a| !a.starts_with("--")).map(|s| s.as_str()).unwrap_or(".");
    let json = auditooor_scan::xray::generate(Path::new(dir), 5);
    let field = |k: &str| -> String {
        let pat = format!("\"{k}\": ");
        json.find(&pat)
            .map(|i| {
                let rest = &json[i + pat.len()..];
                if let Some(body) = rest.strip_prefix('"') {
                    // quoted string: read to the closing quote, honouring escapes
                    let mut out = String::new();
                    let mut chars = body.chars();
                    while let Some(c) = chars.next() {
                        match c {
                            '\\' => out.extend(chars.next()),
                            '"' => break,
                            _ => out.push(c),
                        }
                    }
                    out
                } else {
                    rest.chars()
                        .take_while(|c| *c != ',' && *c != '\n' && *c != '}')
                        .collect::<String>()
                        .trim()
                        .to_string()
                }
            })
            .unwrap_or_default()
    };
    let verdict = field("verdict");
    let why = field("why");
    let perm = field("permissionless_entries");
    let seams = field("seam_contracts");

    println!("gate: {verdict}");
    println!("  permissionless entries : {perm}");
    println!("  seam contracts         : {seams}");
    println!("  why                    : {why}");

    match verdict.as_str() {
        "ABORT" | "FORTRESS" => {
            eprintln!("ABORT — do not spawn hunters on this target.");
            exit(3);
        }
        _ => {
            println!("  → proceed to Phase 2 with the posture above.");
        }
    }
}


/// Protocol known-issues register (K.I.T-style). The LLM extracts findings from
/// the program's audit reports; this stores and matches them so a duplicate dies
/// before a PoC is forged — the pillar that got the Cork run killed post-PoC.
fn cmd_known(args: &[String]) {
    use auditooor_scan::known::{add, brief, check, load, KnownFinding};
    let ledger = flag(args, "--ledger")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
            std::path::Path::new(&home).join(".claude").join("auditooor")
        });
    let sub = args.first().map(|s| s.as_str()).unwrap_or("");
    let protocol = flag(args, "--protocol").unwrap_or("").to_string();
    match sub {
        "add" => {
            if protocol.is_empty() || flag(args, "--title").is_none() {
                eprintln!("known add --protocol P --title T [--root-cause .. --surface .. --mechanism .. --sink .. --impact .. --source .. --id ..]");
                exit(2);
            }
            let f = KnownFinding {
                id: flag(args, "--id").map(str::to_string).unwrap_or_else(|| format!("{}-{}", protocol, load(&ledger, &protocol).len() + 1)),
                protocol: protocol.clone(),
                title: flag(args, "--title").unwrap_or("").into(),
                root_cause: flag(args, "--root-cause").unwrap_or("").into(),
                surface: flag(args, "--surface").unwrap_or("").into(),
                mechanism: flag(args, "--mechanism").unwrap_or("").into(),
                sink: flag(args, "--sink").unwrap_or("").into(),
                impact: flag(args, "--impact").unwrap_or("").into(),
                source: flag(args, "--source").unwrap_or("").into(),
            };
            match add(&ledger, &f) {
                Ok(()) => println!("registered [{}] {} -> {}", f.id, f.title, auditooor_scan::known::register_path(&ledger, &protocol).display()),
                Err(e) => { eprintln!("cannot write register: {e}"); exit(1); }
            }
        }
        "check" => {
            if protocol.is_empty() {
                eprintln!("known check --protocol P --mechanism M --sink S --root-cause \"..\" [--surface ..]");
                exit(2);
            }
            let reg = load(&ledger, &protocol);
            let r = check(
                &reg,
                flag(args, "--mechanism").unwrap_or(""),
                flag(args, "--sink").unwrap_or(""),
                flag(args, "--root-cause").unwrap_or(""),
                flag(args, "--surface").unwrap_or(""),
            );
            println!("{{\n  \"verdict\": \"{}\",\n  \"score\": {:.2},\n  \"register_size\": {},\n  \"why\": \"{}\"\n}}",
                r.verdict.as_str(), r.score, reg.len(), auditooor_scan::json_escape(&r.why));
            // KNOWN exits non-zero so a pipeline can kill the candidate.
            if r.verdict == auditooor_scan::known::Verdict::Known { exit(3); }
        }
        "brief" => {
            if protocol.is_empty() { eprintln!("known brief --protocol P [--out FILE]"); exit(2); }
            let reg = load(&ledger, &protocol);
            let b = brief(&reg, &protocol);
            if let Some(out) = flag(args, "--out") {
                if let Err(e) = std::fs::write(out, &b) { eprintln!("cannot write {out}: {e}"); exit(1); }
                eprintln!("wrote brief -> {out} ({} known findings)", reg.len());
            }
            print!("{b}");
        }
        _ => {
            eprintln!("usage: auditooor-scan known <add|check|brief> --protocol P ...");
            eprintln!("  add   — register a prior finding (LLM extracts from audit reports)");
            eprintln!("  check — match a candidate against the register; exits 3 on KNOWN");
            eprintln!("  brief — emit the DO-NOT-CHASE brief for `pack --brief`");
            exit(2);
        }
    }
}


/// Static confirmation of a hunter hypothesis (GPTScan-style): the cheap
/// deterministic filter between a LEAD and an expensive fork PoC. REFUTED exits 3
/// so a pipeline drops the lead.
fn cmd_confirm(args: &[String]) {
    use auditooor_scan::confirm::{function_body, run};
    let file = match args.iter().find(|a| !a.starts_with("--")) {
        Some(f) => f.as_str(),
        None => { eprintln!("confirm <file.sol> --function F --check cei|unchecked-return|guard"); exit(2); }
    };
    let func = match flag(args, "--function") { Some(f) => f, None => { eprintln!("--function required"); exit(2); } };
    let check = flag(args, "--check").unwrap_or("cei");
    let src = match std::fs::read_to_string(file) { Ok(s) => s, Err(e) => { eprintln!("cannot read {file}: {e}"); exit(1); } };
    let (body, line0) = match function_body(&src, func) {
        Some(x) => x,
        None => { eprintln!("function '{func}' not found (or is a declaration) in {file}"); exit(2); }
    };
    let c = run(check, &body, line0);
    let ev: Vec<String> = c.evidence.iter().map(|l| format!("{}:{}", file, l)).collect();
    println!("{{\n  \"check\": \"{}\",\n  \"function\": \"{}\",\n  \"verdict\": \"{}\",\n  \"why\": \"{}\",\n  \"evidence\": [{}]\n}}",
        check, func, c.verdict.as_str(), auditooor_scan::json_escape(&c.why),
        ev.iter().map(|e| format!("\"{}\"", auditooor_scan::json_escape(e))).collect::<Vec<_>>().join(", "));
    exit(c.verdict.exit_code());
}

fn cmd_detectors(args: &[String]) {
    use auditooor_scan::detectors::scan_all;
    let target = args.iter().find(|a| !a.starts_with("--")).map(|s| s.as_str()).unwrap_or(".");
    let p = Path::new(target);
    // Collect (relative_path, source) pairs for a file or a whole tree.
    let files: Vec<(String, String)> = if p.is_file() {
        std::fs::read_to_string(p).ok().map(|s| vec![(target.to_string(), s)]).unwrap_or_default()
    } else {
        auditooor_scan::walk_files(p)
            .into_iter()
            .filter(|f| f.extension().and_then(|e| e.to_str()) == Some("sol"))
            .filter_map(|f| {
                let rel = f.strip_prefix(p).unwrap_or(&f).to_string_lossy().to_string();
                std::fs::read_to_string(&f).ok().map(|s| (rel, s))
            })
            .collect()
    };

    let mut out = String::from("[\n");
    let mut first = true;
    for (path, src) in &files {
        for d in scan_all(src) {
            if !first {
                out.push_str(",\n");
            }
            first = false;
            out.push_str(&format!(
                "  {{\"file\": \"{}\", \"line\": {}, \"id\": \"{}\", \"severity\": \"{}\", \"title\": \"{}\", \"detail\": \"{}\"}}",
                json_escape(path),
                d.line,
                json_escape(&d.id),
                json_escape(&d.severity),
                json_escape(&d.title),
                json_escape(&d.detail),
            ));
        }
    }
    out.push_str("\n]");
    println!("{out}");
}

fn cmd_harness(args: &[String]) {
    use auditooor_scan::harness::{
        generate_fork_poc, generate_harness, generate_system_harness, parse_functions, SystemContract,
    };

    // System mode: build ONE invariant suite spanning every contract in a dir.
    if has_flag(args, "--system") {
        let dir = args.iter().find(|a| !a.starts_with("--")).map(|s| s.as_str()).unwrap_or(".");
        let mut contracts = Vec::new();
        for f in auditooor_scan::walk_files(Path::new(dir)) {
            if f.extension().and_then(|e| e.to_str()) != Some("sol") {
                continue;
            }
            let src = match std::fs::read_to_string(&f) {
                Ok(s) => s,
                Err(_) => continue,
            };
            let name = src
                .lines()
                .find_map(|l| l.trim().strip_prefix("contract ").map(|r| {
                    r.split(|c: char| c.is_whitespace() || c == '{').next().unwrap_or("C").to_string()
                }));
            if let Some(name) = name {
                let import = format!("src/{}", f.file_name().unwrap().to_string_lossy());
                let fns = parse_functions(&src);
                if !fns.is_empty() {
                    contracts.push(SystemContract { name, import, fns });
                }
            }
        }
        if contracts.is_empty() {
            eprintln!("no in-scope contracts with entrypoints found under {dir}");
            exit(1);
        }
        let out = generate_system_harness(&contracts);
        match flag(args, "--out") {
            Some(path) => {
                if let Some(parent) = Path::new(path).parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                if let Err(e) = std::fs::write(path, &out) {
                    eprintln!("cannot write {path}: {e}");
                    exit(1);
                }
                eprintln!("wrote SYSTEM harness spanning {} contracts -> {path}", contracts.len());
            }
            None => print!("{out}"),
        }
        return;
    }

    // Positional: the .sol file to build a harness for.
    let file = match args.iter().find(|a| !a.starts_with("--")) {
        Some(f) => f.as_str(),
        None => {
            eprintln!("usage: auditooor-scan harness <file.sol> [--contract NAME] [--import PATH] [--out FILE] | --system <dir>");
            exit(2);
        }
    };
    let src = match std::fs::read_to_string(file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("cannot read {file}: {e}");
            exit(1);
        }
    };
    // Contract name: explicit flag, else first `contract X` in the file.
    let contract = flag(args, "--contract").map(|s| s.to_string()).unwrap_or_else(|| {
        src.lines()
            .find_map(|l| l.trim().strip_prefix("contract ").map(|r| {
                r.split(|c: char| c.is_whitespace() || c == '{').next().unwrap_or("Target").to_string()
            }))
            .unwrap_or_else(|| "Target".to_string())
    });
    let import = flag(args, "--import").map(|s| s.to_string()).unwrap_or_else(|| format!("src/{}.sol", contract));

    // Prefer the real AST when built with `--features solar`; fall back to the
    // regex parser if AST parsing is unavailable or yields nothing.
    #[cfg(feature = "solar")]
    let fns = {
        match auditooor_scan::ast::parse_functions_ast(&src) {
            Ok(f) if !f.is_empty() => {
                eprintln!("[solar] AST parse: {} entrypoints", f.len());
                f
            }
            _ => {
                eprintln!("[solar] AST parse empty/failed — falling back to regex parser");
                parse_functions(&src)
            }
        }
    };
    #[cfg(not(feature = "solar"))]
    let fns = parse_functions(&src);

    // Fork-PoC mode: Immunefi-compliant directed exploit against live state.
    if has_flag(args, "--fork") {
        let address = flag(args, "--address").unwrap_or("0x0000000000000000000000000000000000000000");
        let rpc_env = flag(args, "--rpc-env").unwrap_or("RPC_URL");
        let block = flag(args, "--block").and_then(|v| v.parse::<u64>().ok());
        let poc = generate_fork_poc(&contract, address, rpc_env, block, &fns);
        match flag(args, "--out") {
            Some(path) => {
                if let Some(parent) = Path::new(path).parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                if let Err(e) = std::fs::write(path, &poc) {
                    eprintln!("cannot write {path}: {e}");
                    exit(1);
                }
                eprintln!("wrote fork PoC for {contract} @ {address} -> {path}");
            }
            None => print!("{poc}"),
        }
        return;
    }

    let out = generate_harness(&contract, &import, &fns);
    match flag(args, "--out") {
        Some(path) => {
            if let Some(parent) = Path::new(path).parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            if let Err(e) = std::fs::write(path, &out) {
                eprintln!("cannot write {path}: {e}");
                exit(1);
            }
            eprintln!("wrote harness for {contract} ({} entrypoints) -> {path}", fns.len());
        }
        None => print!("{out}"),
    }
}

fn cmd_fingerprint(args: &[String]) {
    let mechanism = flag(args, "--mechanism").unwrap_or("");
    let sink = flag(args, "--sink").unwrap_or("");
    let entrypoint = flag(args, "--entrypoint").unwrap_or("");
    if mechanism.is_empty() || sink.is_empty() || entrypoint.is_empty() {
        eprintln!("fingerprint requires --mechanism, --sink and --entrypoint");
        exit(2);
    }
    let fp = fingerprint(mechanism, sink, entrypoint);
    let default_ledger = format!(
        "{}/.claude/auditooor/ledger.tsv",
        std::env::var("HOME").unwrap_or_else(|_| ".".into())
    );
    let ledger = flag(args, "--ledger").unwrap_or(&default_ledger);
    let ledger_path = Path::new(ledger);
    let status = ledger_check(ledger_path, &fp);
    let status_str = match status {
        LedgerStatus::Novel => "NOVEL",
        LedgerStatus::Duplicate => "DUPLICATE",
    };
    if has_flag(args, "--add") && status == LedgerStatus::Novel {
        let label = flag(args, "--label").unwrap_or("");
        let _ = ledger_add(ledger_path, &fp, label);
    }
    println!(
        "{{\"fingerprint\": \"{}\", \"status\": \"{}\", \"ledger\": \"{}\"}}",
        fp,
        status_str,
        json_escape(ledger)
    );
}
