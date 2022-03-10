use anchor_lang::prelude::*;

#[event]
#[derive(Debug)]
pub struct Info {
    /// The sum of the weights of all currently registered participants
    pub participant_total_weight: u128,
    /// The current distribution vault balance
    pub distribution_amount: u64,
    /// Can the claim phase be started?
    pub can_start_claim_phase: bool,
    /// Can claims be made?
    pub in_claim_phase: bool,

    /// The voter's current weight (if registration/update still possible)
    pub usable_weight: Option<u64>,
    /// The weight the participant is registered with
    pub registered_weight: Option<u64>,
}
