// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

/// Demo target: ETH vault with a permissionless drain.
/// Not a bounty. Used to prove pack → LEAD → executed PoC.
contract OpenVault {
    mapping(address => uint256) public shares;

    function deposit() external payable {
        shares[msg.sender] += msg.value;
    }

    function withdraw(uint256 amt) external {
        shares[msg.sender] -= amt;
        (bool ok,) = msg.sender.call{value: amt}("");
        require(ok);
    }

    /// BUG: any caller empties the vault. No shares burn, no owner check.
    function drainTo(address to) external {
        uint256 bal = address(this).balance;
        (bool ok,) = to.call{value: bal}("");
        require(ok);
    }
}
