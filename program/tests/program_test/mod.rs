use std::cell::RefCell;
use std::{sync::Arc, sync::RwLock};

use log::*;
use solana_program::{program_option::COption, program_pack::Pack};
use solana_program_test::*;
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};
use spl_token::{state::*, *};
use std::str::FromStr;

pub use client::*;
pub use cookies::*;
pub use distribute_client::*;
pub use solana::*;
pub use utils::*;

pub mod client;
pub mod cookies;
pub mod distribute_client;
pub mod solana;
pub mod utils;
pub mod vsr_client;

lazy_static::lazy_static! {
    pub static ref MANGO_MINT_PK: Pubkey = Pubkey::from_str("MangoCzJ36AjZyKwVj3VnYU4GTonjfVEnJmvvWaxLac").unwrap();
}

trait AddPacked {
    fn add_packable_account<T: Pack>(
        &mut self,
        pubkey: Pubkey,
        amount: u64,
        data: &T,
        owner: &Pubkey,
    );
}

impl AddPacked for ProgramTest {
    fn add_packable_account<T: Pack>(
        &mut self,
        pubkey: Pubkey,
        amount: u64,
        data: &T,
        owner: &Pubkey,
    ) {
        let mut account = solana_sdk::account::Account::new(amount, T::get_packed_len(), owner);
        data.pack_into_slice(&mut account.data);
        self.add_account(pubkey, account);
    }
}

struct LoggerWrapper {
    inner: env_logger::Logger,
    program_log: Arc<RwLock<Vec<String>>>,
}

impl Log for LoggerWrapper {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        self.inner.enabled(metadata)
    }

    fn log(&self, record: &log::Record) {
        if record
            .target()
            .starts_with("solana_runtime::message_processor")
        {
            let msg = record.args().to_string();
            if let Some(data) = msg.strip_prefix("Program log: ") {
                self.program_log.write().unwrap().push(data.into());
            }
        }
        self.inner.log(record);
    }

    fn flush(&self) {}
}

pub struct TestContext {
    pub solana: Arc<SolanaCookie>,
    pub mints: Vec<MintCookie>,
    pub users: Vec<UserCookie>,
    pub quote_index: usize,
}

#[derive(Default)]
pub struct TestConfig {
    pub accounts: Vec<(Pubkey, solana_sdk::account::Account)>,
}

impl TestConfig {
    pub fn add_packable_account<T: Pack>(&mut self, pubkey: Pubkey, data: T, owner: Pubkey) {
        let mut account =
            solana_sdk::account::Account::new(u32::MAX as u64, T::get_packed_len(), &owner);
        data.pack_into_slice(&mut account.data);
        self.accounts.push((pubkey, account));
    }

    pub fn add_anchor_account<T: bytemuck::Pod + anchor_lang::Discriminator>(
        &mut self,
        pubkey: Pubkey,
        data: T,
        owner: Pubkey,
    ) {
        let mut bytes = T::discriminator().to_vec();
        bytes.append(&mut bytemuck::bytes_of(&data).to_vec());
        let mut account = solana_sdk::account::Account::new(u32::MAX as u64, bytes.len(), &owner);
        account.data = bytes;
        self.accounts.push((pubkey, account));
    }
}

impl TestContext {
    pub async fn new(config: TestConfig) -> Self {
        // We need to intercept logs to capture program log output
        let log_filter = "solana_rbpf=trace,\
                    solana_runtime::message_processor=debug,\
                    solana_runtime::system_instruction_processor=trace,\
                    solana_program_test=info";
        let env_logger =
            env_logger::Builder::from_env(env_logger::Env::new().default_filter_or(log_filter))
                .format_timestamp_nanos()
                .build();
        let program_log_capture = Arc::new(RwLock::new(vec![]));
        let _ = log::set_boxed_logger(Box::new(LoggerWrapper {
            inner: env_logger,
            program_log: program_log_capture.clone(),
        }));

        let program_id = distribute_by_locked_vote_weight::id();

        let mut test = ProgramTest::new(
            "distribute_by_locked_vote_weight",
            program_id,
            processor!(distribute_by_locked_vote_weight::entry),
        );

        test.add_program("voter_stake_registry", voter_stake_registry::id(), None);

        // intentionally set to half the limit, to catch potential problems early
        test.set_compute_max_units(100000);

        // Setup the environment

        for (pubkey, account) in config.accounts {
            test.add_account(pubkey, account);
        }

        // Mints
        let mut mints: Vec<MintCookie> = vec![
            MintCookie {
                index: 0,
                decimals: 6,
                unit: 10u64.pow(6) as f64,
                base_lot: 100 as f64,
                quote_lot: 10 as f64,
                pubkey: *MANGO_MINT_PK,
                authority: Keypair::new(),
            }, // symbol: "MNGO".to_string()
            MintCookie {
                index: 0,
                decimals: 6,
                unit: 10u64.pow(6) as f64,
                base_lot: 100 as f64,
                quote_lot: 10 as f64,
                pubkey: Pubkey::default(),
                authority: Keypair::new(),
            },
            MintCookie {
                index: 1,
                decimals: 6,
                unit: 10u64.pow(6) as f64,
                base_lot: 0 as f64,
                quote_lot: 0 as f64,
                pubkey: Pubkey::default(),
                authority: Keypair::new(),
            }, // symbol: "USDC".to_string()
        ];
        // Add mints in loop
        for mint_index in 0..mints.len() {
            let mint_pk: Pubkey;
            if mints[mint_index].pubkey == Pubkey::default() {
                mint_pk = Pubkey::new_unique();
            } else {
                mint_pk = mints[mint_index].pubkey;
            }
            mints[mint_index].pubkey = mint_pk;

            test.add_packable_account(
                mint_pk,
                u32::MAX as u64,
                &Mint {
                    is_initialized: true,
                    mint_authority: COption::Some(mints[mint_index].authority.pubkey()),
                    decimals: mints[mint_index].decimals,
                    ..Mint::default()
                },
                &spl_token::id(),
            );
        }
        let quote_index = mints.len() - 1;

        // Users
        let num_users = 4;
        let mut users = Vec::new();
        for _ in 0..num_users {
            let user_key = Keypair::new();
            test.add_account(
                user_key.pubkey(),
                solana_sdk::account::Account::new(
                    u32::MAX as u64,
                    0,
                    &solana_sdk::system_program::id(),
                ),
            );

            // give every user 10^18 (< 2^60) of every token
            // ~~ 1 trillion in case of 6 decimals
            let mut token_accounts = Vec::new();
            for mint_index in 0..mints.len() {
                let token_key = Pubkey::new_unique();
                test.add_packable_account(
                    token_key,
                    u32::MAX as u64,
                    &spl_token::state::Account {
                        mint: mints[mint_index].pubkey,
                        owner: user_key.pubkey(),
                        amount: 1_000_000_000_000_000_000,
                        state: spl_token::state::AccountState::Initialized,
                        ..spl_token::state::Account::default()
                    },
                    &spl_token::id(),
                );

                token_accounts.push(token_key);
            }
            users.push(UserCookie {
                key: user_key,
                token_accounts,
            });
        }

        let mut context = test.start_with_context().await;
        let rent = context.banks_client.get_rent().await.unwrap();

        let solana = Arc::new(SolanaCookie {
            context: RefCell::new(context),
            rent,
            program_log: program_log_capture.clone(),
        });

        TestContext {
            solana: solana.clone(),
            mints,
            users,
            quote_index,
        }
    }
}
