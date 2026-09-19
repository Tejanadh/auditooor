// SPDX-License-Identifier: MIT
pragma solidity ^0.8.21;

// Interface members are declarations of the CALLEE — never entry points here.
interface IFluidDexT1Admin {
    function updateFeeAndRevenueCut(uint fee_, uint revenueCut_) external;
    function setDexFee(uint fee_) external;
}

abstract contract Constants {
    address internal constant TEAM_MULTISIG = address(0xdead);
    uint256 internal immutable RATE;
}

abstract contract Events {
    event LogRebalance(uint256 oldRate, uint256 newRate);
}

// Project-specific gate names: not onlyOwner, not onlyRole. Still gates.
contract FluidRateHandler is Constants, Events {
    modifier onlyRebalancer() {
        if (!_isRebalancer(msg.sender)) revert();
        _;
    }
    modifier onlyMultisig() {
        if (msg.sender != TEAM_MULTISIG) revert();
        _;
    }

    function _isRebalancer(address) internal view returns (bool) {
        return false;
    }

    // role-gated, attributed to FluidRateHandler (not Constants/Events)
    function rebalance() external onlyRebalancer {
        emit LogRebalance(0, 1);
    }

    // admin: multisig is owner-ish
    function setRate(uint256 r_) external onlyMultisig {
        emit LogRebalance(RATE, r_);
    }

    // inline inequality guard, no modifier — still gated
    function setPauseContract(address p_) external {
        if (msg.sender != TEAM_MULTISIG) revert();
        emit LogRebalance(uint160(p_), 0);
    }

    // the genuine article: nothing stops anyone
    function donate() external payable {
        emit LogRebalance(0, msg.value);
    }
}

// A genuinely open function whose RETURN VALUES are named like guards.
// Must stay permissionless: a false gate here would abort a live target.
contract OpenWithGuardyReturnNames {
    function swap(uint256 amountIn_)
        external
        payable
        returns (uint256 authority, uint256 onlyOut, address permissionedTo)
    {
        return (amountIn_, 0, msg.sender);
    }
}

// OZ idiom: the sender is aliased into a local before the comparison.
contract AliasGuarded {
    address private _authority;
    function _msgSender() internal view returns (address) { return msg.sender; }
    function authority() public view returns (address) { return _authority; }

    function setAuthority(address newAuthority) public virtual {
        address caller = _msgSender();
        if (caller != authority()) {
            revert();
        }
        _authority = newAuthority;
    }

    // no guard, but reads the sender — must stay permissionless
    function selfRegister() external {
        address who = _msgSender();
        _authority = who == address(0) ? _authority : _authority;
    }
}

// The most common guard shape in Solidity: a no-arg call on the LEFT of the
// comparison. `pendingOwner() != sender` must read as a gate.
contract CallFormGuard {
    address private _pending;
    function pendingOwner() public view returns (address) { return _pending; }
    function acceptOwnership() public virtual {
        address sender = msg.sender;
        if (pendingOwner() != sender) {
            revert();
        }
        _pending = address(0);
    }
}
