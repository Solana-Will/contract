use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint,
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
    clock::{UnixTimestamp, Clock},
    sysvar::Sysvar,
};


#[derive(BorshSerialize, BorshDeserialize)]
pub struct InheritorInfo {
    pub name: String,
    pub pubkey: Pubkey,
    pub share: u16,  // Divide by 10000.
}

#[derive(BorshSerialize, BorshDeserialize)]
pub struct WillData {
    pub schema_version: u8,  // Extendable, once you have version 255 on a first byte, next byte should be version as well.
    pub withdraw_allowed_ts: UnixTimestamp,
    pub inheritors_names: Vec<String>,
    pub inheritors_pubkeys: Vec<String>,
    pub inheritors_shares: Vec<u16>,
    // pub coins_accounts: Vec<u16>,
    // pub coins_frozen_balances: Vec<u64>,
    // pub inherited_nfts: HashMap<Pubkey, Pubkey>,
    // pub frozen_balances: HashMap<Pubkey, u64>,
}

impl WillData {
    fn check_released(&self) -> Result<(), ProgramError> {
        let now = Clock::get()?.unix_timestamp;
        if self.withdraw_allowed_ts < now {
            return Ok(())
        }
        msg!("Contract will be released at {}, but it is only {} now", self.withdraw_allowed_ts, now);
        Err(ProgramError::Custom(1))
    }
    fn get_share(&self, inheritor: &Pubkey) -> (u64, u64, usize) {
        let mut total_shares = 0_u64;
        let mut inheritor_shares = 0_u64;
        let mut found_index = self.inheritors_shares.len();
        let pubkeystr = inheritor.to_string();
        for i in 0..self.inheritors_shares.len() as usize {
            total_shares += self.inheritors_shares[i] as u64;
            if self.inheritors_shares[i] > 0 && 
                    self.inheritors_pubkeys[i] == pubkeystr &&
                    found_index == self.inheritors_shares.len() {
                inheritor_shares += self.inheritors_shares[i] as u64;
                found_index = i;
            }
        }
        (inheritor_shares, total_shares, found_index)
    }
}


#[derive(BorshDeserialize)]
pub struct SetInheritenceMessage {
    pub selector: u8,
    pub inheritors_names: Vec<String>,
    pub inheritors_pubkeys: Vec<String>,
    pub inheritors_shares: Vec<u16>,
}

#[derive(BorshDeserialize)]
pub struct WithdrawSolMessage {
    pub selector: u8,
    pub lamports: u64,
}

// Declare and export the program's entrypoint
entrypoint!(process_instruction);

