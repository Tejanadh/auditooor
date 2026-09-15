pragma solidity ^0.8.20;
// Flat, simple, NO money keywords, NO complexity — but a critical AC bug.
// money-proximity alone buries this; detector-bonus must surface it.
contract InitTakeover {
    address public owner;
    function initialize(address owner_) external {
        owner = owner_;
    }
}
