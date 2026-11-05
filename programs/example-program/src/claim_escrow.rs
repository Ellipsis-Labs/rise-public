use phoenix_rise::ix;
use phoenix_rise::ix::constants::PHOENIX_PROGRAM_ID;
use pinocchio::account_info::AccountInfo;
use pinocchio::instruction::Signer;
use pinocchio::program_error::ProgramError;
use pinocchio::pubkey::find_program_address;
use pinocchio::{ProgramResult, msg, seeds};

use crate::common::dynamic_tail;
use crate::cpi::{MAX_CPI_ACCOUNTS, check_program_id};
use crate::params::ClaimEscrowParams;

const CLAIMER_SEED: &[u8] = b"escrow_claimer";

pub(crate) fn process(accounts: &[AccountInfo], params: &ClaimEscrowParams) -> ProgramResult {
    let context = ClaimEscrowContext::load(
        accounts,
        params.global_trader_index_count as usize,
        params.active_trader_buffer_count as usize,
    )?;
    context.invoke()
}

struct ClaimEscrowContext<'a> {
    phoenix_program: &'a AccountInfo,
    log_authority: &'a AccountInfo,
    global_config: &'a AccountInfo,
    sender_wallet: &'a AccountInfo,
    sender_trader_account: &'a AccountInfo,
    claimer: &'a AccountInfo,
    receiver_trader_account: &'a AccountInfo,
    receiver_escrow: &'a AccountInfo,
    perp_asset_map: &'a AccountInfo,
    global_trader_index: &'a [AccountInfo],
    active_trader_buffer: &'a [AccountInfo],
}

impl<'a> ClaimEscrowContext<'a> {
    const FIXED_ACCOUNT_COUNT: usize = 9;

    fn load(
        accounts: &'a [AccountInfo],
        global_trader_index_count: usize,
        active_trader_buffer_count: usize,
    ) -> Result<Self, ProgramError> {
        let (global_trader_index, active_trader_buffer) = dynamic_tail(
            accounts,
            Self::FIXED_ACCOUNT_COUNT,
            global_trader_index_count,
            active_trader_buffer_count,
        )?;
        let context = Self {
            phoenix_program: &accounts[0],
            log_authority: &accounts[1],
            global_config: &accounts[2],
            sender_wallet: &accounts[3],
            sender_trader_account: &accounts[4],
            claimer: &accounts[5],
            receiver_trader_account: &accounts[6],
            receiver_escrow: &accounts[7],
            perp_asset_map: &accounts[8],
            global_trader_index,
            active_trader_buffer,
        };
        context.validate()?;
        Ok(context)
    }

    fn validate(&self) -> ProgramResult {
        check_program_id(self.phoenix_program, &PHOENIX_PROGRAM_ID, "Phoenix")
    }

    fn claimer_bump(&self) -> Result<u8, ProgramError> {
        let (claimer, bump) = find_program_address(
            &[CLAIMER_SEED, self.receiver_trader_account.key()],
            &crate::ID,
        );
        if &claimer != self.claimer.key() {
            msg!("rise-example-program: claimer is not the receiver's escrow claimer PDA");
            return Err(ProgramError::InvalidSeeds);
        }
        Ok(bump)
    }

    fn invoke(&self) -> ProgramResult {
        let bump = [self.claimer_bump()?];
        msg!("rise-example-program: Phoenix accept escrow request");
        let accept = ix::cpi::phoenix::AcceptEscrowRequest {
            phoenix_program: self.phoenix_program,
            log_authority: self.log_authority,
            global_config: self.global_config,
            sender_wallet: self.sender_wallet,
            sender_trader_account: self.sender_trader_account,
            receiver_wallet: self.claimer,
            receiver_trader_account: self.receiver_trader_account,
            receiver_escrow: self.receiver_escrow,
            perp_asset_map: self.perp_asset_map,
            global_trader_index: self.global_trader_index,
            active_trader_buffer: self.active_trader_buffer,
        };
        let seeds = seeds!(CLAIMER_SEED, self.receiver_trader_account.key(), &bump);
        let mut scratch = ix::cpi::CpiScratch::<
            { MAX_CPI_ACCOUNTS },
            { ix::cpi::phoenix::AcceptEscrowRequest::DATA_LEN },
        >::new(self.phoenix_program);
        accept.invoke_signed(
            ix::cpi::phoenix::AcceptEscrowRequestArgs,
            &[Signer::from(&seeds)],
            &mut scratch,
        )
    }
}
