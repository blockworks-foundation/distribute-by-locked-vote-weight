#![cfg(feature = "test-bpf")]

use program_test::*;
use solana_program_test::*;
use solana_sdk::{signature::Keypair, transport::TransportError};

use distribute_by_locked_vote_weight::state::*;

mod program_test;

// This is an unspecific happy-case test that just runs a few instructions to check
// that they work in principle. It should be split up / renamed.
#[tokio::test]
async fn test_update_weight() -> Result<(), TransportError> {
    //
    // SETUP: fake registrar / voter accounts
    //
    let mut test_config = TestConfig::default();

    let now_ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let vsr = vsr_client::setup_mock_registrar_and_voters(&mut test_config, now_ts);

    //
    // SETUP: Start
    //
    let context = TestContext::new(test_config).await;
    let solana = &context.solana.clone();

    let admin = &Keypair::new();
    let payer = &context.users[0].key;
    let mint0 = &context.mints[0];
    let payer_mint0_account = context.users[0].token_accounts[0];
    let voter = &vsr.voters[0];

    //
    // SETUP: distribution and participant
    //
    let accounts = send_tx(
        solana,
        CreateDistributionInstruction {
            index: 0,
            end_ts: now_ts + 100,
            weight_ts: now_ts + 100,
            registrar: vsr.registrar,
            mint: mint0.pubkey,
            admin,
            payer,
        },
    )
    .await
    .unwrap();
    let distribution = accounts.distribution;
    let vault = accounts.vault;

    solana
        .transfer_token(payer_mint0_account, payer, vault, 10_000)
        .await;

    let accounts = send_tx(
        solana,
        CreateParticipantInstruction {
            distribution,
            voter: voter.pubkey,
            payer,
        },
    )
    .await
    .unwrap();
    let participant = accounts.participant;

    let participant_data: Participant = solana.get_account(participant).await;
    let distribution_data: Distribution = solana.get_account(distribution).await;
    assert_eq!(
        distribution_data.participant_total_weight,
        participant_data.weight as u128
    );
    // -1 is due to rounding down as end_ts > now_ts
    assert_eq!(participant_data.weight, voter.locked_amount * 12 / 60 - 1);

    //
    // TEST: Deposit some more into the voter and update the participant weight
    //

    let extra_deposit = 1000;
    send_tx(
        solana,
        vsr_client::DepositInstruction {
            deposit_entry_index: 0,
            amount: extra_deposit,
            voter: voter.pubkey,
            deposit_token: payer_mint0_account,
            deposit_authority: payer,
        },
    )
    .await
    .unwrap();

    send_tx(solana, UpdateParticipantInstruction { participant })
        .await
        .unwrap();

    let participant_data: Participant = solana.get_account(participant).await;
    let distribution_data: Distribution = solana.get_account(distribution).await;
    assert_eq!(
        distribution_data.participant_total_weight,
        participant_data.weight as u128
    );
    assert_eq!(
        participant_data.weight,
        (voter.locked_amount + extra_deposit) * 12 / 60 - 1
    );

    Ok(())
}
