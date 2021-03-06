use crate::error::*;
use crate::state::*;
use anchor_lang::prelude::*;
use voter_stake_registry::state as vsr;

/// Updates the weight associated with a participant.
///
/// When a voter locks up more tokens, their weight will increase. Call this to
/// let the distribution and participant accounts know about the update.
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
    require!(
        distribution.in_registration_phase(),
        ErrorKind::TooLateToRegister
    );

    // compute new weight
    let voter = ctx.accounts.voter.load()?;
    let registrar = ctx.accounts.registrar.load()?;
    let weight = distribution.voter_weight(&registrar, &voter)?;
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
        .saturating_sub(participant.weight.into());
    participant.weight = weight;
    distribution.participant_total_weight = distribution
        .participant_total_weight
        .checked_add(weight.into())
        .unwrap();

    Ok(())
}
