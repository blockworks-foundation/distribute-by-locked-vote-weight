use anchor_lang::prelude::*;

/// User account for minting voting rights.
#[account(zero_copy)]
pub struct Participant {
    pub distribution: Pubkey,
    pub voter_authority: Pubkey,
    pub weight: u64,
    pub claimed: bool,
}
// const_assert!(std::mem::size_of::<Voter>() == 2 * 32 + 32 * 80 + 2 + 94);
// const_assert!(std::mem::size_of::<Voter>() % 8 == 0);
