use crate::error::*;
use crate::state::*;
use anchor_lang::prelude::*;
use std::mem::size_of;
use voter_stake_registry::state as vsr;

#[derive(Accounts)]
pub struct CreateParticipant<'info> {
    #[account(
        mut,
        has_one = registrar,
    )]
    pub distribution: AccountLoader<'info, Distribution>,

    #[account(
        init,
        seeds = [distribution.key().as_ref(), b"participant".as_ref(), &voter.key().as_ref()],
        bump,
        payer = payer,
        space = 8 + size_of::<Participant>()
    )]
    pub participant: AccountLoader<'info, Participant>,

    #[account(
        has_one = registrar,
    )]
    pub voter: AccountLoader<'info, vsr::Voter>,
    pub registrar: AccountLoader<'info, vsr::Registrar>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn create_participant(ctx: Context<CreateParticipant>) -> Result<()> {
    let mut distribution = ctx.accounts.distribution.load_mut()?;
    let now_ts = distribution.clock_unix_timestamp();
    require!(now_ts < distribution.registration_end_ts, ErrorKind::TooLateToRegister);
    require!(!distribution.in_claim_phase, ErrorKind::TooLateToRegister);

    let voter = ctx.accounts.voter.load()?;
    let registrar = ctx.accounts.registrar.load()?;
    // TODO: compute the weight at distribution.weight_ts, and get only locked-token contributions
    let weight = voter.weight(&registrar).map_err(|err| {
        msg!("vsr error: {}", err);
        ErrorKind::VoterStakeRegistryError
    })?;
    require!(weight > 0, ErrorKind::NoLockedVoteWeight);

    let mut participant = ctx.accounts.participant.load_init()?;
    *participant = Participant {
        distribution: ctx.accounts.distribution.key(),
        voter: ctx.accounts.voter.key(),
        voter_authority: voter.voter_authority,
        weight,
    };
    distribution.participant_total_weight = distribution
        .participant_total_weight
        .checked_add(weight)
        .unwrap();

    Ok(())
}
