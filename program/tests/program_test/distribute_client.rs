use anchor_lang::prelude::*;
use anchor_lang::solana_program::sysvar::{self, SysvarId};
use anchor_spl::token::Token;
use solana_sdk::instruction;
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::transport::TransportError;

use super::solana::SolanaCookie;
use distribute_by_locked_vote_weight::state::*;

#[async_trait::async_trait(?Send)]
pub trait ClientAccountLoader {
    async fn load_bytes(&self, pubkey: &Pubkey) -> Option<Vec<u8>>;
    async fn load<T: AccountDeserialize>(&self, pubkey: &Pubkey) -> Option<T> {
        let bytes = self.load_bytes(pubkey).await?;
        AccountDeserialize::try_deserialize(&mut &bytes[..]).ok()
    }
}

#[async_trait::async_trait(?Send)]
impl ClientAccountLoader for &SolanaCookie {
    async fn load_bytes(&self, pubkey: &Pubkey) -> Option<Vec<u8>> {
        self.get_account_data(*pubkey).await
    }
}

// TODO: report error outwards etc
pub async fn send_tx<CI: ClientInstruction>(
    solana: &SolanaCookie,
    ix: CI,
) -> std::result::Result<CI::Accounts, TransportError> {
    let (accounts, instruction) = ix.to_instruction(solana).await;
    let signers = ix.signers();
    let instructions = vec![instruction];
    solana
        .process_transaction(&instructions, Some(&signers[..]))
        .await?;
    Ok(accounts)
}

#[async_trait::async_trait(?Send)]
pub trait ClientInstruction {
    type Accounts: anchor_lang::ToAccountMetas;
    type Instruction: anchor_lang::InstructionData;

    async fn to_instruction(
        &self,
        loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, instruction::Instruction);
    fn signers(&self) -> Vec<&Keypair>;
}

fn make_instruction(
    program_id: Pubkey,
    accounts: &impl anchor_lang::ToAccountMetas,
    data: impl anchor_lang::InstructionData,
) -> instruction::Instruction {
    instruction::Instruction {
        program_id,
        accounts: anchor_lang::ToAccountMetas::to_account_metas(accounts, None),
        data: anchor_lang::InstructionData::data(&data),
    }
}

//
// a struct for each instruction along with its
// ClientInstruction impl
//

pub struct CreateDistributionInstruction<'keypair> {
    pub index: u64,
    pub end_ts: u64,
    pub weight_ts: u64,

    pub registrar: Pubkey,
    pub mint: Pubkey,
    pub admin: &'keypair Keypair,
    pub payer: &'keypair Keypair,
}
#[async_trait::async_trait(?Send)]
impl<'keypair> ClientInstruction for CreateDistributionInstruction<'keypair> {
    type Accounts = distribute_by_locked_vote_weight::accounts::CreateDistribution;
    type Instruction = distribute_by_locked_vote_weight::instruction::CreateDistribution;
    async fn to_instruction(
        &self,
        _account_loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, instruction::Instruction) {
        let program_id = distribute_by_locked_vote_weight::id();
        let instruction = Self::Instruction {
            index: self.index,
            end_ts: self.end_ts,
            weight_ts: self.weight_ts,
        };

        let distribution = Pubkey::find_program_address(
            &[
                b"distribution".as_ref(),
                self.admin.pubkey().as_ref(),
                &self.index.to_le_bytes(),
            ],
            &program_id,
        )
        .0;
        let vault =
            spl_associated_token_account::get_associated_token_address(&distribution, &self.mint);

        let accounts = Self::Accounts {
            distribution,
            admin: self.admin.pubkey(),
            payer: self.payer.pubkey(),
            registrar: self.registrar,
            mint: self.mint,
            vault,
            token_program: Token::id(),
            system_program: System::id(),
            associated_token_program: spl_associated_token_account::id(),
            rent: sysvar::rent::Rent::id(),
        };

        let instruction = make_instruction(program_id, &accounts, instruction);
        (accounts, instruction)
    }

    fn signers(&self) -> Vec<&Keypair> {
        vec![self.payer, self.admin]
    }
}

pub struct CreateParticipantInstruction<'keypair> {
    pub distribution: Pubkey,
    pub voter: Pubkey,
    pub payer: &'keypair Keypair,
}
#[async_trait::async_trait(?Send)]
impl<'keypair> ClientInstruction for CreateParticipantInstruction<'keypair> {
    type Accounts = distribute_by_locked_vote_weight::accounts::CreateParticipant;
    type Instruction = distribute_by_locked_vote_weight::instruction::CreateParticipant;
    async fn to_instruction(
        &self,
        account_loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, instruction::Instruction) {
        let program_id = distribute_by_locked_vote_weight::id();
        let instruction = Self::Instruction {};

        let participant = Pubkey::find_program_address(
            &[
                self.distribution.as_ref(),
                b"participant".as_ref(),
                self.voter.as_ref(),
            ],
            &program_id,
        )
        .0;
        let distribution: Distribution = account_loader.load(&self.distribution).await.unwrap();

        let accounts = Self::Accounts {
            distribution: self.distribution,
            participant,
            voter: self.voter,
            registrar: distribution.registrar,
            payer: self.payer.pubkey(),
            system_program: System::id(),
            rent: sysvar::rent::Rent::id(),
        };

        let instruction = make_instruction(program_id, &accounts, instruction);
        (accounts, instruction)
    }

