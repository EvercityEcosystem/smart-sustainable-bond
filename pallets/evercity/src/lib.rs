#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{ 
			decl_module, decl_storage, decl_event, decl_error, 
			dispatch::{DispatchResult},
			traits::{Get},
			ensure,
			codec::{Encode, Decode, Compact, HasCompact, EncodeLike, WrapperTypeEncode},
			sp_runtime::{RuntimeDebug},
};
use frame_system::{ensure_signed};

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

use sp_core::H256;

pub trait Trait: frame_system::Trait {
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;
}


#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

pub const MASTER_ROLE_MASK: u8 = 1u8;
pub const CUSTODIAN_ROLE_MASK: u8 = 2u8;
pub const EMITENT_ROLE_MASK: u8 = 4u8;
pub const INVESTOR_ROLE_MASK: u8 = 8u8;
pub const MANAGER_ROLE_MASK: u8 = 16u8;
pub const AUDITOR_ROLE_MASK: u8 = 32u8;
pub const fn is_roles_correct(roles: u8) -> bool {
    if roles <= 63u8 { // max value of any roles combinations
        return true;
    }
    return false;
}

pub const EVERUSD_DECIMALS: u64 = 10;
pub const EVERUSD_MAX_MINT_AMOUNT: EverUSDBalance = 10000000000000; //1_000_000_000u64 * EVERUSD_DECIMALS;


/// Evercity project types
/// All these types must be put in CUSTOM_TYPES part of config for polkadot.js
/// to be correctly presented in DApp

pub type EverUSDBalance = u64;
//impl EncodeLike<u64> for EverUSDBalance {}

/// Structures, specific for each role

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, Clone, Default, RuntimeDebug)]                                                                       
pub struct EvercityAccountStruct {
    pub roles: u8,
    pub identity: u64,
} 
impl EncodeLike<(u8, u64)> for EvercityAccountStruct {}

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, Clone, Default, RuntimeDebug)]                                                                       
pub struct MintRequestStruct {
    pub amount: EverUSDBalance,
} 
impl EncodeLike<u64> for MintRequestStruct {}


decl_storage! {
    trait Store for Module<T: Trait> as Evercity {
        AccountRegistry
            get(fn account_registry)
            config(genesis_account_registry):
            map hasher(blake2_128_concat) T::AccountId => EvercityAccountStruct; //roles, identities, balances

        // Token balances storages. 
        // Evercity tokens cannot be transferred
        // Only mint/burn by Custodian accounts, invested/redeemed by Investor, paid by Emitent, etc...
        TotalSupplyEverUSD
            get(fn total_supply_everusd):
            EverUSDBalance; // total supply of EverUSD token (u64)
        
        BalanceEverUSD
            get(fn balances_everusd):
            map hasher(blake2_128_concat) T::AccountId => EverUSDBalance;

        MintRequestEverUSD
            get(fn mint_request_everusd):
                map hasher(blake2_128_concat) T::AccountId => MintRequestStruct;
    }
}


decl_event!(
    pub enum Event<T> where AccountId = <T as frame_system::Trait>::AccountId {
        /// Event documentation should end with an array that provides descriptive names for event
        /// parameters. [something, who]
        
        // [TODO] document events
        
        // 1: author, 2: newly added account
        AccountAdd(AccountId, AccountId, u8, u64), 
        
        // 1: author, 2:  updated account, 3: role, 4: identity
        AccountSet(AccountId, AccountId, u8, u64), 
        
        // 1: author, 2: disabled account, 3: role, 4: identity
        AccountDisable(AccountId, AccountId), 
        
        MintRequestCreated(AccountId, EverUSDBalance), 
        MintRequestRevoked(AccountId, EverUSDBalance), 
        MintRequestConfirmed(AccountId, EverUSDBalance), 
        MintRequestDeclined(AccountId, EverUSDBalance), 
    }
);


