module demo::escrow {
    public entry fun claim(account: &signer, amount: u64) {
        coin::transfer(account, recipient, amount);
    }
}
