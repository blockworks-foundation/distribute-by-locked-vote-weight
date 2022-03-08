use crate::error::*;
use crate::state::*;
use anchor_lang::prelude::*;
use voter_stake_registry::state as vsr;

#[derive(Accounts)]
pub struct UpdateParticipant<'info> {
    #[account(
        mut,
        has_one = registrar,
    )]
    pub distribution: AccountLoader<'info, Distribution>,

    #[account(
        mut,
        has_one = distribution,
        has_one = voter,
    )]
    pub participant: AccountLoader<'info, Participant>,

    #[account(
        has_one = registrar,
    )]
    pub voter: AccountLoader<'info, vsr::Voter>,
    pub registrar: AccountLoader<'info, vsr::Registrar>,
}

pub fn update_participant(ctx: Context<UpdateParticipant>) -> Result<()> {
    let mut distribution = ctx.accounts.distribution.load_mut()?;
    let now_ts = distribution.clock_unix_timestamp();
    require!(now_ts <= distribution.end_ts, ErrorKind::SomeError);
    require!(!distribution.in_claim_phase, ErrorKind::SomeError);

    // unset
    let mut participant = ctx.accounts.participant.load_mut()?;
    distribution.participant_total_weight = distribution
        .participant_total_weight
        .saturating_sub(participant.weight);
    participant.weight = 0;

    // compute new weight
    let voter = ctx.accounts.voter.load()?;
    let registrar = ctx.accounts.registrar.load()?;
    // TODO: compute the weight at distribution.weight_ts, and get only locked-token contributions
    let weight = voter.weight(&registrar).map_err(|_| ErrorKind::SomeError)?;
    require!(weight > 0, ErrorKind::SomeError);

    // set
    participant.weight = weight;
    distribution.participant_total_weight += weight;

    Ok(())
}
