use anchor_lang::prelude::*;

#[error]
pub enum ErrorKind {
    // 6000 / 0x1770
    #[msg("unknown error")]
    UnknownError,
    // 6001 / 0x1771
    #[msg("the claim phase has already started")]
    ClaimPhaseAlreadyStarted,
    // 6002 / 0x1772
    #[msg("the earliest claim phase start time has not been reached yet")]
    TooEarlyForClaimPhase,
    // 6003 / 0x1773
    #[msg("the claim phase has not started yet")]
    NotInClaimPhase,
    // 6004 / 0x1774
    #[msg("participant creation is closed because the registration phase has ended")]
    TooLateToRegister,
    // 6005 / 0x1775
    #[msg("the voter has no locked vote weight")]
    NoLockedVoteWeight,
    // 6006 / 0x1776
    #[msg("voter-stake-registry had an error")]
    VoterStakeRegistryError,
    // 6007 / 0x1777
    #[msg("weight must not decrease on update, please report this error")]
    WeightMustNotDecrease,
}
