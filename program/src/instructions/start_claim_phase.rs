use crate::error::*;
use crate::state::*;
use anchor_lang::prelude::*;
use anchor_spl::token::TokenAccount;

#[derive(Accounts)]
pub struct StartClaimPhase<'info> {
    #[account(
        mut,
        has_one = vault
    )]
    pub distribution: AccountLoader<'info, Distribution>,

    pub vault: Box<Account<'info, TokenAccount>>,
}

pub fn start_claim_phase(ctx: Context<StartClaimPhase>) -> Result<()> {
    let mut distribution = ctx.accounts.distribution.load_mut()?;
    let now_ts = distribution.clock_unix_timestamp();
    require!(
        now_ts >= distribution.end_ts,
        ErrorKind::TooEarlyForClaimPhase
    );
    require!(
        !distribution.in_claim_phase,
        ErrorKind::ClaimPhaseAlreadyStarted
    );

    distribution.in_claim_phase = true;
    distribution.total_amount_to_distribute = ctx.accounts.vault.amount;

    // TODO: freeze vault to avoid late deposits?

    Ok(())
}
