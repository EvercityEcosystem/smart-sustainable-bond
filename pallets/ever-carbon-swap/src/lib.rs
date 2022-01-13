#![allow(clippy::unused_unit)]
#![cfg_attr(not(feature = "std"), no_std)]

mod exchange;
mod everusdasset;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod mock;

use frame_system::RawOrigin;
use crate::sp_api_hidden_includes_decl_storage::hidden_include::traits::Get;
use frame_support::{
    ensure,
    decl_error, 
    decl_module, 
    decl_storage,
    decl_event,
    dispatch::{
        DispatchResult,
        Vec,
    },
    traits::UnfilteredDispatchable,
};
use frame_system::{
    ensure_signed,
};
use sp_runtime::traits::StaticLookup;
use frame_support::sp_std::{
    cmp::{
        Eq, 
        PartialEq}, 
};
use pallet_evercity_assets as pallet_assets;
use exchange::{ExchangeStruct, HolderType};
use pallet_evercity::{EverUSDBalance};
use everusdasset::{EverUSDAssetMinRequest};

type AssetId<T> = <T as pallet_assets::Config>::AssetId;
type ExchangeId = u128;
type EverUSDAssetMintRequestId = u128;

pub trait Config: 
    frame_system::Config + 
    // pallet_evercity_accounts::Config + 
    pallet_timestamp::Config + 
    pallet_assets::Config + 
    pallet_evercity_carbon_credits::Config + 
    pallet_evercity::Config + 
{
        type Event: From<Event<Self>> + Into<<Self as frame_system::Config>::Event>;
}

decl_storage! {
    trait Store for Module<T: Config> as CarbonCredits {
        /// Main storage for exchanges
        ExchangeById
            get(fn exchange_by_id):
            map hasher(blake2_128_concat) ExchangeId => Option<ExchangeStruct<T::AccountId, AssetId<T>, T::Balance, EverUSDBalance>>;    
        LastID: ExchangeId;

        /// EverAssetMintRequestById
        EverUsdAssetMintRequestById
            get(fn ever_asset_mint_request_by_id):
            map hasher(blake2_128_concat) EverUSDAssetMintRequestId => Option<EverUSDAssetMinRequest<T::AccountId, AssetId<T>, T::Balance>>;
        LastMintID: EverUSDAssetMintRequestId;
    }
}

// Pallet events
decl_event!(
    pub enum Event<T>
    where
        AccountId = <T as frame_system::Config>::AccountId,
        AssetId = <T as pallet_assets::Config>::AssetId,
        Balance = <T as pallet_assets::Config>::Balance,
    {
        /// \[EverUSDHolder, CarbonCreditsHolder, AssetId, Balance\]
        EchangeCreated(AccountId, AccountId, AssetId, Balance),
    }
);

decl_error! {
    pub enum Error for Module<T: Config> {
        /// Account does not have an auditor role in Accounts Pallet
        AccountNotTokenOwner,
        BadHolder,
        ExchangeIdOwerflow,
        InsufficientEverUSDBalance,
        InsufficientCarbonCreditsBalance,
        ExchangeNotFound,
        BadApprove,
        AssetNotFound
    }
}

