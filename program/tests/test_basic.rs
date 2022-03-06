#![cfg(feature = "test-bpf")]

//use distribute_by_locked_vote_weight::state::*;
use program_test::*;
use solana_program_test::*;
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transport::TransportError,
};
use voter_stake_registry::state as vsr;

mod program_test;

fn to_vsr_account<T: bytemuck::Pod + anchor_lang::Discriminator>(
    data: T,
) -> solana_sdk::account::Account {
    let mut bytes = T::discriminator().to_vec();
    bytes.append(&mut bytemuck::bytes_of(&data).to_vec());
    let mut account = solana_sdk::account::Account::new(
        u32::MAX as u64,
        bytes.len(),
        &voter_stake_registry::id(),
    );
    account.data = bytes;
    account
}

// This is an unspecific happy-case test that just runs a few instructions to check
// that they work in principle. It should be split up / renamed.
#[tokio::test]
async fn test_basic() -> Result<(), TransportError> {
    //
    // SETUP: fake registrar / voter accounts
    //
    let mut test_config = TestConfig::default();

    let registrar = Pubkey::new_unique();
    {
        let mut registrar_data = vsr::Registrar::default();
        registrar_data.voting_mints[0] = vsr::VotingMintConfig {
            mint: *MANGO_MINT_PK,
            grant_authority: Pubkey::default(),
            baseline_vote_weight_scaled_factor: 1_000_000_000,
            max_extra_lockup_vote_weight_scaled_factor: 1_000_000_000,
            lockup_saturation_secs: 365 * 24 * 60 * 60,
            digit_shift: 0,
            reserved1: [0; 7],
            reserved2: [0; 7],
        };
        test_config
            .accounts
            .push((registrar, to_vsr_account(registrar_data)));
    }

    let voter_authority = &Keypair::new();
    let voter = Pubkey::new_unique();
    let now_ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    {
        let mut voter_data = vsr::Voter {
            voter_authority: voter_authority.pubkey(),
            registrar: registrar,
            deposits: [vsr::DepositEntry::default(); 32],
            voter_bump: 0,
            voter_weight_record_bump: 0,
            reserved: [0; 94],
        };
        voter_data.deposits[0] = vsr::DepositEntry {
            lockup: vsr::Lockup::new_from_periods(
                vsr::LockupKind::Constant,
                now_ts as i64,
                now_ts as i64 - 1000,
                100,
            )
            .unwrap(),
            amount_deposited_native: 1000,
            amount_initially_locked_native: 1000,
            is_used: true,
            allow_clawback: false,
            voting_mint_config_idx: 0,
            reserved: [0; 29],
        };
        test_config
            .accounts
            .push((voter, to_vsr_account(voter_data)));
    }

    //
    // SETUP: Start
    //
    let context = TestContext::new(test_config).await;
    let solana = &context.solana.clone();

    let admin = &Keypair::new();
    let payer = &context.users[0].key;
    let mint0 = &context.mints[0];
    let payer_mint0_account = context.users[0].token_accounts[0];

    //
    // SETUP: distribution and participant
    //
    let accounts = send_tx(
        solana,
        CreateDistributionInstruction {
            index: 0,
            end_ts: now_ts + 100,
            weight_ts: now_ts + 100,
            registrar,
            mint: mint0.pubkey,
            admin,
            payer,
        },
    )
    .await
    .unwrap();
    let distribution = accounts.distribution;

    let accounts = send_tx(
        solana,
        CreateParticipantInstruction {
            distribution,
            voter,
            voter_authority,
            payer,
        },
    )
    .await
    .unwrap();
    let participant = accounts.participant;

    Ok(())
}
