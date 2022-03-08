use anchor_lang::prelude::*;
use error::*;
use instructions::*;

mod error;
mod instructions;
pub mod state;

#[macro_use]
extern crate static_assertions;

// The program address.
declare_id!("2qewLEr5fxtK2Rmqeokgw4vA7HphKGUkXLF1NxWPvDEA");

#[program]
pub mod distribute_by_locked_vote_weight {
    use super::*;

    pub fn create_distribution(
        ctx: Context<CreateDistribution>,
        index: u64,
        end_ts: u64,
        weight_ts: u64,
    ) -> Result<()> {
        instructions::create_distribution(ctx, index, end_ts, weight_ts)
    }

    pub fn create_participant(ctx: Context<CreateParticipant>) -> Result<()> {
        instructions::create_participant(ctx)
    }

    pub fn update_participant(ctx: Context<UpdateParticipant>) -> Result<()> {
        instructions::update_participant(ctx)
    }

    pub fn start_claim_phase(ctx: Context<StartClaimPhase>) -> Result<()> {
        instructions::start_claim_phase(ctx)
    }

    pub fn claim(ctx: Context<Claim>) -> Result<()> {
        instructions::claim(ctx)
    }

    pub fn set_time_offset(ctx: Context<SetTimeOffset>, time_offset: i64) -> Result<()> {
        instructions::set_time_offset(ctx, time_offset)
    }
}
