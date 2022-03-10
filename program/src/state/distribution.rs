use crate::error::*;
use anchor_lang::prelude::*;
use voter_stake_registry::state as vsr;

/// Instance of a voting rights distributor.
#[account(zero_copy)]
pub struct Distribution {
    pub admin: Pubkey,
    pub registrar: Pubkey,
    pub mint: Pubkey,
    pub vault: Pubkey,
    pub index: u64,

    /// participants can only be created before this time
    /// Claim can only be called after this time
    pub registration_end_ts: u64,

    /// the time for which the vote weight from locked tokens is computed
    /// if this is one year in the future, then only lockups that last for
    /// at least one year can contribute
    pub weight_ts: u64,

    /// sum of the weights from all participants
    pub participant_total_weight: u128,

    /// the amount of tokens seen in the distribution vault when the claim phase started
    pub total_amount_to_distribute: u64,

    /// Debug only: time offset, to allow tests to move forward in time.
    pub time_offset: i64,

    pub participant_count: u32,
    pub claim_count: u32,

    pub bump: u8,

    pub reserved: [u8; 39],
}
const_assert!(std::mem::size_of::<Distribution>() == 4 * 32 + 7 * 8 + 2 * 4 + 1 + 39);
const_assert!(std::mem::size_of::<Distribution>() % 8 == 0);

impl Distribution {
    pub fn clock_unix_timestamp(&self) -> u64 {
        Clock::get()
            .unwrap()
            .unix_timestamp
            .checked_add(self.time_offset)
            .unwrap() as u64
    }

    pub fn voter_weight(&self, registrar: &vsr::Registrar, voter: &vsr::Voter) -> Result<u64> {
        let now_ts = self.clock_unix_timestamp() as i64;
        Ok(voter
            .weight_locked_guaranteed(&registrar, now_ts, self.weight_ts as i64)
            .map_err(|err| {
                msg!("vsr error: {}", err);
                ErrorKind::VoterStakeRegistryError
            })?)
    }

    pub fn in_registration_phase(&self) -> bool {
        self.clock_unix_timestamp() < self.registration_end_ts
    }

    pub fn in_claim_phase(&self) -> bool {
        !self.in_registration_phase()
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
