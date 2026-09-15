pragma solidity ^0.8.20;
contract GuardedVault {
    address owner;
    function sweep(address to, uint256 amt) external {
        require(msg.sender == owner);
        token.transfer(to, amt);
    }
}
