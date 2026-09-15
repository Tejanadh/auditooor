//! Foundry invariant-harness generation from money-proximity scan output.
//!
//! Port of the fizz idea (detected functions → handler + property templates),
//! but driven by `surface`'s value-keyword census and emitted as a runnable
//! Foundry invariant suite. The generator is deterministic and AST-free; it
//! parses the common function shapes and falls back to Foundry's own
//! `targetContract` auto-fuzzing for everything it can't type.

/// A parsed external/public, state-mutating function.
#[derive(Debug, Clone, PartialEq)]
pub struct FnSig {
    pub name: String,
    /// Solidity types of the parameters, in order (best-effort).
    pub params: Vec<String>,
    pub payable: bool,
}

/// Money semantics inferred from a contract's function names.
#[derive(Debug, Default, PartialEq)]
pub struct MoneyShape {
    pub has_deposit: bool,
    pub has_withdraw: bool,
    pub has_mint: bool,
    pub has_burn: bool,
}

fn strip_line_comment(s: &str) -> &str {
    match s.find("//") {
        Some(i) => &s[..i],
        None => s,
    }
}

/// Extract external/public, non-view/pure function signatures from Solidity source.
/// Best-effort, regex-free: good on the common one-line-declaration shapes that
/// vaults, tokens and pools use.
pub fn parse_functions(src: &str) -> Vec<FnSig> {
    let mut out = Vec::new();
    // Join the header up to the first `{` or `;` so multi-line headers still parse.
    let cleaned: String = src.lines().map(strip_line_comment).collect::<Vec<_>>().join("\n");
    let bytes = cleaned.as_bytes();
    let mut i = 0;
    while let Some(rel) = cleaned[i..].find("function ") {
        let start = i + rel + "function ".len();
        // Header ends at the first `{` or `;` after the parameter list.
        let mut j = start;
        let mut depth = 0i32;
        let mut header_end = None;
        while j < bytes.len() {
            match bytes[j] as char {
                '(' => depth += 1,
                ')' => depth -= 1,
                '{' | ';' if depth <= 0 => {
                    header_end = Some(j);
                    break;
                }
                _ => {}
            }
            j += 1;
        }
        let end = match header_end {
            Some(e) => e,
            None => break,
        };
        let header = &cleaned[start..end];
        i = end + 1;

        // name = up to first '('
        let paren = match header.find('(') {
            Some(p) => p,
            None => continue,
        };
        let name = header[..paren].trim().to_string();
        if name.is_empty() || !name.chars().all(|c| c.is_alphanumeric() || c == '_') {
            continue;
        }
        let close = match header.find(')') {
            Some(c) => c,
            None => continue,
        };
        let param_str = &header[paren + 1..close];
        let tail = header[close + 1..].to_lowercase();

        // Only external/public, mutating functions are fuzzable entrypoints.
        let is_entry = tail.contains("external") || tail.contains("public");
        let is_readonly = tail.contains("view") || tail.contains("pure");
        if !is_entry || is_readonly {
            continue;
        }
        let payable = tail.contains("payable");

        let params: Vec<String> = if param_str.trim().is_empty() {
            Vec::new()
        } else {
            param_str
                .split(',')
                .filter_map(|p| p.trim().split_whitespace().next().map(|t| t.to_string()))
                .collect()
        };
        out.push(FnSig { name, params, payable });
    }
    out
}

/// Infer money semantics from the function set + the value keywords found.
pub fn infer_shape(fns: &[FnSig]) -> MoneyShape {
    let mut s = MoneyShape::default();
    for f in fns {
        let n = f.name.to_lowercase();
        if n.contains("deposit") || (n.contains("mint") && f.payable) {
            s.has_deposit = true;
        }
        if n.contains("withdraw") || n.contains("redeem") {
            s.has_withdraw = true;
        }
        if n.contains("mint") {
            s.has_mint = true;
        }
        if n.contains("burn") {
            s.has_burn = true;
        }
    }
    s
}

