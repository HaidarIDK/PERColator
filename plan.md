Below is an idiot-proof, Claude-ready implementation plan for an optimal fuzzing suite for this exact risk engine. It’s structured as a checklist of deliverables, with clear “done when” conditions, file layout, and invariant definitions so Claude can’t drift.

⸻

0) Goal and non-goals

Goal: Build a fuzzing suite that (a) catches real invariants regressions, (b) never “passes by skipping” due to errors, (c) is reproducible, and (d) covers the unique surfaces of this engine: warmup budget + reserved insurance, risk-reduction-only gating, funding settlement, ADL scan/waterfall, panic_settle_all and force_realize_losses.

Non-goals: Performance microbenching; matcher correctness (NoOp only).

⸻

Files
	1.	tests/fuzzing.rs

Feature gate
	•	All fuzz tests compile only under --features fuzz (like you already do).

⸻

2) Define the invariant contract once (shared helper)


2.1 Snapshot type (for “no state change on error”)

Define a Snapshot capturing exactly what matters:
	•	vault
	•	insurance_balance, insurance_fee_revenue
	•	loss_accum
	•	risk_reduction_only, warmup_paused, warmup_pause_slot
	•	warmed_pos_total, warmed_neg_total, warmup_insurance_reserved
	•	current_slot, funding_index_qpb_e6, last_funding_slot
	•	for a small set of accounts touched: entire Account structs
	•	optionally: num_used_accounts, used bitmap, next_account_id, free_head, next_free[idx] for allocation tests

Provide:
	•	Snapshot::take(engine, &[u16])
	•	assert_unchanged(before, after) (exact equality)

2.2 Global invariants (asserted after every step)

Implement assert_global_invariants(engine):
	1.	Conservation
assert!(engine.check_conservation())
	2.	Warmup budget & reservation invariants (these are critical):

	•	W+ <= W- + raw_spendable
where raw_spendable = engine.insurance_spendable_raw()
	•	reserved <= raw_spendable
	•	insurance_balance >= floor + reserved
floor = params.risk_reduction_threshold

	3.	Risk reduction mode semantics

	•	If risk_reduction_only == true then warmup_paused == true
	•	If warmup_paused == true then warmup_pause_slot <= current_slot (should hold by construction)

	4.	Account local sanity (for each used account):

	•	reserved_pnl <= max(0, pnl) (otherwise withdrawable logic can underflow into saturating weirdness)
	•	If kind == User then matcher fields are zero
	•	If kind == LP then kind is LP (obvious) and id is nonzero once allocated
	•	entry_price == 0 iff position_size == 0 is NOT guaranteed by code (panic_settle sets entry_price), so don’t assert that. Only assert entry_price <= 1e12 if you want a sanity bound.
	•	funding_index can be any i128, but should be updated on touch; don’t assert equality globally.

Done when: this helper compiles and is used by every fuzz test.

⸻

3) Write the “No silent skipping” rule and enforce it everywhere

Claude must follow this rule:

For any operation returning Result, tests must assert one of:
	•	On Ok: operation-specific postconditions hold, plus global invariants.
	•	On Err(e): either (a) full state unchanged (snapshot), or (b) only explicitly allowed fields changed (rare; must be documented per op).

For this engine, the intended behavior is almost always no mutation on Err. However execute_trade() currently mutates insurance fee and user pnl before margin checks — that’s a real footgun. The fuzz suite will catch it. Do not weaken the tests to accommodate it.

⸻

4) Build an Action-based state machine fuzzer (the main thing)


4.1 Define Action enum

Actions should cover every behavior surface:
	•	AddUser { fee_payment }
	•	AddLP { fee_payment }
	•	Deposit { idx, amount }
	•	Withdraw { idx, amount }
	•	AdvanceSlot { dt }
	•	AccrueFunding { now_slot_delta, oracle_price, rate_bps }
	•	Touch { idx }
	•	ExecuteTrade { lp_idx, user_idx, oracle_price, size }
	•	ApplyADL { loss }
	•	PanicSettleAll { oracle_price }
	•	ForceRealizeLosses { oracle_price }
	•	TopUpInsurance { amount }

4.2 Strategy generation rules (so runs aren’t mostly errors)

