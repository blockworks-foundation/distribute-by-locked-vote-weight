use crate::error::*;
use crate::events::Info;
use crate::state::*;
use anchor_lang::prelude::*;
use anchor_spl::token::TokenAccount;
use voter_stake_registry::state as vsr;

/// Logs an Info event. Used for getting status information in uis.
#[derive(Accounts)]
pub struct LogInfo<'info> {
    #[account(
        mut,
        has_one = registrar,
        has_one = vault,
    )]
    pub distribution: AccountLoader<'info, Distribution>,
    pub vault: Account<'info, TokenAccount>,

    // Can be an empty account if the participant isn't created yet
    #[account(
        seeds = [distribution.key().as_ref(), b"participant".as_ref(), &voter.key().as_ref()],
        bump,
    )]
    pub participant: UncheckedAccount<'info>,

    #[account(
        has_one = registrar,
    )]
    pub voter: AccountLoader<'info, vsr::Voter>,
    pub registrar: AccountLoader<'info, vsr::Registrar>,
}

pub fn log_info(ctx: Context<LogInfo>) -> Result<()> {
    let distribution = ctx.accounts.distribution.load()?;
    let now_ts = distribution.clock_unix_timestamp();
    let can_register = now_ts < distribution.registration_end_ts;

    let voter = ctx.accounts.voter.load()?;
    let registrar = ctx.accounts.registrar.load()?;
    let usable_weight = if can_register {
        Some(distribution.voter_weight(&registrar, &voter)?)
    } else {
        None
    };
    let registered_weight =
        AccountLoader::<'_, Participant>::try_from(&ctx.accounts.participant.to_account_info())
            .and_then(|l| l.load().map(|p| p.weight))
            .ok();

    emit!(Info {
        participant_total_weight: distribution.participant_total_weight,
        distribution_amount: ctx.accounts.vault.amount,
        can_start_claim_phase: !distribution.in_claim_phase && !can_register,
        in_claim_phase: distribution.in_claim_phase,
        usable_weight,
        registered_weight,
    });

    Ok(())
}