/// Produce a fuzz-input expression for a Solidity parameter type.
fn fuzz_arg(ty: &str, idx: usize) -> String {
    let t = ty.trim();
    if t.starts_with("uint") {
        format!("bound(seed{idx}, 0, 1e24)")
    } else if t.starts_with("int") {
        format!("int256(bound(seed{idx}, 0, 1e24))")
    } else if t == "address" || t == "address payable" {
        "actor".to_string()
    } else if t == "bool" {
        format!("(seed{idx} % 2 == 0)")
    } else if t.starts_with("bytes32") {
        format!("bytes32(seed{idx})")
    } else {
        // Unknown / complex type — zero-ish default; Foundry auto-fuzz still covers it.
        format!("/* {t} */ 0")
    }
}

/// Whether every param type is one we can synthesise a wrapper call for.
fn all_params_simple(params: &[String]) -> bool {
    params.iter().all(|t| {
        let t = t.trim();
        t.starts_with("uint")
            || t.starts_with("int")
            || t == "address"
            || t == "address payable"
            || t == "bool"
            || t.starts_with("bytes32")
    })
}

/// Generate a complete, runnable Foundry invariant suite as a single .t.sol file.
pub fn generate_harness(contract: &str, import_path: &str, fns: &[FnSig]) -> String {
    let shape = infer_shape(fns);
    let mut s = String::new();
    s.push_str("// SPDX-License-Identifier: MIT\n");
    s.push_str("pragma solidity ^0.8.13;\n\n");
    s.push_str("// AUTO-GENERATED by auditooor-scan harness. Handler ghosts track money flow;\n");
    s.push_str("// invariants below encode the solvency/conservation properties the scanner\n");
    s.push_str("// inferred from value-keyword proximity. Tighten the TODOs against real intent.\n\n");
    s.push_str("import {Test} from \"forge-std/Test.sol\";\n");
    s.push_str("import {StdInvariant} from \"forge-std/StdInvariant.sol\";\n");
    s.push_str(&format!("import {{{contract}}} from \"{import_path}\";\n\n"));

    // ---- Handler ----
    // Clamped + 3 actors: this is the fizz-quality bit. A single 0xA11CE actor
    // never sees donation/inflation or cross-user accounting bugs.
    s.push_str("contract Handler is Test {\n");
    s.push_str(&format!("    {contract} public target;\n"));
    s.push_str("    address[3] internal actors;\n");
    s.push_str("    uint256 public ghost_depositSum;\n");
    s.push_str("    uint256 public ghost_withdrawSum;\n");
    s.push_str("    mapping(address => uint256) public ghostInOf;\n");
    s.push_str("    mapping(address => uint256) public ghostOutOf;\n\n");
    s.push_str(&format!("    constructor({contract} _t) {{\n"));
    s.push_str("        target = _t;\n");
    s.push_str("        actors[0] = address(0xA11CE);\n");
    s.push_str("        actors[1] = address(0xB0B);\n");
    s.push_str("        actors[2] = address(0xC0DE);\n");
    s.push_str("    }\n\n");
    s.push_str("    function _actor(uint256 seed) internal returns (address a) {\n");
    s.push_str("        a = actors[seed % 3];\n");
    s.push_str("        vm.startPrank(a, a);\n");
    s.push_str("    }\n\n");

    for f in fns {
        if !all_params_simple(&f.params) {
            s.push_str(&format!(
                "    // skipped h_{}: non-trivial params {:?} — Foundry auto-fuzz still covers it via targetContract.\n",
                f.name, f.params
            ));
            continue;
        }
        let mut seed_params: Vec<String> = vec!["uint256 actorSeed".into()];
        seed_params.extend((0..f.params.len()).map(|k| format!("uint256 seed{k}")));
        if f.payable {
            seed_params.push("uint256 vseed".to_string());
        }
        let args: Vec<String> = f.params.iter().enumerate().map(|(k, t)| {
            if t.trim() == "address" || t.trim() == "address payable" {
                "actors[seed".to_string() + &k.to_string() + " % 3]"
            } else {
                fuzz_arg(t, k)
            }
        }).collect();
        let call_args = args.join(", ");
        let nlc = f.name.to_lowercase();

        // Clamped handler (default).
        s.push_str(&format!("    function h_{}({}) public {{\n", f.name, seed_params.join(", ")));
        s.push_str("        address actor = _actor(actorSeed);\n");
        if f.payable {
            s.push_str("        uint256 v = bound(vseed, 0, 100 ether);\n");
            s.push_str("        vm.deal(actor, actor.balance + v);\n");
            s.push_str(&format!("        try target.{}{{value: v}}({}) {{\n", f.name, call_args));
            if nlc.contains("deposit") || nlc.contains("mint") {
                s.push_str("            ghost_depositSum += v;\n");
                s.push_str("            ghostInOf[actor] += v;\n");
                s.push_str("            ghostInOf[actor] += v;\n");
            }
            s.push_str("        } catch {}\n");
            s.push_str("        vm.stopPrank();\n");
        } else if nlc.contains("withdraw") || nlc.contains("redeem") {
            s.push_str("        uint256 room = ghost_depositSum > ghost_withdrawSum ? ghost_depositSum - ghost_withdrawSum : 0;\n");
            if !f.params.is_empty() && f.params[0].starts_with("uint") {
                s.push_str("        seed0 = bound(seed0, 0, room == 0 ? 0 : room);\n");
            }
            s.push_str("        uint256 __pre = actor.balance;\n");
            s.push_str(&format!("        try target.{}({}) {{\n", f.name, call_args));
            s.push_str("            uint256 got = actor.balance - __pre;\n");
            s.push_str("            ghost_withdrawSum += got;\n");
            s.push_str("            ghostOutOf[actor] += got;\n");
            s.push_str("        } catch {}\n");
            s.push_str("        vm.stopPrank();\n");
        } else {
            s.push_str(&format!("        try target.{}({}) {{}} catch {{}}\n", f.name, call_args));
            s.push_str("        vm.stopPrank();\n");
        }
        s.push_str("    }\n\n");

        // Unclamped twin — donation, overflow, first-depositor live here.
        if nlc.contains("withdraw") || nlc.contains("redeem") || nlc.contains("deposit") {
            s.push_str(&format!("    function h_raw_{}({}) public {{\n", f.name, seed_params.join(", ")));
            s.push_str("        address actor = _actor(actorSeed);\n");
            if f.payable {
                s.push_str("        uint256 v = vseed; // UNCLAMPED\n");
                s.push_str("        vm.deal(actor, actor.balance + v);\n");
                s.push_str(&format!("        try target.{}{{value: v}}({}) {{}} catch {{}}\n", f.name, call_args));
            } else {
                s.push_str(&format!("        try target.{}({}) {{}} catch {{}}\n", f.name, call_args));
            }
            s.push_str("        vm.stopPrank();\n");
            s.push_str("    }\n\n");
        }
        // Near-zero / truncation: values that divide-to-zero on 1e18 scale.
        if (nlc.contains("deposit") || nlc.contains("mint") || nlc.contains("withdraw") || nlc.contains("redeem"))
            && f.params.iter().any(|t| t.starts_with("uint"))
        {
            s.push_str(&format!("    function h_tiny_{}({}) public {{\n", f.name, seed_params.join(", ")));
            s.push_str("        address actor = _actor(actorSeed);\n");
            s.push_str("        seed0 = bound(seed0, 1, 1e6); // sub-unit / dust\n");
            s.push_str(&format!("        try target.{}({}) {{}} catch {{}}\n", f.name, call_args));
            s.push_str("        vm.stopPrank();\n");
            s.push_str("    }\n\n");
        }
    }
    // Donation: share-price inflation / first-depositor live here (fizz handler-patterns).
    s.push_str("    function h_donateETH(uint256 actorSeed, uint256 vseed) public {\n");
    s.push_str("        address actor = _actor(actorSeed);\n");
    s.push_str("        uint256 v = bound(vseed, 1, 50 ether);\n");
    s.push_str("        vm.deal(actor, actor.balance + v);\n");
    s.push_str("        (bool ok,) = address(target).call{value: v}(\"\");\n");
    s.push_str("        ok; // donation may revert; that's fine\n");
    s.push_str("        vm.stopPrank();\n");
    s.push_str("    }\n\n");
    s.push_str("    receive() external payable {}\n");
    s.push_str("}\n\n");

    // ---- Invariant test ----
    s.push_str("contract AuditooorInvariant is StdInvariant, Test {\n");
    s.push_str(&format!("    {contract} public target;\n"));
    s.push_str("    Handler public handler;\n\n");
    s.push_str("    function setUp() public {\n");
    s.push_str(&format!("        target = new {contract}();\n"));
    s.push_str("        handler = new Handler(target);\n");
    s.push_str("        targetContract(address(handler));\n");
    s.push_str("    }\n\n");

    if shape.has_deposit && shape.has_withdraw {
        s.push_str("    /// Solvency: the vault's ETH balance must cover net deposits.\n");
        s.push_str("    /// A withdraw path that pays out more than was put in breaks this.\n");
        s.push_str("    function invariant_solvency() public view {\n");
        s.push_str("        uint256 liabilities = handler.ghost_depositSum() - handler.ghost_withdrawSum();\n");
        s.push_str("        assertGe(address(target).balance, liabilities, \"INSOLVENT: paid out more than deposited\");\n");
        s.push_str("    }\n\n");
        // The LOGIC-bug invariant: in a no-yield vault, aggregate withdrawals can never
        // exceed aggregate deposits. A rounding-direction error (assets rounded UP on the
        // way out) lets a deposit→withdraw round-trip profit — this catches that class
        // (e.g. the Balancer-style rounding bug) even with every access guard intact.
        s.push_str("    /// No-free-money: absent yield, total out <= total in. A rounding-\n");
        s.push_str("    /// direction bug makes a deposit->withdraw round-trip profit and breaks this.\n");
        s.push_str("    /// (If the vault legitimately earns yield, replace with a per-actor\n");
        s.push_str("    /// round-trip check that accounts for the earned share.)\n");
        s.push_str("    function invariant_no_free_money() public view {\n");
        s.push_str("        assertLe(handler.ghost_withdrawSum(), handler.ghost_depositSum(), \"FREE MONEY: withdrew more than ever deposited\");\n");
        s.push_str("    }\n\n");
        s.push_str("    /// Per-actor: absent yield, no actor withdraws more than they deposited.\n");
        s.push_str("    /// Catches donation/inflation and share-price theft the global sum can hide.\n");
        s.push_str("    function invariant_no_actor_profit() public view {\n");
        s.push_str("        address a0 = address(0xA11CE);\n");
        s.push_str("        address a1 = address(0xB0B);\n");
        s.push_str("        address a2 = address(0xC0DE);\n");
        s.push_str("        assertLe(handler.ghostOutOf(a0), handler.ghostInOf(a0), \"actor0 profit\");\n");
        s.push_str("        assertLe(handler.ghostOutOf(a1), handler.ghostInOf(a1), \"actor1 profit\");\n");
        s.push_str("        assertLe(handler.ghostOutOf(a2), handler.ghostInOf(a2), \"actor2 profit\");\n");
        s.push_str("    }\n\n");
    }
    if shape.has_mint && shape.has_burn {
        s.push_str("    /// Supply conservation placeholder: wire to the token's totalSupply().\n");
        s.push_str("    /// TODO: assert totalSupply == sum(balances) or minted - burned.\n");
        s.push_str("    function invariant_supply_conservation() public view {\n");
        s.push_str("        assertTrue(true); // replace with real supply accounting\n");
        s.push_str("    }\n\n");
    }
    if !(shape.has_deposit && shape.has_withdraw) && !(shape.has_mint && shape.has_burn) {
        s.push_str("    /// No deposit/withdraw or mint/burn pair detected. Baseline: the target\n");
        s.push_str("    /// must not be self-destructible into the handler. TODO: add a real property.\n");
        s.push_str("    function invariant_baseline() public view {\n");
        s.push_str("        assertTrue(address(target).code.length > 0, \"target self-destructed\");\n");
        s.push_str("    }\n\n");
    }
    s.push_str("}\n");
    s
}

