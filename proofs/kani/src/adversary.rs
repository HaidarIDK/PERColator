//! Adversarial step generator

#[cfg(kani)]
use kani::any;
use model_safety::{state::*, transitions::*};

#[derive(Clone, Copy)]
pub enum Step {
    Deposit,
    Trade,
    Loss,
    Socialize,
    WithdrawP,
    WithdrawPnL,
    Tick,
    MatcherNoise,
}

#[cfg(kani)]
impl kani::Arbitrary for Step {
    fn any() -> Self {
        let choice: u8 = any();
        match choice % 8 {
            0 => Step::Deposit,
            1 => Step::Trade,
            2 => Step::Loss,
            3 => Step::Socialize,
            4 => Step::WithdrawP,
            5 => Step::WithdrawPnL,
            6 => Step::Tick,
            _ => Step::MatcherNoise,
        }
    }
}

#[cfg(kani)]
pub fn adversary_step(s: State) -> State {
    if s.users.is_empty() {
        return s;
    }

    match any::<Step>() {
        Step::Deposit => {
            let uid: usize = (any::<u8>() as usize) % s.users.len();
            let x: u128 = any();
            deposit(s, uid, x)
        }
        Step::Trade => {
            let uid: usize = (any::<u8>() as usize) % s.users.len();
            let r: i128 = any();
            trade_settle(s, uid, r)
        }
        Step::Loss => {
            let d: u128 = any();
            loss_event(s, d)
        }
        Step::Socialize => {
            let d: u128 = any();
            socialize_losses(s, d)
        }
        Step::WithdrawP => {
            let uid: usize = (any::<u8>() as usize) % s.users.len();
            let x: u128 = any();
            withdraw_principal(s, uid, x)
        }
        Step::WithdrawPnL => {
            let uid: usize = (any::<u8>() as usize) % s.users.len();
            let x: u128 = any();
            let step = any::<u32>() % 8;
            withdraw_pnl(s, uid, x, step)
        }
        Step::Tick => {
            let steps: u32 = any::<u32>() % 8;
            tick_warmup(s, steps)
        }
        Step::MatcherNoise => matcher_noise(s),
    }
}
