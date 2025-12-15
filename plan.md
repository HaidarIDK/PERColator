Step-by-step plan for Claude: rewrite to a single fixed 4096-account slab (no_std, no unsafe, no options)

Hard requirements (must follow exactly)
	•	#![no_std]
	•	#![forbid(unsafe_code)]
	•	No Vec, no heap allocation, no alloc crate usage
	•	No AccountStorage<T> trait
	•	Single unified account array: 4096 max accounts
	•	Users and LPs are distinguished by a kind field (no Option fields)
	•	Iteration must use a bitmap to skip unused slots
	•	All scans must be allocation-free and as tight as possible
	•	Keep existing behavior semantics as much as possible (warmup, withdrawal-only mode, liquidation, ADL), but implemented on the new slab

⸻

Step 0 — Create a new design doc + checklist file
	1.	Add docs/slab_4096_rewrite.md with:
	•	New memory layout (RiskEngine + Account)
	•	Indexing scheme (u16 account index)
	•	Bitmap iteration algorithm (word scan + trailing_zeros)
	•	List of functions to port
	•	Tests + Kani harness list (from this plan)
	2.	Do not change code until the doc exists.

⸻

Step 1 — Delete heap/Vec dependency
	1.	Remove:
	•	extern crate alloc;
	•	use alloc::vec::Vec;
	2.	Remove the AccountStorage<T> trait and its Vec<T> impl.
	3.	Remove RiskEngine<U,L> generics and VecRiskEngine alias.

Goal checkpoint: code compiles (even if incomplete) without alloc.

⸻

Step 2 — Define new constants and enums

Add near top-level:

pub const MAX_ACCOUNTS: usize = 4096;
pub const BITMAP_WORDS: usize = MAX_ACCOUNTS / 64; // 64

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccountKind {
    User = 0,
    LP = 1,
}


⸻

Step 3 — Replace Warmup and Account with fixed, copyable, no-Option layout

3.1 Remove the old Warmup struct and embed the fields directly

Define the new account:

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Account {
    pub kind: AccountKind,

    // capital & pnl
    pub capital: u128,
    pub pnl: i128,
    pub reserved_pnl: u128,

    // warmup
    pub warmup_started_at_slot: u64,
    pub warmup_slope_per_step: u128,

    // position
    pub position_size: i128,
    pub entry_price: u64,

    // funding
    pub funding_index: i128,

    // LP matcher info (meaningful only for LP)
    pub matcher_program: [u8; 32],
    pub matcher_context: [u8; 32],
}

3.2 Remove matching_engine() method and replace with simple checks

Add helper methods:

impl Account {
    pub fn is_user(&self) -> bool { self.kind == AccountKind::User }
    pub fn is_lp(&self) -> bool { self.kind == AccountKind::LP }
}


⸻

Step 4 — Rewrite RiskEngine as a single slab with bitmap + freelist

4.1 Define engine

Replace the old RiskEngine with:

#[repr(C)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RiskEngine {
    pub vault: u128,

    pub insurance_fund: InsuranceFund, // keep as-is or inline balance+revenues

    pub params: RiskParams,

    pub current_slot: u64,

    pub funding_index_qpb_e6: i128,
    pub last_funding_slot: u64,

    pub loss_accum: u128,
    pub withdrawal_only: bool,
    pub withdrawal_mode_withdrawn: u128,

    pub warmup_paused: bool,
    pub warmup_pause_slot: u64,

    // occupancy
    pub used: [u64; BITMAP_WORDS],

    // freelist (no unsafe overlays)
    pub free_head: u16,              // 0..4095, or u16::MAX for none
    pub next_free: [u16; MAX_ACCOUNTS],

    // accounts
    pub accounts: [Account; MAX_ACCOUNTS],
}

4.2 Add EMPTY_ACCOUNT constructor for initialization (no const Account needed)

Because Account contains arrays, use a fn empty_account() -> Account helper and build the array in new() by repeated value fill.

Implement:

fn empty_account() -> Account { Account { kind: AccountKind::User, capital:0, pnl:0, reserved_pnl:0,
  warmup_started_at_slot:0, warmup_slope_per_step:0, position_size:0, entry_price:0,
  funding_index:0, matcher_program:[0;32], matcher_context:[0;32] } }

