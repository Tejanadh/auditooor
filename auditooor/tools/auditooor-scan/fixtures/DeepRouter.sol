pragma solidity ^0.8.20;
// One value sink, buried deep in a branchy path with external hops.
// Few money keywords, but this is exactly where live bugs hide (zhero).
contract DeepRouter {
    function route(address target, uint256 kind, uint256 amt) external {
        if (kind == 1) {
            if (amt > 0) {
                for (uint i = 0; i < 3; i++) {
                    if (i == 2 && amt > 100) {
                        (bool ok,) = target.call(abi.encodeWithSignature("pull(uint256)", amt));
                        require(ok && amt != 0);
                        while (amt > 1000) {
                            token.transfer(target, amt); // buried value sink
                            amt = amt / 2;
                        }
                    }
                }
            }
        }
    }
}
