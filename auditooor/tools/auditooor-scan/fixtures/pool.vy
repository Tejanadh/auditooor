# pragma version 0.2.15
@external
@payable
def add_liquidity(amounts: uint256[2]):
    self.balances[0] += amounts[0]
@external
def remove_liquidity(amount: uint256):
    send(msg.sender, amount)
