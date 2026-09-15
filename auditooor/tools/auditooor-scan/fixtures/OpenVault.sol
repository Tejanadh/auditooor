pragma solidity ^0.8.20;
contract OpenVault {
    mapping(address => uint256) public shares;
    function deposit() external payable { shares[msg.sender] += msg.value; }
    function withdraw(uint256 amt) external {
        shares[msg.sender] -= amt;
        (bool ok,) = msg.sender.call{value: amt}("");
        require(ok);
    }
    function drainTo(address to, uint256 amt) external {
        token.transfer(to, amt);
    }
}