// ---------------------------------------------------------------------------
// Fork-based PoC generation (Immunefi-compliant)
// ---------------------------------------------------------------------------

/// True if `ty` is an ABI-simple type we can safely put in a generated interface
/// (so the PoC compiles against the live contract). Arrays of simple types pass.
fn is_abi_simple(ty: &str) -> bool {
    let mut t = ty.trim();
    // Strip trailing array suffixes: uint256[], address[3][], etc.
    while let Some(open) = t.rfind('[') {
        if t.ends_with(']') {
            t = t[..open].trim_end();
        } else {
            break;
        }
    }
    t.starts_with("uint")
        || t.starts_with("int")
        || t.starts_with("bytes")
        || t == "address"
        || t == "address payable"
        || t == "bool"
        || t == "string"
}

/// Render a Solidity interface for the functions whose params are all ABI-simple.
fn render_interface(name: &str, fns: &[FnSig]) -> (String, Vec<String>) {
    let mut s = format!("interface I{name} {{\n");
    let mut skipped = Vec::new();
    for f in fns {
        if f.params.iter().all(|p| is_abi_simple(p)) {
            let params = f.params.join(", ");
            let mutb = if f.payable { " payable" } else { "" };
            s.push_str(&format!("    function {}({}) external{};\n", f.name, params, mutb));
        } else {
            skipped.push(format!("{}({})", f.name, f.params.join(", ")));
        }
    }
    s.push_str("}\n");
    (s, skipped)
}

