#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Encode, Decode, Compact, HasCompact};
use frame_support::{decl_module, decl_storage, decl_event, decl_error, dispatch, traits::Get, ensure};
use frame_system::{self as system, ensure_signed}; 
use frame_support::dispatch::DispatchResult;

pub const MASTER_ROLE_MASK: u8 = 1u8;
pub const EMITENT_ROLE_MASK: u8 = 2u8;
pub const CUSTODIAN_ROLE_MASK: u8 = 4u8;
/*
1u8,    // Master
2u8,    // Emitent
4u8,    // Custodian
8u8,    // Investor
16u8,   // Manager
32u8,   // Auditor
*/

/*
#[derive(Debug, PartialEq, Encode, Decode, Default)]
pub struct EvercityAccountInfo {
    pub roles_mask: u8,
    pub nickname: u64,
}
*/

pub trait Trait: frame_system::Trait {
	type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;
}


decl_storage! {
	trait Store for Module<T: Trait> as EvercityModule {
        Balances get(fn accounts_map):
            map hasher(blake2_128_concat) T::AccountId => u128;
        Account get(fn accounts):
            map hasher(blake2_128_concat) T::AccountId => (u8, u8); //roles, flags

        // maps to store role-specific info about accounts. "bool" will be replaced to structs
        MasterAccount:
            map hasher(blake2_128_concat) T::AccountId => bool;
        EmitentAccount:
            map hasher(blake2_128_concat) T::AccountId => bool;
        CustodianAccount:
            map hasher(blake2_128_concat) T::AccountId => bool;
	}
}


decl_event!(
	pub enum Event<T> where AccountId = <T as frame_system::Trait>::AccountId {
		/// Event documentation should end with an array that provides descriptive names for event
		/// parameters. [something, who]
        
        // [TODO] document events 
		AccountAddWithRoleAndFlags(AccountId, u8, u8),
	}
);


decl_error! {
	pub enum Error for Module<T: Trait> {
		NoneValue,
        
        /// Account was already added and present in mapping
		AccountToAddAlreadyExists,
	}
}


decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		// Errors must be initialized if they are used by the pallet.
		type Error = Error<T>;

		// Events must be initialized if they are used by the pallet.
		fn deposit_event() = default;

        // writer functions here
   		#[weight = 10_000]
        fn add_account_with_role(origin, who: T::AccountId, role: u8, flags: u8) -> DispatchResult {
            // let _caller_account_id = ensure_signed(origin)?;
            ensure!(!Account::<T>::contains_key(&who), Error::<T>::AccountToAddAlreadyExists);

            // [TODO] add tests

            Account::<T>::insert(&who, (role, flags));
            if (role & MASTER_ROLE_MASK) != 0 {
                MasterAccount::<T>::insert(&who, true);        
            }
            if (role & EMITENT_ROLE_MASK) != 0 {
                EmitentAccount::<T>::insert(&who, true);        
            }
            if (role & CUSTODIAN_ROLE_MASK) != 0 {
                CustodianAccount::<T>::insert(&who, true);        
            }

            Self::deposit_event(RawEvent::AccountAddWithRoleAndFlags(who, role, flags));
            Ok(())
        }

   /*
		#[weight = 10_000]
        fn set_master_role_to_account(who: AccountId) -> DispatchResult {
            let _ = ensure_signed(origin)?;
            // let mut acc = Self::account(who).ok_or(Error::<T>::AccountNotExist)?;
            Self::deposit_event(RawEvent::AccountSetRole(account_id, MASTER_ROLE_MASK));
            Ok(())
        }
        */
    }
}

impl<T: Trait> Module<T> {
    // reader functions here
}


#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;



