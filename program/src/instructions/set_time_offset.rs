use crate::error::*;
use crate::state::*;
use anchor_lang::prelude::*;

/// Debug-only instruction, used to advance time in tests
#[derive(Accounts)]
#[instruction(time_offset: i64)]
pub struct SetTimeOffset<'info> {
    #[account(mut, has_one = admin)]
    pub distribution: AccountLoader<'info, Distribution>,
    pub admin: Signer<'info>,
}

pub fn set_time_offset(ctx: Context<SetTimeOffset>, time_offset: i64) -> Result<()> {
    // TODO: Limit using this instruction to one specific admin key in tests
    let distribution = &mut ctx.accounts.distribution.load_mut()?;
    distribution.time_offset = time_offset;
    Ok(())
}