4.3 Implement new(params) to initialize:
	•	used all zero
	•	free_head = 0
	•	next_free[i] = i+1, last = u16::MAX
	•	accounts[i] = empty_account()

⸻

Step 5 — Implement bitmap helpers and tight iteration

Add helpers:

fn is_used(&self, idx: usize) -> bool {
    let w = idx >> 6;
    let b = idx & 63;
    ((self.used[w] >> b) & 1) == 1
}

fn set_used(&mut self, idx: usize) {
    let w = idx >> 6;
    let b = idx & 63;
    self.used[w] |= 1u64 << b;
}

fn clear_used(&mut self, idx: usize) {
    let w = idx >> 6;
    let b = idx & 63;
    self.used[w] &= !(1u64 << b);
}

Add a tight iterator-like internal method (not trait-based):

fn for_each_used_mut<F: FnMut(usize, &mut Account)>(&mut self, mut f: F) {
    for (block, word) in self.used.iter().copied().enumerate() {
        let mut w = word;
        while w != 0 {
            let bit = w.trailing_zeros() as usize;
            let idx = block * 64 + bit;
            w &= w - 1;
            f(idx, &mut self.accounts[idx]);
        }
    }
}

Also an immutable version if needed.

⸻

Step 6 — Implement O(1) account allocation / creation

6.1 alloc_slot() -> Result<u16>
	•	if free_head == u16::MAX return error
	•	idx = free_head
	•	free_head = next_free[idx]
	•	mark used bit
	•	return idx

6.2 add_user(fee_payment) -> Result<u16>
	•	allocate slot
	•	initialize account fields:
	•	kind=User
	•	funding_index = engine funding index
	•	warmup_started_at_slot = current_slot
	•	matcher arrays zero
	•	apply creation fee to insurance as before
	•	return index

6.3 add_lp(program, context, fee_payment) -> Result<u16>
	•	allocate slot
	•	initialize with kind=LP, store program/context
	•	rest same as user

Do not implement delete/free now (keep simplest). No free_slot().

⸻

Step 7 — Port “touch” funding settlement to unified accounts

7.1 Replace touch_user and touch_lp with one:

pub fn touch_account(&mut self, idx: u16) -> Result<()> { ... }

	•	validate used bit set
	•	settle funding using existing logic, operating on self.accounts[idx]
	•	update funding_index snapshot

7.2 Update all call sites to call touch_account(user_idx) / touch_account(lp_idx).

⸻

Step 8 — Port warmup math to new fields + warmup pause

8.1 Rewrite withdrawable_pnl(&self, acct: &Account) -> u128

Use:
	•	acct.warmup_started_at_slot
	•	acct.warmup_slope_per_step
	•	apply global warmup pause exactly as previously planned (effective slot clamp)

8.2 Rewrite update_warmup_slope(idx: u16) -> Result<()>

Single function for all accounts:
	•	reads account.pnl
	•	calculates desired slope
	•	respects global warmup rate cap (use existing total_warmup_rate logic only if you keep it; otherwise remove it entirely for simplicity)
	•	If warmup_paused, do not change warmup_started_at_slot
	•	store slope

(For this rewrite: keep the slope cap logic if you already rely on it; otherwise delete total_warmup_rate completely and set slope = pnl/T. Simplest is: remove total_warmup_rate and do no global cap. If you keep it, you must also maintain it without scanning. Since you asked “tight loops and simplest,” delete total_warmup_rate and the cap fields in params.)

Explicit instruction: Delete total_warmup_rate and both warmup rate cap params to avoid O(N) maintenance.

⸻

Step 9 — Port deposit/withdraw for unified slab

9.1 deposit(idx: u16, amount: u128)
	•	validate used
	•	accounts[idx].capital += amount
	•	vault += amount

9.2 withdraw(idx: u16, amount: u128)
	•	validate used
	•	touch_account(idx)
	•	compute warmed_up_pnl via withdrawable_pnl
	•	convert warmed_up_pnl into capital (same as before)
	•	apply withdrawal-only haircut logic (keep existing formula; if it scans, rewrite to O(1) aggregates OR drop haircut and keep withdrawal-only = “only closing positions and no withdraw” — simplest is: in withdrawal_only mode disallow withdrawals entirely)

Explicit instruction (simplest, no aggregates):
	•	In withdrawal_only == true, return Err(WithdrawalOnlyMode) for withdrawals.
	•	Keep top_up_insurance_fund as recovery method.