// Program entrypoint's implementation
pub fn process_instruction(
    program_id: &Pubkey, // Public key of the account the hello world program was loaded into
    accounts: &[AccountInfo], // The account to say hello to
    _instruction_data: &[u8],
) -> ProgramResult {
    msg!("Hello World Rust program entrypoint");

    // Iterating accounts is safer then indexing
    let accounts_iter = &mut accounts.iter();

    // Get the account to say hello to
    let sender = next_account_info(accounts_iter)?;
    let account = next_account_info(accounts_iter)?;
    
    // The account must be owned by the program in order to modify its data
    if account.owner != program_id {
        msg!("Greeted account {} (owner = {}) does not have the correct program id {}", account.key, account.owner, program_id);
        return Err(ProgramError::IncorrectProgramId);
    }

    let timeout: i64 = 5 * 60;
    match _instruction_data[0] {
        // 0 -> Modify inheritors.
        0 => {
            check_ownership(account.key, sender.key, program_id)?;

            let mut will_data = WillData::deserialize(&mut &account.data.borrow()[..])?;
            let msg = SetInheritenceMessage::deserialize(&mut &_instruction_data[..])?;
            will_data.schema_version = 1_u8;
            will_data.withdraw_allowed_ts = Clock::get()?.unix_timestamp + timeout;
            will_data.inheritors_names = msg.inheritors_names;
            will_data.inheritors_pubkeys = msg.inheritors_pubkeys;
            will_data.inheritors_shares = msg.inheritors_shares;
            will_data.serialize(&mut &mut account.data.borrow_mut()[..])?;
        },

        // 1 - withdraw own funds SOL
        1 => {
            check_ownership(account.key, sender.key, program_id)?;

            let msg = WithdrawSolMessage::deserialize(&mut &_instruction_data[..])?;
            **account.try_borrow_mut_lamports()? -= msg.lamports;
            **sender.try_borrow_mut_lamports()? += msg.lamports;

            let mut will_data = WillData::deserialize(&mut &account.data.borrow()[..])?;
            will_data.withdraw_allowed_ts = Clock::get()?.unix_timestamp + timeout;
            will_data.serialize(&mut &mut account.data.borrow_mut()[..])?;
        },

        // 2 - withdraw inheritance
        2 => {
            let mut will_data = WillData::deserialize(&mut &account.data.borrow()[..])?;
            will_data.check_released()?;

            let (inheritor_shares, total_shares, inheritor_index) = will_data.get_share(sender.key);
            if inheritor_shares == 0 {
                return Err(ProgramError::Custom(2))
            }

            let lamports_to_transfer = (**account.lamports.borrow()) / total_shares * inheritor_shares;
            **account.try_borrow_mut_lamports()? -= lamports_to_transfer;
            **sender.try_borrow_mut_lamports()? += lamports_to_transfer;
            will_data.inheritors_shares[inheritor_index] = 0;
            will_data.serialize(&mut &mut account.data.borrow_mut()[..])?;
        },

        3_u8..=u8::MAX => {}
    }
    //         if will_data.owner == Pubkey::new_from_array([0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0]) {
    //         // if account.data.borrow()[0] == 0 {
    //             msg!("Initializing...");
    //             will_data.owner = *sender.key;
    //             will_data.withdraw_allowed_ts = Clock::get()?.unix_timestamp;
    //             will_data.inheritor1 = Pubkey::new_from_array([0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0]);
    //             will_data.inheritor1_share = 33;
    //             will_data.inheritor2 = Pubkey::new_from_array([0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0]);
    //             will_data.inheritor2_share = 33;
    //             will_data.inheritor3 = Pubkey::new_from_array([0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0]);
    //             will_data.inheritor3_share = 34;
    //             // will_data.owner = *sender.key;
    //             will_data.serialize(&mut &mut account.data.borrow_mut()[..])?;
    //         } else {
    //             msg!("Trying to re-initialize account {}!", will_data.owner);
    //             return Err(ProgramError::AccountAlreadyInitialized);
    //         }
    //     },
    //     1 => {
    //         if will_data.owner != *sender.key {
    //             msg!("If you {} are not an owner {} you can not take back sol", *sender.key, will_data.owner);
    //             return Err(ProgramError::InvalidAccountData);
    //         }

    //         **account.try_borrow_mut_lamports()? -= 1000000000;
    //         **sender.try_borrow_mut_lamports()? += 1000000000;
    //     },
    //     2_u8..=u8::MAX => {}
    // }
    //
    // Increment and store the number of times the account has been greeted
    //
    // greeting_account.counter += 1;
    // greeting_account.serialize(&mut &mut account.data.borrow_mut()[..])?;
    
    //msg!("Greeted {} time(s)!", greeting_account.counter);

// }
    // account.data.borrow_mut()[0] = 0x1;

    Ok(())
}

fn check_ownership(account_key: &Pubkey, sender_key: &Pubkey, program_id: &Pubkey) -> Result<(), ProgramError> {
    let seed = "solana-will.com/my/v3/1";
    let expected_account = Pubkey::create_with_seed(sender_key, seed, program_id)?;
    if *account_key != expected_account {
        // msg!("Sender {} with seed {} should be {} But got {}", sender_key, seed, expected_account, account_key);
        msg!("Sender {} with seed {} should be {}", sender_key, seed, expected_account);
        msg!("But got {}", account_key);
        return Err(ProgramError::IncorrectProgramId);
    }
    Ok(())
}

// Sanity tests
#[cfg(test)]
mod test {
    use super::*;
    use solana_program::clock::Epoch;
    use std::mem;

    #[test]
    fn test_sanity() {
        let program_id = Pubkey::default();
        let key = Pubkey::default();
        let mut lamports = 0;
        let mut data = vec![0; mem::size_of::<u32>()];
        let owner = Pubkey::default();
        let account = AccountInfo::new(
            &key,
            false,
            true,
            &mut lamports,
            &mut data,
            &owner,
            false,
            Epoch::default(),
        );
        let instruction_data: Vec<u8> = Vec::new();

        let accounts = vec![account];

        assert_eq!(
            WillData::try_from_slice(&accounts[0].data.borrow())
                .unwrap()
                .counter,
            0
        );
        process_instruction(&program_id, &accounts, &instruction_data).unwrap();
        assert_eq!(
            WillData::try_from_slice(&accounts[0].data.borrow())
                .unwrap()
                .counter,
            1
        );
        process_instruction(&program_id, &accounts, &instruction_data).unwrap();
        assert_eq!(
            WillData::try_from_slice(&accounts[0].data.borrow())
                .unwrap()
                .counter,
            2
        );
    }
}
