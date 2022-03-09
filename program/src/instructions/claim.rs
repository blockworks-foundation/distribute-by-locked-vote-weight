use crate::error::*;
use crate::state::*;
use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount};

#[derive(Accounts)]
pub struct Claim<'info> {
    #[account(
        has_one = vault,
    )]
    pub distribution: AccountLoader<'info, Distribution>,

    #[account(
        mut,
        has_one = distribution,
        has_one = voter_authority,
        close = sol_destination,
    )]
    pub participant: AccountLoader<'info, Participant>,

    #[account(mut)]
    pub vault: Box<Account<'info, TokenAccount>>,

    #[account(mut)]
    pub target_token: Box<Account<'info, TokenAccount>>,

    pub voter_authority: Signer<'info>,

    #[account(mut)]
    pub sol_destination: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
}

impl<'info> Claim<'info> {
    pub fn transfer_ctx(&self) -> CpiContext<'_, '_, '_, 'info, token::Transfer<'info>> {
        let program = self.token_program.to_account_info();
        let accounts = token::Transfer {
            from: self.vault.to_account_info(),
            to: self.target_token.to_account_info(),
            authority: self.distribution.to_account_info(),
        };
        CpiContext::new(program, accounts)
    }
}

pub fn claim(ctx: Context<Claim>) -> Result<()> {
    let distribution = ctx.accounts.distribution.load()?;
    require!(distribution.in_claim_phase, ErrorKind::NotInClaimPhase);

    let participant = ctx.accounts.participant.load()?;

    // This rounds down, meaning not all tokens may be fully distributed.
    let amount = u64::try_from(
        (distribution.total_amount_to_distribute as u128)
            .checked_mul(participant.weight as u128)
            .unwrap()
            .checked_div(distribution.participant_total_weight)
            .unwrap(),
    )
    .unwrap();

    token::transfer(
        ctx.accounts
            .transfer_ctx()
            .with_signer(&[distribution_seeds!(distribution)]),
        amount,
    )?;

    Ok(())
}