This removes the need for O(N) principal haircut and keeps everything tight.

(You asked for simplest; this is the simplest consistent rule.)

9.3 Add lp_deposit and lp_withdraw as aliases

Do not keep separate APIs. Use deposit/withdraw for all accounts; the kind does not matter.

⸻

Step 10 — Port trading using indices into the slab

Rewrite execute_trade to:
	•	assert used bits for both indices
	•	assert account kinds:
	•	lp.kind == LP
	•	user.kind == User
	•	touch both
	•	call matching engine using lp’s program/context
	•	update pnl and position fields exactly as before
	•	call update_warmup_slope on both indices

Remove all references to self.users / self.lps.

⸻

Step 11 — Rewrite ADL to be scan-based and tight (4096 fixed)

Explicit instruction (simplest + tight): keep your original waterfall exactly, implemented with two tight bitmap passes, no allocations.

11.1 First pass: compute total_unwrapped
	•	iterate used accounts via bitmap
	•	for each, compute unwrapped using:
	•	positive pnl
	•	withdrawable_pnl
	•	reserved_pnl
	•	sum into total_unwrapped

11.2 Second pass: apply proportional haircut
	•	loss_to_socialize = min(total_loss, total_unwrapped)
	•	iterate used accounts again
	•	compute that same account’s unwrapped
	•	haircut_i = loss_to_socialize * unwrapped / total_unwrapped
	•	subtract haircut_i from acct.pnl

11.3 Remaining loss after unwrapped
	•	remaining_loss = total_loss - loss_to_socialize
	•	if remaining_loss > 0:
	•	debit insurance_fund.balance
	•	if insurance insufficient:
	•	set loss_accum to leftover
	•	set withdrawal_only = true
	•	set warmup_paused=true and warmup_pause_slot=current_slot

No Vecs, no intermediate lists.

⸻

Step 12 — Port liquidation to unified slab

Rewrite liquidate_account(victim_idx: u16, keeper_idx: u16, oracle_price: u64)
	•	validate used
	•	touch victim
	•	check maintenance margin
	•	close position to 0, realize pnl, charge fees
	•	pay keeper share to keeper.pnl
	•	update warmup slope for victim and keeper

No separate user/lp functions.

⸻

Step 13 — Tests (must add)

Add #[cfg(test)] extern crate std; and unit tests:
	1.	Bitmap allocation test

	•	add 10 accounts
	•	ensure used bits set and indices unique

	2.	Scan ADL does not allocate and works

	•	set pnl states for many accounts
	•	run apply_adl
	•	verify pnl reductions and insurance usage order

	3.	Warmup pause works

	•	pause warmup and ensure withdrawable doesn’t increase with slot advancement

	4.	Withdrawal-only blocks withdraw

	•	set withdrawal_only=true
	•	withdraw must error

	5.	Compute/Conservation sanity

	•	after sequences, check_conservation() true (update it to scan slab via bitmap)

⸻

Step 14 — Kani proofs (must add)

Add harnesses:
	1.	No principal reduction in ADL

	•	set up 1-3 accounts, run apply_adl, assert capital unchanged for all

	2.	Insurance not used until unwrapped exhausted

	•	set up accounts with enough unwrapped to cover loss
	•	apply_adl(loss)
	•	assert insurance unchanged

	3.	Bitmap iteration never touches unused slots

	•	construct used bitmap with a single bit and ensure only that idx changes (use sentinels)

⸻

Step 15 — Delete dead code + final audit

Claude must:
	•	remove all old structs/traits/modules no longer referenced
	•	grep to ensure:
	•	no alloc
	•	no Vec
	•	no Option<[u8;32]> in Account
	•	no AccountStorage
	•	no unsafe (forbidden anyway)
	•	ensure all loops use bitmap iteration, not 0..4096 naive loops

⸻

Final acceptance checklist (Claude must include in PR description)
	•	single slab accounts: [Account; 4096]
	•	bitmap iteration implemented and used in scans
	•	no heap, no alloc, no vec
	•	apply_adl uses two bitmap passes, no allocations
	•	withdrawal-only blocks withdrawals (simplest)
	•	warmup pause implemented globally
	•	tests added and passing
	•	kani harnesses added and passing
