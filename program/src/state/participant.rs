use anchor_lang::prelude::*;

/// User account for minting voting rights.
#[account(zero_copy)]
pub struct Participant {
    pub distribution: Pubkey,
    pub voter: Pubkey,
    pub voter_authority: Pubkey,
    pub weight: u64,
}
const_assert!(std::mem::size_of::<Participant>() == 3 * 32 + 8);
const_assert!(std::mem::size_of::<Participant>() % 8 == 0);