Claude must bias the generator so we actually execute meaningful paths:
	•	Always start by creating at least:
	•	1 LP and 2 users (if possible)
	•	initial deposits for them
	•	insurance balance occasionally near threshold (to trigger force_realize paths)
	•	Use “mostly valid indices”: pick from engine’s known allocated set 80% of the time, random 20% (to test AccountNotFound).

4.3 The driver loop

For each proptest case:
	•	init engine with:
	•	max_accounts small (like 16 or 32) for speed
	•	warmup_period_slots > 0
	•	nonzero floor sometimes (two parameter regimes)
	•	maintain a Vec<u16> of live accounts (allocated indices)

For each action:
	1.	Take snapshot of all indices touched by that action (and maybe also global snapshot).
	2.	Execute action.
	3.	If Err:
	•	assert snapshot unchanged (strict).
	4.	Always assert assert_global_invariants(engine) after the step.
	5.	Additionally, after “large” actions (panic_settle/force_realize/apply_adl/execute_trade), assert extra postconditions (below).

4.4 Operation-specific postconditions (must implement)

AddUser/AddLP
On Ok(idx):
	•	engine.is_used(idx)
	•	num_used_accounts incremented by 1
	•	next_account_id incremented by 1
	•	accounts[idx].account_id unique vs previous created (track set of ids)
	•	insurance increased by required_fee and vault increased by same
On Err: unchanged state.

Deposit
On Ok:
	•	vault += amount
	•	account.capital += amount
	•	Also: because deposit calls settle_warmup_to_capital, it may modify pnl/capital further. So check delta carefully:
	•	At minimum: vault increased by exactly amount.
	•	account.capital increased by at least 0 and at most amount + warmed_from_pnl (hard to compute).
So don’t assert exact capital delta; assert:
	•	vault_after == vault_before + amount
	•	capital_after >= capital_before (deposit shouldn’t reduce capital overall)
And global invariants.

Withdraw
On Ok:
	•	vault_after == vault_before - amount
	•	capital_after == capital_before - amount after warmup settlement; since withdraw calls settle_warmup first, you can snapshot post-settle only by doing a separate call; easiest: assert:
	•	capital_after <= capital_before (strict decrease by amount should hold if you snapshot right before withdraw, but warmup could increase capital first; so: assert capital_after + amount <= capital_before + possible_warmup_gain is messy)
So for withdraw, do this pattern:
	•	Call withdraw
	•	On Ok: assert vault_after == vault_before - amount
	•	Also assert account.capital + amount <= old_capital + clamp_pos(old_pnl) (withdraw can only come from capital, but warmup can convert pnl to capital; this inequality is safe)
On Err: snapshot unchanged.

AdvanceSlot
Always Ok (no Result). Assert:
	•	current_slot monotone increasing
	•	warmup pause semantics not violated.

AccrueFunding
On Ok:
	•	last_funding_slot updated to now_slot
	•	funding_index_qpb_e6 changes by expected formula (you can compute reference)
On Err:
	•	unchanged snapshot (especially funding_index_qpb_e6 and last_funding_slot)

Touch
On Ok:
	•	accounts[idx].funding_index == engine.funding_index_qpb_e6
	•	If position_size == 0: pnl unchanged
On Err: unchanged.

ExecuteTrade
On Ok:
	•	positions should net: Δuser_pos == exec_size, Δlp_pos == -exec_size
	•	insurance_fee_revenue increased by fee
	•	insurance_balance increased by fee
	•	If the trade closes a portion, realized pnl should be zero-sum between user and LP before fees. Since fee is taken from user only, the pair sum should drop by fee (minus rounding).
So assert:
	•	(user.pnl + lp.pnl) after == (before) - fee ± 1 (allow 1 rounding)
	•	Maintenance margin holds for both (it already checks but assert anyway)
On Err:
	•	snapshot unchanged. This will expose current mutation bugs if present.

ApplyADL
On Ok:
	•	capital for every account unchanged (ADL must not touch capital)
	•	insurance decreases by at most spendable_unreserved
	•	if loss exceeds (unwrapped + spendable_unreserved), loss_accum increases
On Err: unchanged (shouldn’t error except enforce_op).

PanicSettleAll
On Ok:
	•	risk_reduction_only == true
	•	warmup_paused == true
	•	all positions are 0 for all used accounts
	•	entry_price == oracle_price for any account that had a position (hard to know); simpler: just check positions are 0.
	•	After it runs, global invariants must hold.