/// Generate an Immunefi-compliant **fork-based directed exploit PoC**: forks the
/// live chain at a block, attaches to the *deployed* contract at `address` (no
/// fresh deploy), drives the attack, and asserts attacker profit against real
/// on-chain state. This is the only PoC shape Immunefi accepts — unit-test PoCs
/// against a fresh deploy are rejected.
pub fn generate_fork_poc(
    contract: &str,
    address: &str,
    rpc_env: &str,
    block: Option<u64>,
    fns: &[FnSig],
) -> String {
    let (iface, skipped) = render_interface(contract, fns);
    let fork_call = match block {
        Some(b) => format!("vm.createSelectFork(vm.envString(\"{rpc_env}\"), {b});"),
        None => format!("vm.createSelectFork(vm.envString(\"{rpc_env}\"));"),
    };

    let mut s = String::new();
    s.push_str("// SPDX-License-Identifier: MIT\n");
    s.push_str("pragma solidity ^0.8.13;\n\n");
    s.push_str("// AUTO-GENERATED by auditooor-scan (fork PoC). Immunefi-compliant shape:\n");
    s.push_str("//   * forks the LIVE chain at a block  * attaches to the DEPLOYED contract\n");
    s.push_str("//   * no fresh deploy  * asserts attacker profit against real state.\n");
    s.push_str(&format!("// Run: forge test --match-contract {contract}Exploit -vvv \\\n"));
    s.push_str(&format!("//        (with {rpc_env} set to an archive RPC for the target chain)\n\n"));
    s.push_str("import {Test, console} from \"forge-std/Test.sol\";\n\n");

    s.push_str(&iface);
    s.push_str("\ninterface IERC20 { function balanceOf(address) external view returns (uint256); }\n\n");

    if !skipped.is_empty() {
        s.push_str("// NOTE: these entrypoints have struct/tuple params — add them to the\n");
        s.push_str("// interface manually (declare the struct) if the exploit needs them:\n");
        for sk in &skipped {
            s.push_str(&format!("//   - {sk}\n"));
        }
        s.push('\n');
    }

    s.push_str(&format!("contract {contract}Exploit is Test {{\n"));
    s.push_str(&format!("    I{contract} internal target = I{contract}({address});\n"));
    s.push_str("    address internal attacker = makeAddr(\"attacker\");\n");
    s.push_str("    // TODO: set to the token whose theft/inflation you are demonstrating\n");
    s.push_str("    // (address(0) means measure the attacker's native ETH balance instead).\n");
    s.push_str("    IERC20 internal profitToken = IERC20(address(0));\n\n");

    s.push_str("    function setUp() public {\n");
    s.push_str(&format!("        {fork_call}\n"));
    s.push_str("        vm.deal(attacker, 10 ether); // seed realistic gas/capital\n");
    s.push_str("    }\n\n");

    s.push_str("    function _profit() internal view returns (uint256) {\n");
    s.push_str("        return address(profitToken) == address(0)\n");
    s.push_str("            ? attacker.balance\n");
    s.push_str("            : profitToken.balanceOf(attacker);\n");
    s.push_str("    }\n\n");

    s.push_str("    function test_exploit() public {\n");
    s.push_str("        uint256 before = _profit();\n");
    s.push_str("        vm.startPrank(attacker);\n\n");
    s.push_str("        // ------------------------------------------------------------------\n");
    s.push_str("        // TODO: the exploit sequence against LIVE state. Call target.<fn>(...)\n");
    s.push_str("        // using the interface above. Example skeleton:\n");
    if let Some(first) = fns.iter().find(|f| f.params.iter().all(|p| is_abi_simple(p))) {
        let example_args: Vec<String> = first
            .params
            .iter()
            .map(|p| if p.starts_with("address") { "attacker".to_string() } else if p == "bool" { "true".to_string() } else { "0".to_string() })
            .collect();
        let val = if first.payable { "{value: 0}" } else { "" };
        s.push_str(&format!("        // target.{}{}({});\n", first.name, val, example_args.join(", ")));
    }
    s.push_str("        // ------------------------------------------------------------------\n\n");
    s.push_str("        vm.stopPrank();\n");
    s.push_str("        uint256 gained = _profit() - before;\n");
    s.push_str("        console.log(\"attacker profit:\", gained);\n");
    s.push_str("        assertGt(gained, 0, \"NO PROFIT: exploit did not extract value from live state\");\n");
    s.push_str("    }\n");
    s.push_str("}\n");
    s
}

