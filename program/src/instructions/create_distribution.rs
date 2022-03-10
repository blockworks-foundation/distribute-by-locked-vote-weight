use crate::error::*;
use crate::state::*;
use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token::{Mint, Token, TokenAccount};
use std::mem::size_of;
use voter_stake_registry::state as vsr;

#[derive(Accounts)]
#[instruction(index: u64)]
pub struct CreateDistribution<'info> {
    #[account(
        init,
        seeds = [b"distribution".as_ref(), admin.key().as_ref(), &index.to_le_bytes()],
        bump,
        payer = payer,
        space = 8 + size_of::<Distribution>()
    )]
    pub distribution: AccountLoader<'info, Distribution>,
    pub admin: Signer<'info>,

    pub registrar: AccountLoader<'info, vsr::Registrar>,

    pub mint: Account<'info, Mint>,

    #[account(
        init,
        associated_token::authority = distribution,
        associated_token::mint = mint,
        payer = payer
    )]
    pub vault: Account<'info, TokenAccount>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn create_distribution(
    ctx: Context<CreateDistribution>,
    index: u64,
    registration_end_ts: u64,
    weight_ts: u64,
) -> Result<()> {
    require!(
        registration_end_ts <= weight_ts,
        ErrorKind::WeightNotDuringRegistration
    );

    let bump = Pubkey::find_program_address(
        &[
            b"distribution".as_ref(),
            ctx.accounts.admin.key().as_ref(),
            &index.to_le_bytes(),
        ],
        &crate::id(),
    )
    .1;

    let mut distribution = ctx.accounts.distribution.load_init()?;
    *distribution = Distribution {
        admin: ctx.accounts.admin.key(),
        registrar: ctx.accounts.registrar.key(),
        vault: ctx.accounts.vault.key(),
        mint: ctx.accounts.mint.key(),
        index,
        bump,
        participant_total_weight: 0,
        registration_end_ts,
        weight_ts,
        in_claim_phase: false,
        total_amount_to_distribute: 0,
        time_offset: 0,
        participant_count: 0,
        claim_count: 0,
        reserved: [0; 38],
    };

    Ok(())
}