decl_module! {
    pub struct Module<T: Config> for enum Call where origin: T::Origin {
        type Error = Error<T>;
        // fn deposit_event() = default;
        #[weight = 10_000 + T::DbWeight::get().reads_writes(2, 2)]
        pub fn create_exhange(origin, 
            partner_holder: T::AccountId,
            ever_usd_count: EverUSDBalance,
            carbon_credits_asset_id: AssetId<T>,
            carbon_credits_count: T::Balance,
            holder_type: HolderType,
        ) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            // let mut new_exchange = ExchangeStruct::new(ever_usd_holder, carbon_credits_holder, ever_usd_count, carbon_credits_asset_id, carbon_credits_count);
            // ensure!();
            let asset_opt = pallet_assets::Module::<T>::get_asset_details(carbon_credits_asset_id);
            ensure!(asset_opt.is_some(), Error::<T>::AssetNotFound);
            
            let new_exchange = match holder_type {
                HolderType::EverUSDHolder => {

                    // Check EverUSD balance HERE!!!!!
                    let current_everusd_balance = pallet_evercity::Module::<T>::get_balance(&caller);
                    if ever_usd_count > current_everusd_balance {
                        return Err(Error::<T>::InsufficientEverUSDBalance.into());
                    } 

                    ExchangeStruct::new(caller, partner_holder, ever_usd_count, carbon_credits_asset_id, carbon_credits_count, exchange::EVERUSD_HOLDER_APPROVED)
                },
                HolderType::CarbonCreditsHolder => {
                    let current_carbon_credits_balace = pallet_evercity_assets::Module::<T>::balance(carbon_credits_asset_id, caller.clone());
                    if carbon_credits_count > current_carbon_credits_balace {
                        return Err(Error::<T>::InsufficientCarbonCreditsBalance.into());
                    }

                    ExchangeStruct::new(partner_holder, caller, ever_usd_count, carbon_credits_asset_id, carbon_credits_count, exchange::CARBON_CREDITS_HOLDER_APPROVED)
                },
            };

            let new_id = match LastID::get().checked_add(1) {
                Some(id) => id,
                None => return Err(Error::<T>::ExchangeIdOwerflow.into()),
            };
            ExchangeById::<T>::insert(new_id, new_exchange);
            LastID::mutate(|x| *x = new_id);

            Ok(())
        }

        #[weight = 10_000 + T::DbWeight::get().reads_writes(5, 3)]
        pub fn approve_exchange(origin, exchange_id: ExchangeId, holder_type: HolderType) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            ExchangeById::<T>::try_mutate(
                exchange_id, |project_to_mutate| -> DispatchResult {
                    match project_to_mutate  {
                        None => return Err(Error::<T>::ExchangeNotFound.into()),
                        Some(exchange) => {
                            match holder_type {
                                HolderType::EverUSDHolder => {
                                    ensure!(exchange.approved == exchange::CARBON_CREDITS_HOLDER_APPROVED, Error::<T>::BadApprove);
                                    ensure!(caller == exchange.ever_usd_holder, Error::<T>::BadHolder);

                                },
                                HolderType::CarbonCreditsHolder => {
                                    ensure!(exchange.approved == exchange::EVERUSD_HOLDER_APPROVED, Error::<T>::BadApprove);
                                    ensure!(caller == exchange.carbon_credits_holder, Error::<T>::BadHolder);
                                },
                            }

                            let current_everusd_balance = pallet_evercity::Module::<T>::get_balance(&exchange.ever_usd_holder);
                            let carbon_credits_balance = pallet_evercity_assets::Module::<T>::balance(exchange.carbon_credits_asset_id, exchange.carbon_credits_holder.clone());

                            if exchange.ever_usd_count > current_everusd_balance {
                                return Err(Error::<T>::InsufficientEverUSDBalance.into());
                            }
                            if exchange.carbon_credits_count > carbon_credits_balance  {
                                return Err(Error::<T>::InsufficientCarbonCreditsBalance.into());
                            }

                            // transfer carbon credits
                            let cc_holder_origin = frame_system::RawOrigin::Signed(exchange.carbon_credits_holder.clone()).into();
                            pallet_evercity_carbon_credits::Module::<T>::transfer_carbon_credits(
                                    cc_holder_origin, 
                                    exchange.carbon_credits_asset_id, 
                                    exchange.ever_usd_holder.clone(), 
                                    exchange.carbon_credits_count
                            )?;

                            // transfer everusd then
                            pallet_evercity::Module::<T>::transfer_everusd(&exchange.ever_usd_holder, &exchange.carbon_credits_holder, exchange.ever_usd_count)?;
                        }
                    }
                    Ok(())
                })?;

            Ok(())
        }

        #[weight = 10_000 + T::DbWeight::get().reads_writes(2, 2)]
        pub fn swap_everusd_bond_asset(origin, ever_usd_balance: EverUSDBalance, asset_balance: T::Balance, asset_id: AssetId<T>) -> DispatchResult {
            let caller = ensure_signed(origin)?;

            ensure!(pallet_evercity::Module::<T>::account_is_custodian(&caller), Error::<T>::InsufficientCarbonCreditsBalance);

            Ok(())
        }

        #[weight = 10_000 + T::DbWeight::get().reads_writes(2, 2)]
        pub fn create_everusd_asset_mint_reques(origin, asset_id: AssetId<T>, amount: T::Balance) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            // let 
            let mint_request = everusdasset::EverUSDAssetMinRequest::new(caller, asset_id, amount);
            

            Ok(())
        }
    }
}