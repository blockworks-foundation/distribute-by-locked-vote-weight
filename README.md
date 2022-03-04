# Description

This program can be used to distribute tokens to users.
Users get a fraction of the allocated funds, based on their locked vote weight
at some point in the future.

Example:
- `create_distribution` with
  - `end_ts` = one month from now: This is when the register phase ends and the claim phase begins.
  - `weight_ts` = one year from now: This is the time for which eligible locked vote weight will be calculated.
  - a specific voter-stake-registry registrar
- deposit tokens to the ATA of the distribution
- anyone with a voter account on the voter-stake-registry registrar can register with the distribution by calling `create_participant`
- when end_ts has been reachend, someone calls `start_claim_phase` permissionlessly
- anyone who's previously registered can `claim` their share of the deposited tokens

## How is the vote weight used exactly?

When users call `create_participant` their vote weight at `weight_ts` is calculated,
counting only vote weight generated from locked tokens.
Constant lockups are treated as if they were cliffs.

Examples for a case where `weight_ts` is two years in the future:
1. You have a one-year cliff or one-year constant lockup: You have no weight and can't register.
2. You have a three-year cliff lockup: Your weight is the bonus vote weight produced by a one-year cliff lockup.
3. You have a five-year constant lockup: Your weight is the bonus vote weight produced by a three-year cliff lockup.
4. You have a five-year monthly vested lockup: Your weight is the bonus vote weight produced by the tokens that'll still be locked in two years, adjusted for their vesting points.

# License

This code is currently not free to use while in development.

