use std::cell::RefCell;
use std::sync::{Arc, RwLock};

use anchor_lang::AccountDeserialize;
use anchor_spl::token::TokenAccount;
use solana_program::{program_pack::Pack, rent::*, system_instruction};
use solana_program_test::*;
use solana_sdk::{
    account::ReadableAccount,
    instruction::Instruction,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use spl_token::*;

pub struct SolanaCookie {
    pub context: RefCell<ProgramTestContext>,
    pub rent: Rent,
    pub program_log: Arc<RwLock<Vec<String>>>,
}

impl SolanaCookie {
    #[allow(dead_code)]
    pub async fn process_transaction(
        &self,
        instructions: &[Instruction],
        signers: Option<&[&Keypair]>,
    ) -> Result<(), BanksClientError> {
        self.program_log.write().unwrap().clear();

        let mut context = self.context.borrow_mut();

        let mut transaction =
            Transaction::new_with_payer(&instructions, Some(&context.payer.pubkey()));

        let mut all_signers = vec![&context.payer];

        if let Some(signers) = signers {
            all_signers.extend_from_slice(signers);
        }

        // This fails when warping is involved - https://gitmemory.com/issue/solana-labs/solana/18201/868325078
        // let recent_blockhash = self.context.banks_client.get_recent_blockhash().await.unwrap();

        transaction.sign(&all_signers, context.last_blockhash);

        context
            .banks_client
            .process_transaction_with_commitment(
                transaction,
                solana_sdk::commitment_config::CommitmentLevel::Processed,
            )
            .await
    }

    pub async fn get_clock(&self) -> solana_program::clock::Clock {
        self.context
            .borrow_mut()
            .banks_client
            .get_sysvar::<solana_program::clock::Clock>()
            .await
            .unwrap()
    }

    #[allow(dead_code)]
    pub async fn advance_by_slots(&self, slots: u64) {
        let clock = self.get_clock().await;
        self.context
            .borrow_mut()
            .warp_to_slot(clock.slot + slots + 1)
            .unwrap();
    }

    #[allow(dead_code)]
    pub async fn create_token_account(&self, owner: &Pubkey, mint: Pubkey) -> Pubkey {
        let keypair = Keypair::new();
        let rent = self.rent.minimum_balance(spl_token::state::Account::LEN);

        let instructions = [
            system_instruction::create_account(
                &self.context.borrow().payer.pubkey(),
                &keypair.pubkey(),
                rent,
                spl_token::state::Account::LEN as u64,
                &spl_token::id(),
            ),
            spl_token::instruction::initialize_account(
                &spl_token::id(),
                &keypair.pubkey(),
                &mint,
                owner,
            )
            .unwrap(),
        ];

        self.process_transaction(&instructions, Some(&[&keypair]))
            .await
            .unwrap();
        return keypair.pubkey();
    }

    #[allow(dead_code)]
    pub async fn transfer_token(
        &self,
        source: Pubkey,
        authority: &Keypair,
        destination: Pubkey,
        amount: u64,
    ) {
        let instructions = [spl_token::instruction::transfer(
            &spl_token::id(),
            &source,
            &destination,
            &authority.pubkey(),
            &vec![],
            amount,
        )
        .unwrap()];

        self.process_transaction(&instructions, Some(&[authority]))
            .await
            .unwrap();
    }

    #[allow(dead_code)]
    pub async fn get_account_data(&self, address: Pubkey) -> Option<Vec<u8>> {
        Some(
            self.context
                .borrow_mut()
                .banks_client
                .get_account(address)
                .await
                .unwrap()?
                .data()
                .to_vec(),
        )
    }

    #[allow(dead_code)]
    pub async fn get_account_opt<T: AccountDeserialize>(&self, address: Pubkey) -> Option<T> {
        let data = self.get_account_data(address).await?;
        let mut data_slice: &[u8] = &data;
        AccountDeserialize::try_deserialize(&mut data_slice).ok()
    }

    #[allow(dead_code)]
    pub async fn get_account<T: AccountDeserialize>(&self, address: Pubkey) -> T {
        self.get_account_opt(address).await.unwrap()
    }

    #[allow(dead_code)]
    pub async fn token_account_balance(&self, address: Pubkey) -> u64 {
        self.get_account::<TokenAccount>(address).await.amount
    }

    #[allow(dead_code)]
    pub fn program_log(&self) -> Vec<String> {
        self.program_log.read().unwrap().clone()
    }
}