// ---------------------------------------------------------------------------
// System-level invariant harness (multi-contract)
// ---------------------------------------------------------------------------

/// One contract in a system harness.
pub struct SystemContract {
    pub name: String,
    pub import: String,
    pub fns: Vec<FnSig>,
}

fn var_name(contract: &str) -> String {
    let mut c = contract.chars();
    match c.next() {
        Some(f) => f.to_lowercase().collect::<String>() + c.as_str(),
        None => "c".to_string(),
    }
}

/// Emit a handler wrapper for one entrypoint, tracking system-boundary value.
/// `deposit`-like payable fns add to ghost_in; `withdraw`-like fns measure the
/// balance delta into ghost_out. Everything else is just fuzzed for state.
fn system_wrapper(cvar: &str, prefix: &str, f: &FnSig) -> String {
    let nlc = f.name.to_lowercase();
    let mut s = String::new();
    // Wrapper param seeds (one uint per simple param; complex params -> skip fn).
    if !all_params_simple(&f.params) {
        return format!("    // skipped {prefix}_{}: complex params {:?} (Foundry auto-fuzz still covers via targetContract)\n", f.name, f.params);
    }
    let mut seeds: Vec<String> = (0..f.params.len()).map(|k| format!("uint256 seed{k}")).collect();
    if f.payable {
        seeds.push("uint256 vseed".to_string());
    }
    let args: Vec<String> = f.params.iter().enumerate().map(|(k, t)| fuzz_arg(t, k)).collect();
    let call_args = args.join(", ");
    s.push_str(&format!("    function {prefix}_{}({}) public {{\n", f.name, seeds.join(", ")));
    let inflow = nlc.contains("deposit") || nlc.contains("stake") && !nlc.contains("unstake") || nlc.contains("mint");
    let outflow = nlc.contains("withdraw") || nlc.contains("redeem") || nlc.contains("unstake") || nlc.contains("claim") || nlc.contains("harvest");
    if f.payable {
        s.push_str("        uint256 v = bound(vseed, 0, 100 ether);\n");
        s.push_str("        vm.deal(address(this), address(this).balance + v);\n");
        s.push_str(&format!("        try {cvar}.{}{{value: v}}({}) {{\n", f.name, call_args));
        if inflow {
            s.push_str("            ghost_in += v;\n");
        }
        s.push_str("        } catch {}\n");
    } else if outflow {
        s.push_str("        uint256 __pre = address(this).balance;\n");
        s.push_str(&format!("        try {cvar}.{}({}) {{\n", f.name, call_args));
        s.push_str("            ghost_out += address(this).balance - __pre;\n");
        s.push_str("        } catch {}\n");
    } else {
        s.push_str(&format!("        try {cvar}.{}({}) {{}} catch {{}}\n", f.name, call_args));
    }
    s.push_str("    }\n\n");
    s
}