    fn signers(&self) -> Vec<&Keypair> {
        vec![self.payer]
    }
}

pub struct UpdateParticipantInstruction {
    pub participant: Pubkey,
}
#[async_trait::async_trait(?Send)]
impl ClientInstruction for UpdateParticipantInstruction {
    type Accounts = distribute_by_locked_vote_weight::accounts::UpdateParticipant;
    type Instruction = distribute_by_locked_vote_weight::instruction::UpdateParticipant;
    async fn to_instruction(
        &self,
        account_loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, instruction::Instruction) {
        let program_id = distribute_by_locked_vote_weight::id();
        let instruction = Self::Instruction {};

        let participant: Participant = account_loader.load(&self.participant).await.unwrap();
        let distribution: Distribution = account_loader
            .load(&participant.distribution)
            .await
            .unwrap();

        let accounts = Self::Accounts {
            distribution: participant.distribution,
            participant: self.participant,
            voter: participant.voter,
            registrar: distribution.registrar,
        };

        let instruction = make_instruction(program_id, &accounts, instruction);
        (accounts, instruction)
    }

    fn signers(&self) -> Vec<&Keypair> {
        vec![]
    }
}

pub struct SetTimeOffsetInstruction<'keypair> {
    pub distribution: Pubkey,
    pub admin: &'keypair Keypair,
    pub time_offset: i64,
}
#[async_trait::async_trait(?Send)]
impl<'keypair> ClientInstruction for SetTimeOffsetInstruction<'keypair> {
    type Accounts = distribute_by_locked_vote_weight::accounts::SetTimeOffset;
    type Instruction = distribute_by_locked_vote_weight::instruction::SetTimeOffset;
    async fn to_instruction(
        &self,
        _account_loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, instruction::Instruction) {
        let program_id = distribute_by_locked_vote_weight::id();
        let instruction = Self::Instruction {
            time_offset: self.time_offset,
        };

        let accounts = Self::Accounts {
            distribution: self.distribution,
            admin: self.admin.pubkey(),
        };

        let instruction = make_instruction(program_id, &accounts, instruction);
        (accounts, instruction)
    }

    fn signers(&self) -> Vec<&Keypair> {
        vec![self.admin]
    }
}

pub struct StartClaimPhaseInstruction {
    pub distribution: Pubkey,
}
#[async_trait::async_trait(?Send)]
impl ClientInstruction for StartClaimPhaseInstruction {
    type Accounts = distribute_by_locked_vote_weight::accounts::StartClaimPhase;
    type Instruction = distribute_by_locked_vote_weight::instruction::StartClaimPhase;
    async fn to_instruction(
        &self,
        account_loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, instruction::Instruction) {
        let program_id = distribute_by_locked_vote_weight::id();
        let instruction = Self::Instruction {};

        let distribution: Distribution = account_loader.load(&self.distribution).await.unwrap();

        let accounts = Self::Accounts {
            distribution: self.distribution,
            vault: distribution.vault,
        };

        let instruction = make_instruction(program_id, &accounts, instruction);
        (accounts, instruction)
    }

    fn signers(&self) -> Vec<&Keypair> {
        vec![]
    }
}

pub struct ClaimInstruction<'keypair> {
    pub participant: Pubkey,
    pub voter_authority: &'keypair Keypair,
    pub target_token: Pubkey,
    pub sol_destination: Pubkey,
}
#[async_trait::async_trait(?Send)]
impl<'keypair> ClientInstruction for ClaimInstruction<'keypair> {
    type Accounts = distribute_by_locked_vote_weight::accounts::Claim;
    type Instruction = distribute_by_locked_vote_weight::instruction::Claim;
    async fn to_instruction(
        &self,
        account_loader: impl ClientAccountLoader + 'async_trait,
    ) -> (Self::Accounts, instruction::Instruction) {
        let program_id = distribute_by_locked_vote_weight::id();
        let instruction = Self::Instruction {};

        let participant: Participant = account_loader.load(&self.participant).await.unwrap();
        let distribution: Distribution = account_loader
            .load(&participant.distribution)
            .await
            .unwrap();

        let accounts = Self::Accounts {
            distribution: participant.distribution,
            participant: self.participant,
            vault: distribution.vault,
            target_token: self.target_token,
            voter_authority: self.voter_authority.pubkey(),
            sol_destination: self.sol_destination,
            token_program: Token::id(),
        };

        let instruction = make_instruction(program_id, &accounts, instruction);
        (accounts, instruction)
    }

    fn signers(&self) -> Vec<&Keypair> {
        vec![self.voter_authority]
    }
}
