use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount};
use solana_sdk::instruction;
use solana_sdk::signature::{Keypair, Signer};

use super::client::*;
use super::{TestConfig, MANGO_MINT_PK};
use voter_stake_registry::state::*;

#[allow(dead_code)]
pub struct VoterCookie {
    pub pubkey: Pubkey,
    pub authority: Keypair,
    pub locked_amount: u64,
}

#[allow(dead_code)]
pub struct VoterStakeRegistryCookie {
    pub registrar: Pubkey,
    pub voters: Vec<VoterCookie>,
}

#[allow(dead_code)]
pub fn setup_mock_registrar_and_voters(
    test_config: &mut TestConfig,
    now_ts: u64,
) -> VoterStakeRegistryCookie {
    let registrar = Pubkey::new_unique();
    {
        let mut registrar_data = Registrar::default();
        registrar_data.voting_mints[0] = VotingMintConfig {
            mint: *MANGO_MINT_PK,
            grant_authority: Pubkey::default(),
            baseline_vote_weight_scaled_factor: 1_000_000_000,
            max_extra_lockup_vote_weight_scaled_factor: 1_000_000_000,
            lockup_saturation_secs: 5 * 365 * 24 * 60 * 60,
            digit_shift: 0,
            reserved1: [0; 7],
            reserved2: [0; 7],
        };
        test_config.add_anchor_account(registrar, registrar_data, voter_stake_registry::id());
    }

    let mut voters = vec![];
    for locked_amount in [1000, 500] {
        let voter_authority = Keypair::new();
        let (voter, voter_bump) = Pubkey::find_program_address(
            &[
                registrar.as_ref(),
                b"voter".as_ref(),
                voter_authority.pubkey().as_ref(),
            ],
            &voter_stake_registry::id(),
        );

        let mut voter_data = Voter {
            voter_authority: voter_authority.pubkey(),
            registrar: registrar,
            deposits: [DepositEntry::default(); 32],
            voter_bump,
            voter_weight_record_bump: 0,
            reserved: [0; 94],
        };
        voter_data.deposits[0] = DepositEntry {
            lockup: Lockup::new_from_periods(
                LockupKind::Constant,
                now_ts as i64,
                now_ts as i64 - 1000,
                365,
            )
            .unwrap(),
            amount_deposited_native: locked_amount,
            amount_initially_locked_native: locked_amount,
            is_used: true,
            allow_clawback: false,
            voting_mint_config_idx: 0,
            reserved: [0; 29],
        };
        test_config.add_anchor_account(voter, voter_data, voter_stake_registry::id());

        let vault =
            spl_associated_token_account::get_associated_token_address(&voter, &MANGO_MINT_PK);
        let vault_account = spl_token::state::Account {
            mint: *MANGO_MINT_PK,
            owner: voter,
            amount: locked_amount,
            state: spl_token::state::AccountState::Initialized,
            ..spl_token::state::Account::default()
        };
        test_config.add_packable_account(vault, vault_account, spl_token::id());

        voters.push(VoterCookie {
            pubkey: voter,
            authority: voter_authority,
            locked_amount,
        });
    }

    VoterStakeRegistryCookie { registrar, voters }
}

pub struct DepositInstruction<'keypair> {
    pub deposit_entry_index: u8,
    pub amount: u64,

    pub voter: Pubkey,
    pub deposit_token: Pubkey,
    pub deposit_authority: &'keypair Keypair,
}
#[async_trait::async_trait(?Send)]
impl<'keypair> ClientInstruction for DepositInstruction<'keypair> {
    type Accounts = voter_stake_registry::accounts::Deposit;
    type Instruction = voter_stake_registry::instruction::Deposit;
    async fn to_instruction(
        &self,
        account_loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, instruction::Instruction) {
        let program_id = voter_stake_registry::id();
        let instruction = Self::Instruction {
            deposit_entry_index: self.deposit_entry_index,
            amount: self.amount,
        };

        let voter: Voter = account_loader.load(&self.voter).await.unwrap();
        let deposit_token: TokenAccount = account_loader.load(&self.deposit_token).await.unwrap();
        let vault = spl_associated_token_account::get_associated_token_address(
            &self.voter,
            &deposit_token.mint,
        );

        let accounts = Self::Accounts {
            registrar: voter.registrar,
            voter: self.voter,
            vault,
            deposit_token: self.deposit_token,
            deposit_authority: self.deposit_authority.pubkey(),
            token_program: Token::id(),
        };

        let instruction = make_instruction(program_id, &accounts, instruction);
        (accounts, instruction)
    }

    fn signers(&self) -> Vec<&Keypair> {
        vec![self.deposit_authority]
    }
}