decl_error! {
    pub enum Error for Module<T: Trait> {
        NoneValue,
        
        /// Account was already added and present in mapping
        AccountToAddAlreadyExists,
        
        // [TODO] add parameters to errors
        
        /// Account not authorized
        AccountNotAuthorized,
        
        /// Account does not exist
        AccountNotExist,
        
        /// Account parameters are invalid
        AccountRoleParamIncorrect,

		/// Account already created one mint request, only one allowed at a time(to be changed in future)
		MintRequestAlreadyExist,

        /// Mint request for given account doesnt exist
		MintRequestDoesntExist,
		
        /// Incorrect parameters for mint request(miant amount > MAX_MINT_AMOUNT)
        MintRequestParamIncorrect,

    }
}


decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        // Errors must be initialized if they are used by the pallet.
        type Error = Error<T>;

        // Events must be initialized if they are used by the pallet.
        fn deposit_event() = default;


        /// Account management functions

		/// Method: account_disable(who: AccountId)
        /// Arguments: who: AccountId
        /// Access: Master role
        ///
        /// Disables access to platform. Disable all roles, account is not allowed to perform any actions
        /// but still have her data in blockchain (to not loose related entities)

        #[weight = 10_000]
        fn account_disable(origin, who: T::AccountId) -> DispatchResult {
            let _caller = ensure_signed(origin)?;
            ensure!(Self::account_is_master(&_caller), Error::<T>::AccountNotAuthorized);
            ensure!(AccountRegistry::<T>::contains_key(&who), Error::<T>::AccountNotExist);
            
            let mut _acc = AccountRegistry::<T>::get(&who);
            _acc.roles = 0u8; // set no roles

            AccountRegistry::<T>::insert(&who, _acc);

            Self::deposit_event(RawEvent::AccountDisable(_caller.clone(), who));
            Ok(())
        }

		/// Method: account_add_with_role_and_data(origin, who: T::AccountId, role: u8, identity: u64)
        /// Access: Master role
        ///
        /// Adds new master account
		/// Access: only accounts with Master role 
        #[weight = 10_000]
        fn account_add_with_role_and_data(origin, who: T::AccountId, role: u8, identity: u64) -> DispatchResult {
            let _caller = ensure_signed(origin)?;
            ensure!(Self::account_is_master(&_caller), Error::<T>::AccountNotAuthorized);
            ensure!(!AccountRegistry::<T>::contains_key(&who), Error::<T>::AccountToAddAlreadyExists);
            ensure!(is_roles_correct(role), Error::<T>::AccountRoleParamIncorrect);

            let _new_acc = EvercityAccountStruct { roles: role, identity: identity};
            AccountRegistry::<T>::insert(&who, _new_acc);

            Self::deposit_event(RawEvent::AccountAdd(_caller.clone(), who, role, identity));
            Ok(())
        }

		/// Method: account_set_with_role_and_data(origin, who: T::AccountId, role: u8, identity: u64)
        /// Arguments: who: AccountId, <account parameters(to be changed in future)>
        /// Access: Master role
        ///
        /// Adds new master account or modifies existing, adding Master role rights
		/// Access: only accounts with Master role 
        #[weight = 10_000]
        fn account_set_with_role_and_data(origin, who: T::AccountId, role: u8, identity: u64) -> DispatchResult {
            let _caller = ensure_signed(origin)?;
            ensure!(Self::account_is_master(&_caller), Error::<T>::AccountNotAuthorized);
            ensure!(AccountRegistry::<T>::contains_key(&who), Error::<T>::AccountNotExist);
            ensure!(is_roles_correct(role), Error::<T>::AccountRoleParamIncorrect);
            
            let _new_acc = EvercityAccountStruct {roles: role, identity: identity};
            AccountRegistry::<T>::insert(&who, _new_acc);

            Self::deposit_event(RawEvent::AccountSet(_caller.clone(), who, role, identity));
            Ok(())
        }

        /// Token balances manipulation functions

        /// Creates mint request to mint given amount of tokens on address of caller(emitent or investor)
        #[weight = 15_000]
        fn token_mint_request_create_everusd(origin, amount_to_mint: EverUSDBalance) -> DispatchResult {
            let _caller = ensure_signed(origin)?;
            ensure!(Self::account_is_emitent(&_caller) ||
					Self::account_is_investor(&_caller),
					Error::<T>::AccountNotAuthorized);
			ensure!(!MintRequestEverUSD::<T>::contains_key(&_caller), Error::<T>::MintRequestAlreadyExist);
			ensure!(amount_to_mint < EVERUSD_MAX_MINT_AMOUNT, Error::<T>::MintRequestParamIncorrect);

            let _new_mint_request = MintRequestStruct { amount: amount_to_mint };
            MintRequestEverUSD::<T>::insert(&_caller, _new_mint_request);

            Self::deposit_event(RawEvent::MintRequestCreated(_caller.clone(), amount_to_mint));
            Ok(())
        }

        #[weight = 5_000]
        fn token_mint_request_revoke_everusd(origin) -> DispatchResult {
            let _caller = ensure_signed(origin)?;
			ensure!(MintRequestEverUSD::<T>::contains_key(&_caller), Error::<T>::MintRequestDoesntExist);
            let _amount = MintRequestEverUSD::<T>::get(&_caller).amount;
            MintRequestEverUSD::<T>::remove(&_caller);
            Self::deposit_event(RawEvent::MintRequestRevoked(_caller.clone(), _amount));
            Ok(())
        }

        /// Token balances manipulation functions
        #[weight = 15_000]
        fn token_mint_request_confirm_everusd(origin, who: T::AccountId) -> DispatchResult {
            let _caller = ensure_signed(origin)?;
            ensure!(Self::account_is_custodian(&_caller),Error::<T>::AccountNotAuthorized);
			ensure!(MintRequestEverUSD::<T>::contains_key(&who), Error::<T>::MintRequestDoesntExist);
            let _mint_request = MintRequestEverUSD::<T>::get(&who);

            // add tokens to user's balance and total supply of EverUSD
            let _amount_to_add = _mint_request.clone().amount;
            let _new_everusd_balance = BalanceEverUSD::<T>::get(&who) + _amount_to_add.clone();
            BalanceEverUSD::<T>::insert(&who, _new_everusd_balance);
            let _total_supply = TotalSupplyEverUSD::get();
            TotalSupplyEverUSD::set(_total_supply +_amount_to_add.clone());

            MintRequestEverUSD::<T>::remove(&who);
            Self::deposit_event(RawEvent::MintRequestConfirmed(who.clone(), _amount_to_add.clone()));
            Ok(())
        }

        #[weight = 5_000]
        fn token_mint_request_decline_everusd(origin, who: T::AccountId) -> DispatchResult {
            let _caller = ensure_signed(origin)?;
            ensure!(Self::account_is_custodian(&_caller),Error::<T>::AccountNotAuthorized);
			ensure!(MintRequestEverUSD::<T>::contains_key(&who), Error::<T>::MintRequestDoesntExist);
            let _amount = MintRequestEverUSD::<T>::get(&who).amount;
            MintRequestEverUSD::<T>::remove(&who);
            Self::deposit_event(RawEvent::MintRequestDeclined(_caller.clone(), _amount));
            Ok(())
        }



    }
}

