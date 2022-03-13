#![cfg(feature = "test-bpf")]

use program_test::*;
use solana_program_test::*;
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transport::TransportError,
};

use distribute_by_locked_vote_weight::events;
use distribute_by_locked_vote_weight::state::*;

mod program_test;

fn deserialize_event<T: anchor_lang::Event>(event: &str) -> Option<T> {
    let data = base64::decode(event).ok()?;
    if data.len() < 8 || data[0..8] != T::discriminator() {
        return None;
    }
    T::try_from_slice(&data[8..]).ok()
}

async fn get_info(solana: &SolanaCookie, distribution: Pubkey, voter: Pubkey) -> events::Info {
    solana.advance_by_slots(1).await;
    send_tx(
        solana,
        LogInfoInstruction {
            distribution,
            voter,
        },
    )
    .await
    .unwrap();
    let log = solana.program_log();
    deserialize_event::<distribute_by_locked_vote_weight::events::Info>(&log[1]).unwrap()
}

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

    // Check info before any participants are created
    let info_event = get_info(solana, distribution, voter0.pubkey).await;
    assert_eq!(info_event.participant_total_weight, 0);
    assert_eq!(info_event.distribution_amount, distribution_amount);
    assert!(!info_event.in_claim_phase);
    // -1 is due to rounding down as end_ts > now_ts
    let weight0 = voter0.locked_amount * 12 / 60 - 1;
    assert_eq!(info_event.usable_weight, Some(weight0));
    assert!(info_event.registered_weight.is_none());

    // Participant 0
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
    assert_eq!(participant_data.weight, weight0);

    // Participant 1
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

    // Check info after participants are created
    let info_event = get_info(solana, distribution, voter0.pubkey).await;
    assert_eq!(
        info_event.participant_total_weight,
        (weight0 + weight1) as u128
    );
    assert_eq!(info_event.distribution_amount, distribution_amount);
    assert!(!info_event.in_claim_phase);
    assert_eq!(info_event.usable_weight, Some(weight0));
    assert_eq!(info_event.registered_weight, Some(weight0));

    // claiming is impossible now
    assert!(send_tx(
        solana,
        ClaimInstruction {
            participant: participant0,
            voter_authority: &voter0.authority,
            target_token: payer_mint0_account,
            payer: payer.pubkey(),
        },
    )
    .await
    .is_err());

    //
    // STEP 1: advance time
    //
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

    // Check that it's reflected in info
    let info_event = get_info(solana, distribution, voter0.pubkey).await;
    assert!(info_event.in_claim_phase);
    assert_eq!(info_event.usable_weight, None);

    // updating is impossible now
    assert!(send_tx(
        solana,
        UpdateParticipantInstruction {
            participant: participant0
        }
    )
    .await
    .is_err());

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
            payer: payer.pubkey(),
        },
    )
    .await
    .unwrap();

    // participant account is closed
    assert!(solana.get_account_data(participant0).await.is_none());

    let balance = solana.token_account_balance(payer_mint0_account).await;
    assert_eq!(
        balance,
        start_balance - distribution_amount + distribution_amount * weight0 / (weight0 + weight1)
    );

    send_tx(
        solana,
        ClaimInstruction {
            participant: participant1,
            voter_authority: &voter1.authority,
            target_token: payer_mint0_account,
            payer: payer.pubkey(),
        },
    )
    .await
    .unwrap();

    // participant account is closed
    assert!(solana.get_account_data(participant1).await.is_none());

    // rounding down will happen for fractional amounts
    let vault_balance = solana.token_account_balance(vault).await;
    assert_eq!(
        vault_balance,
        distribution_amount
            - (distribution_amount * weight0) / (weight0 + weight1)
            - (distribution_amount * weight1) / (weight0 + weight1)
    );
    let balance = solana.token_account_balance(payer_mint0_account).await;
    assert_eq!(balance, start_balance - vault_balance);

    Ok(())
}
