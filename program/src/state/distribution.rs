use anchor_lang::prelude::*;

/// Instance of a voting rights distributor.
#[account(zero_copy)]
pub struct Distribution {
    pub admin: Pubkey,
    pub registrar: Pubkey,
    pub mint: Pubkey,
    pub vault: Pubkey,
    pub index: u64,

    pub bump: u8,

    // participants can only be created before this time
    // claims can only be made after this time
    pub end_ts: u64,

    // the time for which the vote weight from locked tokens is computed
    // if this is one year in the future, then only lockups that last for
    // at least one year can contribute
    pub weight_ts: u64,

    pub participant_total_weight: u64,

    pub in_claim_phase: bool,
    pub total_amount_to_distribute: u64,

    /// Debug only: time offset, to allow tests to move forward in time.
    pub time_offset: i64,
}
// const_assert!(std::mem::size_of::<Registrar>() == 5 * 32 + 4 * 152 + 8 + 1 + 95);
// const_assert!(std::mem::size_of::<Registrar>() % 8 == 0);

impl Distribution {
    pub fn clock_unix_timestamp(&self) -> u64 {
        Clock::get()
            .unwrap()
            .unix_timestamp
            .checked_add(self.time_offset)
            .unwrap() as u64
    }
}

#[macro_export]
macro_rules! distribution_seeds {
    ( $distribution:expr ) => {
        &[
            b"distribution".as_ref(),
            $distribution.admin.as_ref(),
            &$distribution.index.to_le_bytes(),
            &[$distribution.bump],
        ]
    };
}

pub use distribution_seeds;
