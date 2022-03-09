#![cfg(feature = "test-bpf")]

use program_test::*;
use solana_program_test::*;
use solana_sdk::{
    signature::{Keypair, Signer},
    transport::TransportError,
};

use distribute_by_locked_vote_weight::state::*;

mod program_test;

// This is an unspecific happy-case test that runs through a particular distribution.
#[tokio::test]
async fn test_full_lifecycle() -> Result<(), TransportError> {
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
    let start_balance = solana.token_account_balance(payer_mint0_account).await;
    let distribution_amount = 1000;
    let voter0 = &vsr.voters[0];
    let voter1 = &vsr.voters[1];

    //
    // STEP 1: distribution and participants
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
        .transfer_token(payer_mint0_account, payer, vault, distribution_amount)
        .await;

    let accounts = send_tx(
        solana,
        CreateParticipantInstruction {
            distribution,
            voter: voter0.pubkey,
            payer,
        },
    )
    .await
    .unwrap();
    let participant0 = accounts.participant;

    let participant_data: Participant = solana.get_account(participant0).await;
    // -1 is due to rounding down as end_ts > now_ts
    let weight0 = voter0.locked_amount * 12 / 60 - 1;
    assert_eq!(participant_data.weight, weight0);

    let accounts = send_tx(
        solana,
        CreateParticipantInstruction {
            distribution,
            voter: voter1.pubkey,
            payer,
        },
    )
    .await
    .unwrap();
    let participant1 = accounts.participant;

    let participant_data: Participant = solana.get_account(participant1).await;
    let weight1 = voter1.locked_amount * 12 / 60 - 1;
    assert_eq!(participant_data.weight, weight1);

    //
    // STEP 1: go to claim phase
    //
    assert!(send_tx(solana, StartClaimPhaseInstruction { distribution })
        .await
        .is_err());

    send_tx(
        solana,
        SetTimeOffsetInstruction {
            distribution,
            admin,
            time_offset: 1000,
        },
    )
    .await
    .unwrap();

    solana.advance_by_slots(2).await;
    send_tx(solana, StartClaimPhaseInstruction { distribution })
        .await
        .unwrap();

    //
    // STEP 3: claim
    //
    let balance = solana.token_account_balance(payer_mint0_account).await;
    assert_eq!(balance, start_balance - distribution_amount);

    send_tx(
        solana,
        ClaimInstruction {
            participant: participant0,
            voter_authority: &voter0.authority,
            target_token: payer_mint0_account,
            sol_destination: payer.pubkey(),
        },
    )
    .await
    .unwrap();

    // participant account is closed
    assert!(solana.get_account_data(participant0).await.is_none());

    let balance = solana.token_account_balance(payer_mint0_account).await;
    assert_eq!(balance, start_balance - distribution_amount + distribution_amount * weight0 / (weight0 + weight1));

    send_tx(
        solana,
        ClaimInstruction {
            participant: participant1,
            voter_authority: &voter1.authority,
            target_token: payer_mint0_account,
            sol_destination: payer.pubkey(),
        },
    )
    .await
    .unwrap();

    // participant account is closed
    assert!(solana.get_account_data(participant1).await.is_none());

    // rounding down will happen for fractional amounts
    let vault_balance = solana.token_account_balance(vault).await;
    assert_eq!(vault_balance, distribution_amount - (distribution_amount * weight0) / (weight0 + weight1) - (distribution_amount * weight1) / (weight0 + weight1));
    let balance = solana.token_account_balance(payer_mint0_account).await;
    assert_eq!(balance, start_balance - vault_balance);

    Ok(())
}