/// Generate a system-level invariant harness across several contracts. Deploys
/// every contract, drives actors across ALL of them from one handler, and asserts
/// conservation at the SYSTEM BOUNDARY (total value out <= total value in; the
/// whole protocol stays solvent). This catches logic bugs that live in the
/// interaction between contracts — invisible to any per-file scan.
pub fn generate_system_harness(contracts: &[SystemContract]) -> String {
    let mut s = String::new();
    s.push_str("// SPDX-License-Identifier: MIT\n");
    s.push_str("pragma solidity ^0.8.13;\n\n");
    s.push_str("// AUTO-GENERATED by auditooor-scan (SYSTEM harness). Deploys the whole\n");
    s.push_str("// protocol and asserts value conservation across the SYSTEM boundary, so a\n");
    s.push_str("// bug that only manifests through cross-contract sequences still breaks it.\n");
    s.push_str("// FILL THE WIRING in setUp() (constructor args + connect-the-contracts).\n\n");
    s.push_str("import {Test, console} from \"forge-std/Test.sol\";\n");
    s.push_str("import {StdInvariant} from \"forge-std/StdInvariant.sol\";\n");
    for c in contracts {
        s.push_str(&format!("import {{{}}} from \"{}\";\n", c.name, c.import));
    }
    s.push('\n');

    // ---- Handler ----
    s.push_str("contract Handler is Test {\n");
    for c in contracts {
        s.push_str(&format!("    {} public {};\n", c.name, var_name(&c.name)));
    }
    s.push_str("    address internal actor = address(0xA11CE);\n");
    s.push_str("    uint256 public ghost_in;   // total value into the system\n");
    s.push_str("    uint256 public ghost_out;  // total value out of the system\n\n");
    let ctor_params: Vec<String> = contracts.iter().map(|c| format!("{} _{}", c.name, var_name(&c.name))).collect();
    s.push_str(&format!("    constructor({}) {{\n", ctor_params.join(", ")));
    for c in contracts {
        let v = var_name(&c.name);
        s.push_str(&format!("        {v} = _{v};\n"));
    }
    s.push_str("    }\n\n");
    for c in contracts {
        let v = var_name(&c.name);
        for f in &c.fns {
            s.push_str(&system_wrapper(&v, &format!("h_{v}"), f));
        }
    }
    s.push_str("    receive() external payable {}\n");
    s.push_str("}\n\n");

    // ---- System invariants ----
    s.push_str("contract AuditooorSystemInvariant is StdInvariant, Test {\n");
    for c in contracts {
        s.push_str(&format!("    {} public {};\n", c.name, var_name(&c.name)));
    }
    s.push_str("    Handler public handler;\n\n");
    s.push_str("    function setUp() public {\n");
    for c in contracts {
        let v = var_name(&c.name);
        s.push_str(&format!("        {v} = new {}(); // TODO: constructor args\n", c.name));
    }
    s.push_str("        // ==== WIRING: connect the contracts (fill this in) ====\n");
    s.push_str("        // e.g. staker.setPool(address(pool)); pool.setStaker(address(staker));\n");
    s.push_str("        // Optionally seed pool state to model a live protocol holding others' funds:\n");
    for c in contracts {
        s.push_str(&format!("        // vm.deal(address({}), 100 ether);\n", var_name(&c.name)));
    }
    let ctor_args: Vec<String> = contracts.iter().map(|c| var_name(&c.name)).collect();
    s.push_str(&format!("        handler = new Handler({});\n", ctor_args.join(", ")));
    s.push_str("        targetContract(address(handler));\n");
    s.push_str("    }\n\n");
    s.push_str("    /// No free money across the WHOLE system: absent yield, total out <= total in.\n");
    s.push_str("    /// A cross-contract sequence that extracts more than entered breaks this.\n");
    s.push_str("    function invariant_system_no_free_money() public view {\n");
    s.push_str("        assertLe(handler.ghost_out(), handler.ghost_in(), \"SYSTEM FREE MONEY: out > in across contracts\");\n");
    s.push_str("    }\n\n");
    s.push_str("    /// System solvency: the sum of every contract's balance covers net deposits.\n");
    s.push_str("    function invariant_system_solvency() public view {\n");
    s.push_str("        uint256 bal;\n");
    for c in contracts {
        s.push_str(&format!("        bal += address({}).balance;\n", var_name(&c.name)));
    }
    s.push_str("        uint256 liab = handler.ghost_in() - handler.ghost_out();\n");
    s.push_str("        assertGe(bal, liab, \"SYSTEM INSOLVENT: protocol owes more than it holds\");\n");
    s.push_str("    }\n");
    s.push_str("}\n");
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_harness_spans_contracts_and_boundary() {
        let contracts = vec![
            SystemContract {
                name: "Pool".into(),
                import: "src/Pool.sol".into(),
                fns: parse_functions("contract Pool { function deposit() external payable {} function withdraw(uint256 a) external {} }"),
            },
            SystemContract {
                name: "Staker".into(),
                import: "src/Staker.sol".into(),
                fns: parse_functions("contract Staker { function stake(uint256 a) external {} function unstake(uint256 a) external {} }"),
            },
        ];
        let out = generate_system_harness(&contracts);
        assert!(out.contains("invariant_system_no_free_money"));
        assert!(out.contains("invariant_system_solvency"));
        assert!(out.contains("h_pool_deposit"));
        assert!(out.contains("h_staker_unstake"));
        assert!(out.contains("import {Pool}"));
        assert!(out.contains("import {Staker}"));
        // both contracts summed in solvency
        assert!(out.contains("address(pool).balance"));
        assert!(out.contains("address(staker).balance"));
    }

    #[test]
    fn parses_common_shapes() {
        let src = r#"
            contract V {
                function deposit() external payable {}
                function withdraw(uint256 amt) external {}
                function balanceOf(address a) external view returns (uint256) {}
                function _internal() internal {}
            }
        "#;
        let fns = parse_functions(src);
        let names: Vec<&str> = fns.iter().map(|f| f.name.as_str()).collect();
        assert!(names.contains(&"deposit"));
        assert!(names.contains(&"withdraw"));
        assert!(!names.contains(&"balanceOf"), "view must be excluded");
        assert!(!names.contains(&"_internal"), "internal must be excluded");
        let dep = fns.iter().find(|f| f.name == "deposit").unwrap();
        assert!(dep.payable);
        let wd = fns.iter().find(|f| f.name == "withdraw").unwrap();
        assert_eq!(wd.params, vec!["uint256".to_string()]);
    }

    #[test]
    fn infers_vault_shape() {
        let fns = parse_functions("contract V { function deposit() external payable {} function withdraw(uint256 a) external {} }");
        let s = infer_shape(&fns);
        assert!(s.has_deposit && s.has_withdraw);
    }

    #[test]
    fn fork_poc_attaches_to_live_address_no_deploy() {
        let fns = parse_functions(
            "contract Vault { function deposit() external payable {} function withdraw(uint256 a) external {} }",
        );
        let poc = generate_fork_poc("Vault", "0x1234567890123456789012345678901234567890", "RPC_URL", Some(19_000_000), &fns);
        // Must fork, must attach to the live address, must NOT deploy a fresh one.
        assert!(poc.contains("createSelectFork"));
        assert!(poc.contains("0x1234567890123456789012345678901234567890"));
        assert!(!poc.contains("new Vault("), "fork PoC must not deploy a fresh contract");
        assert!(poc.contains("assertGt(gained, 0"));
        assert!(poc.contains("interface IVault"));
        assert!(poc.contains("function withdraw(uint256) external;"));
        assert!(poc.contains("function deposit() external payable;"));
    }

    #[test]
    fn fork_poc_defers_struct_params_to_manual() {
        let fns = vec![FnSig { name: "settle".into(), params: vec!["Order".into()], payable: false }];
        let poc = generate_fork_poc("Ex", "0xabc", "RPC", None, &fns);
        // Struct param must be listed as manual, not put in the interface.
        assert!(poc.contains("struct/tuple params"));
        assert!(!poc.contains("function settle(Order)"));
    }

    #[test]
    fn generated_harness_has_solvency_invariant() {
        let fns = parse_functions("contract Vault { function deposit() external payable {} function withdraw(uint256 a) external {} }");
        let out = generate_harness("Vault", "src/Vault.sol", &fns);
        assert!(out.contains("invariant_solvency"));
        assert!(out.contains("actors[0]"));
        assert!(out.contains("h_raw_withdraw") || out.contains("h_raw_deposit"));
        assert!(out.contains("h_deposit"));
        assert!(out.contains("h_withdraw"));
        assert!(out.contains("StdInvariant"));
        assert!(out.contains("invariant_no_actor_profit"));
        assert!(out.contains("ghostInOf"));
        assert!(out.contains("h_donateETH"));
        assert!(out.contains("h_tiny_"));
    }
}
