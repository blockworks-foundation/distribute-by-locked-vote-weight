use crate::error::*;
use crate::state::*;
use anchor_lang::prelude::*;
use std::mem::size_of;
use voter_stake_registry::state as vsr;

/// Creates a participant for a distribution, based on their voter account.
///
/// Having a participant account means that Claim can be called when the claim
/// phase has started. Use UpdateParticipant if the voter's weight increases and
/// you want to update the value stored in the participant account.
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
    require!(
        distribution.in_registration_phase(),
        ErrorKind::TooLateToRegister
    );

    let voter = ctx.accounts.voter.load()?;
    let registrar = ctx.accounts.registrar.load()?;
    let weight = distribution.voter_weight(&registrar, &voter)?;
    require!(weight > 0, ErrorKind::NoLockedVoteWeight);

    let mut participant = ctx.accounts.participant.load_init()?;
    *participant = Participant {
        distribution: ctx.accounts.distribution.key(),
        voter: ctx.accounts.voter.key(),
        voter_authority: voter.voter_authority,
        payer: ctx.accounts.payer.key(),
        weight,
    };
    distribution.participant_total_weight = distribution
        .participant_total_weight
        .checked_add(weight.into())
        .unwrap();
    distribution.participant_count = distribution.participant_count.checked_add(1).unwrap();

    Ok(())
}
