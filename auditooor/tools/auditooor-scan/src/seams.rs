//! Periphery / seam detection.
//!
//! The audited *core* of a mature protocol is a fortress — everything machine-
//! findable is gone. Value that is still reachable sits in the **seams**: routers,
//! wrappers, adapters, zaps, migration helpers, cross-integration glue — contracts
//! added around the core, often with unclear audit coverage. This ranks contracts
//! by how periphery-like they are, so the hunt points at the un-swept surface
//! instead of the fortress. It is a targeting signal, not a verdict.

use crate::detectors::blank_comments;

/// Peripherality analysis for one contract.
#[derive(Debug, Clone, PartialEq)]
pub struct SeamScore {
    pub contract: String,
    /// >0 leans periphery/seam, <0 leans audited-core.
    pub periphery_score: i32,
    pub classification: String,
    pub local_imports: u32,
    pub external_calls: u32,
    pub state_slots: u32,
    pub reasons: Vec<String>,
}

/// Contract-name tokens that signal glue/periphery.
const PERIPHERY_NAMES: &[&str] = &[
    "router", "wrapper", "adapter", "zap", "migrat", "gateway", "helper",
    "periphery", "forwarder", "relayer", "messaging", "integration", "bridge",
    "aggregator", "batch", "multicall", "handler", "manager",
];

fn count(hay: &str, needle: &str) -> u32 {
    hay.matches(needle).count() as u32
}

/// Analyze one contract's source into a peripherality score.
pub fn analyze_contract(name: &str, src: &str) -> SeamScore {
    let blanked = blank_comments(src);
    let lower = blanked.to_lowercase();
    let name_lc = name.to_lowercase();
    let mut reasons = Vec::new();

    // Name signal — the strongest single indicator of glue.
    let name_hits: u32 = PERIPHERY_NAMES.iter().filter(|k| name_lc.contains(*k)).count() as u32;
    if name_hits > 0 {
        reasons.push(format!("periphery name token(s): {name}"));
    }

    // Fan-out: local imports of other project contracts (a router wires many).
    // Counted on RAW source — import paths live in string literals that the
    // comment/string blanker wipes.
    let raw_lower = src.to_lowercase();
    let local_imports: u32 = raw_lower
        .lines()
        .filter(|l| l.contains("import") && l.contains(".sol") && (l.contains("./") || l.contains("../")))
        .count() as u32;
    if local_imports > 0 {
        reasons.push(format!("{local_imports} local import(s) of other contracts (wiring)"));
    }

    // External-call density: low-level calls + delegatecalls = forwarding/glue.
    let external_calls = count(&lower, ".call(")
        + count(&lower, ".call{")
        + count(&lower, ".delegatecall(")
        + count(&lower, ".staticcall(");
    if external_calls > 0 {
        reasons.push(format!("{external_calls} low-level/forwarding call(s)"));
    }

    // State weight: accounting lives in the core. Mappings + explicit storage.
    let mappings = count(&lower, "mapping(");
    let balances = count(&lower, "balances") + count(&lower, "totalsupply") + count(&lower, "shares[");
    let state_slots = mappings + balances;
    if state_slots > 0 {
        reasons.push(format!("{state_slots} accounting/state signal(s) (core-like)"));
    }

    // Peripherality: name and forwarding push toward SEAM; accounting pulls to CORE.
    let periphery_score = (name_hits as i32) * 5
        + (local_imports as i32) * 2
        + (external_calls as i32) * 2
        - (state_slots as i32) * 3;

    let classification = if periphery_score >= 4 {
        "SEAM".to_string()
    } else if periphery_score <= -3 {
        "CORE".to_string()
    } else {
        "MIXED".to_string()
    };

    SeamScore {
        contract: name.to_string(),
        periphery_score,
        classification,
        local_imports,
        external_calls,
        state_slots,
        reasons,
    }
}

/// Analyze a set of (contract_name, source) pairs, ranked most-periphery first.
pub fn analyze_all(contracts: &[(String, String)]) -> Vec<SeamScore> {
    let mut out: Vec<SeamScore> = contracts.iter().map(|(n, s)| analyze_contract(n, s)).collect();
    out.sort_by(|a, b| b.periphery_score.cmp(&a.periphery_score).then(a.contract.cmp(&b.contract)));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn router_ranks_as_seam_above_core_vault() {
        let router = r#"
            import {IVault} from "./Vault.sol";
            contract SwapRouter {
                function swap(address v, uint256 a) external {
                    (bool ok,) = v.call(abi.encodeWithSignature("pull(uint256)", a));
                    require(ok);
                }
            }
        "#;
        let vault = r#"
            contract Vault {
                mapping(address => uint256) public shares;
                mapping(address => uint256) public balances;
                uint256 public totalSupply;
                function deposit() external payable { shares[msg.sender] += msg.value; }
            }
        "#;
        let scores = analyze_all(&[("SwapRouter".into(), router.into()), ("Vault".into(), vault.into())]);
        assert_eq!(scores[0].contract, "SwapRouter");
        assert_eq!(scores[0].classification, "SEAM");
        let vault_score = scores.iter().find(|s| s.contract == "Vault").unwrap();
        assert_eq!(vault_score.classification, "CORE");
        assert!(scores[0].periphery_score > vault_score.periphery_score);
    }

    #[test]
    fn comments_do_not_inflate_peripherality() {
        // A core vault whose comment mentions "router" must not be miscalled a seam.
        let vault = "contract Vault { // not a router, just a note\n mapping(address=>uint256) public bal; }";
        let s = analyze_contract("Vault", vault);
        assert_eq!(s.classification, "CORE");
    }
}