ForceRealizeLosses
If insurance > floor => must return Unauthorized and state unchanged.
If allowed and Ok:
	•	risk_reduction_only == true, warmup_paused == true
	•	all positions are 0
	•	No positive warmup conversions happened (it doesn’t call settle_warmup_to_capital); don’t assert warmed_pos, but you can assert warmed_pos_total did not increase in this function (shouldn’t).
	•	warmed_neg_total increases by sum(payments from capital) (hard to compute), so just assert monotone nondecreasing.

TopUpInsurance
On Ok:
	•	vault += amount
	•	if loss_accum > 0 it decreases first
	•	if after coverage loss_accum == 0 and insurance >= floor => may exit risk mode (assert that exit condition matches function result boolean)
On Err: unchanged.

Done when: proptest state machine runs 1k+ cases without flaking and fails on intentionally injected bug (see §8).

⸻

5) Add a small set of “unit property” fuzz tests (surgical)

In implement 8–12 focused props that are cheap and pinpoint blame:
	1.	withdrawable_pnl monotone in slot for positive pnl (with pause semantics)
	2.	withdrawable_pnl == 0 if pnl<=0 or slope==0 or elapsed==0
	3.	warmup_paused freezes progress: if paused, increasing current_slot beyond pause_slot does not change withdrawable
	4.	settle_warmup_to_capital idempotent at same slot (call twice, no change)
	5.	settle_warmup_to_capital never decreases warmed_pos_total / warmed_neg_total / warmup_insurance_reserved (monotone)
	6.	apply_adl never changes any capital
	7.	touch_account idempotent if global index unchanged
	8.	accrue_funding with dt=0 is no-op
	9.	account_fee_multiplier monotone increasing as remaining decreases (sanity)
	10.	add_user/add_lp never allocates when num_used_accounts >= max_accounts

Each must follow the “no silent skipping” rule.

⸻

6) Deterministic regression fuzzer (seeded) with shrinking-like output

	•	Keep your xorshift approach, but upgrade:
	•	log seed, step, last action when failing
	•	store last N actions to print minimal repro trace
	•	Run seeds 1..=2000, steps 500 each (tunable)

Invariants checked after each step:
	•	assert_global_invariants
	•	plus: “if op Err => unchanged snapshot”

This test becomes your “CI hammer.”

⸻

7) Two parameter regimes (must test both)

Claude must run the whole suite under two RiskParams presets:

Regime A: Normal mode
	•	risk_reduction_threshold = 0 or small
	•	warmup_period_slots = 100

Regime B: Floor + risk mode sensitivity
	•	risk_reduction_threshold = 1000
	•	warmup_period_slots = 100

Reason: many invariants only activate when floor/reservation matters.

⸻

8) Prove the fuzz suite actually catches bugs (mandatory)

Claude must add a short “canary bug” patch in a temporary branch (or behind a cfg flag) and show tests fail:

Examples:
	•	In apply_adl, mistakenly subtract from capital instead of pnl
	•	In settle_warmup_to_capital, remove reservation increment
	•	In execute_trade, apply fee to both user and LP

Done when: at least one test fails for each canary. Then revert the canary.

⸻

9) Output expectations (so Claude doesn’t underdeliver)

Claude’s final PR must include:
	•	The new fuzz harness files
	•	Shared invariants helper
	•	At least one state-machine proptest
	•	Deterministic seeded fuzzer
	•	All tests follow: Err => no mutation (or explicitly documented exception)
	•	README / TESTING.md snippet:
	•	how to run: cargo test --features fuzz
	•	how to run deterministic only
	•	how to increase cases via PROPTEST_CASES

⸻

10) One explicit warning to Claude (important)

Your current execute_trade() appears to mutate state before margin checks, meaning it can return Err(Undercollateralized) after charging fees / changing pnl/positions. The plan above will catch that. Claude must not weaken tests; instead, if failures appear, fix the engine so execute_trade is atomic on failure (either pre-check, or compute into temps and commit at end).

⸻

If you want, I can also give Claude a ready-to-paste Action enum + proptest strategy skeleton that is biased toward valid indices and triggers the nasty paths (floor crossings, warmup pause, force_realize gate).
