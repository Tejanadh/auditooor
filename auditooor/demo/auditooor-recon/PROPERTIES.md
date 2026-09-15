# PROPERTIES (mechanical seeds)

- [ ] `P-01` (pair, EXPLORATORY) round-trip: withdraw after deposit returns ≤ in, absent documented fees
- [ ] `P-02` (aggregate, EXPLORATORY) shares == Σ parts across every write site (flag any function that writes one side only)
- [ ] `P-03` (delta-gap, EXPLORATORY) deposit writes only [shares] / [] — check the sibling path credits the other side
- [ ] `P-04` (delta-gap, EXPLORATORY) withdraw writes only [] / [shares] — check the sibling path credits the other side
- [ ] `P-05` (conservation, SHOULD-HOLD) absent documented yield, aggregate value out ≤ aggregate value in at the system boundary
- [ ] `P-06` (authority, SHOULD-HOLD) no unprivileged sequence grants a role/allowance the actor was not given
