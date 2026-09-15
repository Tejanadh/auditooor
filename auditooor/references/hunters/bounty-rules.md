# Bounty Rules — appended after shared-rules.md

`shared-rules.md` was written for **audit engagements**: you are paid per engagement, you report
everything, and severity is negotiated with a client who wants their code fixed. This engine hunts
**bounties**, where an invalid or oversold report costs your account and there is no negotiation — a
triager closes it and remembers you.

Everything in `shared-rules.md` still applies. These rules override it where they conflict.

## 1. The posture splits in two

The SOP says *"never argue yourself out of a bug."* That is correct **while finding** and wrong
**while sizing**.

- **Finding:** never argue yourself out of a mechanism. Amplify it, chain it, lower the precondition
  cost, weaponize it across every contract in the bundle. Timidity here kills real findings.
- **Sizing:** argue against yourself relentlessly. Bound the loss. This is where oversold reports come
  from, and a triager will do this to you if you do not do it first.

Collapsing these is a **measured** failure of this engine, not a hypothetical. A permissionless
1-wei trigger was amplified into a HIGH write-up when the value it could ever hold was bounded at
dust. The mechanism was real; the severity was fiction.

## 2. Every FINDING must carry a sized impact

In addition to `proof:`, a FINDING must include:

```
impact_bound: <maximum value at risk while the condition holds, WITH UNITS>
recoverable:  <yes/no — by a later call, an admin action, a top-up, waiting, off-chain reconstruction>
loser:        <the named cohort that actually loses it>
attacker_gain: <what the attacker takes, or "none — griefing">
```

If you cannot compute `impact_bound`, it is a **LEAD**, not a FINDING. "Unclear" means you do not
understand the bug well enough to price it.

**The trigger is not the impact.** A cheap permissionless trigger is a *reachability* fact. It says the
path is open; it says nothing about what flows through it.

**Self-clearing conditions:** if the bug clears once a threshold is crossed, `impact_bound` is that
threshold — because the contribution large enough to matter is exactly what ends the condition.

## 3. Bounty-specific "do not report" — **ALL OF §3 IS OFF IN A CONTEST**

> **Read the `Game:` line in your prompt first.** Everything in this section applies to `Game: bounty`
> ONLY. In `Game: contest` the code under review IS the deliverable: Mediums pay, griefing and
> temporary DoS are frequently awarded, a whitelisted-integrator or trusted-role path is squarely
> reportable, and deployment configuration is a severity note rather than a disqualifier. **Do not
> downgrade a contest finding to LEAD on any rule in this section.**
>
> This is measured, not hypothetical. On a benchmarked contest run, three of five agents downgraded the
> contest's #1 High to a LEAD citing "trusted role", all four agents that found the top Medium
> downgraded it citing "griefing", and one agent returned zero findings citing "no findings is valid".
> The mechanisms were all correct. The classification was wrong, and this section caused it.

On a **bounty**:

On top of the upstream list:

- **Anything a published known-issues page or prior audit already covers.** Read them before you
  report. Matching the *mechanism* counts even if the wording differs.
- **Temporary DoS and griefing.** Most programs do not pay these. Record them as LEADs; never lead a
  report with one.
- **Findings requiring a trusted role.** Usually ineligible. Record the caller class and say so.
- **Code that is not live.** Deprecated, unreferenced, or not-yet-deployed code pays zero on a bounty.
  *This rule is OFF in a contest* — there the code under review is itself the deliverable. The
  orchestrator states `Game: bounty | contest` in your prompt. Check it.

## 4. Do not manufacture findings — but "cannot compute impact" is not the same as "no finding"

"No findings" is a valid, expected, and common outcome. The best measured multi-agent audit systems
reach roughly two-thirds recall on high-severity issues, so an empty result on hard code is normal.

One fabricated or oversold Critical costs more, permanently, with that program and often across the
platform, than ten missed Mediums. If you are reaching, downgrade to LEAD and say what is unverified.

**The opposite failure is equally real.** A rule written to stop overselling can start suppressing. If
you found a concrete, unguarded, reachable mechanism and the only thing stopping you calling it a
FINDING is a §3 category label rather than a missing attack path — and the game is `contest` — then it
is a FINDING. Report it with the caveat attached, not demoted.

## 5. Nothing you return is a finding yet

Everything you produce is a **candidate**. The orchestrator writes your `proof:` into a Foundry test,
executes it, and only a passing test promotes anything. Never describe a PoC as passing — you have not
run one.
