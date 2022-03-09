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
    require!(now_ts < distribution.registration_end_ts, ErrorKind::TooLateToRegister);
    require!(!distribution.in_claim_phase, ErrorKind::TooLateToRegister);

    // compute new weight
    let voter = ctx.accounts.voter.load()?;
    let registrar = ctx.accounts.registrar.load()?;
    // TODO: compute the weight at distribution.weight_ts, and get only locked-token contributions
    let weight = voter.weight(&registrar).map_err(|err| {
        msg!("vsr error: {}", err);
        ErrorKind::VoterStakeRegistryError
    })?;
    require!(weight > 0, ErrorKind::NoLockedVoteWeight);

    // unregister old weight and set the new one
    let mut participant = ctx.accounts.participant.load_mut()?;
    // it should be impossible for locked token weight to decrease on a second call
    // since only fully-locked tokens enter the computation
    require!(
        weight >= participant.weight,
        ErrorKind::WeightMustNotDecrease
    );
    distribution.participant_total_weight = distribution
        .participant_total_weight
        .saturating_sub(participant.weight);
    participant.weight = weight;
    distribution.participant_total_weight = distribution
        .participant_total_weight
        .checked_add(weight)
        .unwrap();

    Ok(())
}