impl<T: Trait> Module<T> {

    pub fn account_is_master(_acc: &T::AccountId) -> bool {
        if 	AccountRegistry::<T>::contains_key(_acc) &&
            (AccountRegistry::<T>::get(_acc).roles & MASTER_ROLE_MASK != 0) {
            return true;
        }
        return false;
    }
    
    pub fn account_is_custodian(_acc: &T::AccountId) -> bool {
        if 	AccountRegistry::<T>::contains_key(_acc) &&
            (AccountRegistry::<T>::get(_acc).roles & CUSTODIAN_ROLE_MASK != 0) {
            return true;
        }
        return false;
    }
    
    pub fn account_is_emitent(_acc: &T::AccountId) -> bool {
        if 	AccountRegistry::<T>::contains_key(_acc) &&
            (AccountRegistry::<T>::get(_acc).roles & EMITENT_ROLE_MASK != 0) {
            return true;
        }
        return false;
    }

    pub fn account_is_investor(_acc: &T::AccountId) -> bool {
        if 	AccountRegistry::<T>::contains_key(_acc) &&
            (AccountRegistry::<T>::get(_acc).roles & INVESTOR_ROLE_MASK != 0) {
            return true;
        }
        return false;
    }




    pub fn balance_everusd(_acc: &T::AccountId) -> EverUSDBalance {
        return BalanceEverUSD::<T>::get(_acc); 
    }



}



