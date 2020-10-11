#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{ 
			decl_module, decl_storage, decl_event, decl_error, 
			dispatch::{DispatchResult},
			traits::{Get},
			ensure,
			codec::{Encode, Decode, Compact, HasCompact, EncodeLike},
			sp_runtime::{RuntimeDebug},
};
use frame_system::{ensure_signed};

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

use sp_core::H256;

pub trait Trait: frame_system::Trait {
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;
}

// TODO - move to enum
pub const MASTER_ROLE_MASK: u8 = 1u8;
pub const CUSTODIAN_ROLE_MASK: u8 = 2u8;
pub const EMITENT_ROLE_MASK: u8 = 4u8;
pub const INVESTOR_ROLE_MASK: u8 = 8u8;
pub const MANAGER_ROLE_MASK: u8 = 16u8;
pub const AUDITOR_ROLE_MASK: u8 = 32u8;


// #[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
// #[derive(Encode, Decode, Clone, Default, RuntimeDebug)]                                                                       
pub type MasterAccountTuple = (u64, u64);


decl_storage! {
    trait Store for Module<T: Trait> as Evercity {
        BalancesEverUSD get(fn accounts_map):
            map hasher(blake2_128_concat) T::AccountId => u128;
        AccountRegistry get(fn accounts) config(genesis_account_registry):
            map hasher(blake2_128_concat) T::AccountId => (u8, u64, u128); //roles, identities, balances

        // maps to store role-specific info about accounts. "bool" will be replaced to structs
        MasterAccount config(genesis_master_accounts):
            map hasher(blake2_128_concat) T::AccountId => MasterAccountTuple;
        CustodianAccount config(genesis_custodian_accounts):
            map hasher(blake2_128_concat) T::AccountId => bool;
    }
}


decl_event!(
    pub enum Event<T> where AccountId = <T as frame_system::Trait>::AccountId {
        /// Event documentation should end with an array that provides descriptive names for event
        /// parameters. [something, who]
        
        // [TODO] document events 
        AccountAddMaster(AccountId), // TODO, TEMP - add parameters
        AccountRemoveFromRegistry(AccountId), // TODO, TEMP - add parameters
    }
);


decl_error! {
    pub enum Error for Module<T: Trait> {
        NoneValue,
        
        /// Account was already added and present in mapping
        AccountToAddAlreadyExists,
        // [TODO] add parameters to errors
        AccountNotAuthorized,
        AccountNotExist,
    }
}


decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        // Errors must be initialized if they are used by the pallet.
        type Error = Error<T>;

        // Events must be initialized if they are used by the pallet.
        fn deposit_event() = default;

        /// Adds master account or modifies existing, adding Master role rights
		/// Access: only accounts with Master role 
        #[weight = 10_000]
        fn account_add_master(origin, who: T::AccountId, identity: u64) -> DispatchResult {
            let _caller = ensure_signed(origin)?;
            ensure!(Self::is_master(&_caller), Error::<T>::AccountNotAuthorized);

            // [TODO] append add
            ensure!(!AccountRegistry::<T>::contains_key(&who), Error::<T>::AccountToAddAlreadyExists);

            // [TODO] add tests

            AccountRegistry::<T>::insert(&who, (MASTER_ROLE_MASK, identity, 0u128));
            MasterAccount::<T>::insert(&who, (identity, identity));        
            Self::deposit_event(RawEvent::AccountAddMaster(who));
            Ok(())
        }

		


		/// Disables access to platform (all metadata still present in specific maps for account)
		/// Access: only accounts with Master role 
        #[weight = 10_000]
        fn account_remove_from_registry(origin, who: T::AccountId) -> DispatchResult {
            let _caller = ensure_signed(origin)?;
            ensure!(Self::is_master(&_caller), Error::<T>::AccountNotAuthorized);
            ensure!(AccountRegistry::<T>::contains_key(&who), Error::<T>::AccountNotExist);
            // [TODO] add tests
            AccountRegistry::<T>::remove(&who);

            Self::deposit_event(RawEvent::AccountRemoveFromRegistry(who));
            Ok(())
        }

    }
}

impl<T: Trait> Module<T> {
    pub fn is_master(_acc: &T::AccountId) -> bool {
        if 	AccountRegistry::<T>::contains_key(_acc) &&
            MasterAccount::<T>::contains_key(_acc) &&
            (AccountRegistry::<T>::get(_acc).0 & MASTER_ROLE_MASK != 0) {

            return true;
        }
        return false;
    }
}


#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;



