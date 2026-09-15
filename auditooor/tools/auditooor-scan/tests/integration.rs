//! Integration tests over the `fixtures/` repo.

use auditooor_scan::*;
use std::collections::BTreeSet;
use std::path::Path;

fn fixtures() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn walk_excludes_tests_and_finds_all_ecosystems() {
    let root = fixtures().join("fixtures");
    let scans = scan_repo(&root);
    let ecos: BTreeSet<String> = scans.iter().map(|s| s.ecosystem.clone()).collect();
    // Every ecosystem fixture must be detected.
    for e in ["evm", "vyper", "solana", "move", "rust"] {
        assert!(ecos.contains(e), "missing ecosystem {e}; got {ecos:?}");
    }
    // Non-source files (notes.txt) must be ignored.
    assert!(!scans.iter().any(|s| s.path.ends_with("notes.txt")));
}

#[test]
fn open_vault_outranks_guarded_vault() {
    let root = fixtures().join("fixtures");
    let scans = scan_repo(&root);
    let open = scans.iter().find(|s| s.path.ends_with("OpenVault.sol")).unwrap();
    let guarded = scans.iter().find(|s| s.path.ends_with("GuardedVault.sol")).unwrap();
    assert!(
        open.score > guarded.score,
        "permissionless OpenVault ({}) must outrank guarded ({})",
        open.score,
        guarded.score
    );
}

#[test]
fn highest_score_is_a_permissionless_money_mover() {
    let root = fixtures().join("fixtures");
    let scans = scan_repo(&root);
    let top = &scans[0];
    // The top-ranked file must carry at least one permissionless value site.
    assert!(
        top.value_sites.iter().any(|v| v.permissionless_hint),
        "top file {} should have a permissionless value site",
        top.path
    );
    // The pure helper (no value flow) must rank at/near the bottom with score 0.
    let helper = scans.iter().find(|s| s.path.ends_with("helper.rs")).unwrap();
    assert_eq!(helper.score, 0);
}

#[test]
fn buried_value_sink_gets_high_path_complexity() {
    // The transfer in DeepRouter is nested ~7 braces deep behind if/for/while/require
    // and an external .call — its path_complexity must dominate a flat vault transfer.
    let root = fixtures().join("fixtures");
    let scans = scan_repo(&root);
    let deep = scans.iter().find(|s| s.path.ends_with("DeepRouter.sol")).unwrap();
    let guarded = scans.iter().find(|s| s.path.ends_with("GuardedVault.sol")).unwrap();

    let deep_pc = deep.value_sites.iter().map(|v| v.path_complexity).max().unwrap_or(0);
    let flat_pc = guarded.value_sites.iter().map(|v| v.path_complexity).max().unwrap_or(0);
    assert!(
        deep_pc > flat_pc,
        "buried sink pc {deep_pc} must exceed flat sink pc {flat_pc}"
    );

    // And DeepRouter, despite few keywords, must out-risk the flat GuardedVault.
    assert!(
        deep.risk_score > guarded.risk_score,
        "deep-path DeepRouter risk {} should beat flat GuardedVault risk {}",
        deep.risk_score,
        guarded.risk_score
    );
    // Its complexity-weighted risk must exceed its own bare money score — proof the
    // complexity signal actually lifts it.
    assert!(deep.risk_score > deep.score);
}

#[test]
fn flat_access_control_bug_is_not_buried() {
    // InitTakeover has no money keyword and no complexity — money-proximity would
    // score it ~0. Detector leads (AC-01 + AC-03) must lift it above a pure helper.
    let root = fixtures().join("fixtures");
    let scans = scan_repo(&root);
    let init = scans.iter().find(|s| s.path.ends_with("InitTakeover.sol")).unwrap();
    let helper = scans.iter().find(|s| s.path.ends_with("helper.rs")).unwrap();
    assert!(init.detector_hits >= 2, "expected AC leads, got {}", init.detector_hits);
    assert!(
        init.risk_score > helper.risk_score,
        "flat AC bug (risk {}) must outrank a pure helper (risk {})",
        init.risk_score,
        helper.risk_score
    );
    // Its money-proximity score alone is tiny; the detector bonus is what saves it.
    assert!(init.risk_score > init.score, "detector bonus must lift risk above money score");
}

#[test]
fn maturity_recommends_a_tactic() {
    let root = fixtures().join("fixtures");
    let scans = scan_repo(&root);
    let (total, tactic) = repo_maturity(&scans);
    assert!(total > 0);
    assert!(!tactic.is_empty());
}

#[test]
fn xray_pack_names_permissionless_open_vault() {
    let root = fixtures().join("fixtures");
    let json = auditooor_scan::xray::generate(&root, 8);
    assert!(json.contains("\"verdict\":"));
    assert!(json.contains("deposit"));
    assert!(json.contains("permissionless"));
    assert!(json.contains("\"git\""));
    assert!(!json.contains("\"verdict\": \"FORTRESS\""));
}

#[test]
fn fingerprint_ledger_roundtrip() {
    let dir = std::env::temp_dir().join(format!("auditooor-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let ledger = dir.join("ledger.tsv");
    let _ = std::fs::remove_file(&ledger);

    let fp = fingerprint("share inflation", "vault.totalSupply", "deposit()");
    assert_eq!(ledger_check(&ledger, &fp), LedgerStatus::Novel);
    ledger_add(&ledger, &fp, "first-depositor inflation").unwrap();
    assert_eq!(ledger_check(&ledger, &fp), LedgerStatus::Duplicate);

    // A different candidate is still novel.
    let fp2 = fingerprint("oracle staleness", "pool.price", "getPrice()");
    assert_eq!(ledger_check(&ledger, &fp2), LedgerStatus::Novel);

    std::fs::remove_dir_all(&dir).ok();
}
