#![cfg_attr(not(feature = "std"), no_std)]
use frame_support::{
    decl_error, decl_event, decl_module, decl_storage,
    dispatch::DispatchResult,
    ensure,
    traits::{
        Currency, ExistenceRequirement, Get, LockIdentifier, LockableCurrency, WithdrawReason,
        WithdrawReasons,
    },
    weights::Weight,
};

#[cfg(test)]
mod tests;

use frame_system::ensure_signed;

const EVERCITY_LOCK_ID: LockIdentifier = *b"ever/fee";

type BalanceOf<T> =
    <<T as Trait>::Currency as Currency<<T as frame_system::Trait>::AccountId>>::Balance;

pub trait WeightInfo {
    fn transfer() -> Weight;
}

impl WeightInfo for () {
    fn transfer() -> Weight {
        10000 as Weight
    }
}

pub trait Trait: frame_system::Trait {
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;
    /// The currency in which fees are paid and contract balances are held.
    type Currency: LockableCurrency<Self::AccountId>;
    type WeightInfo: WeightInfo;
    /// The maximum value that can be transferred at once
    type MaximumTransferValue: Get<BalanceOf<Self>>;
}

decl_event!(
    pub enum Event<T>
    where
        AccountId = <T as frame_system::Trait>::AccountId,
        BalanceOf = BalanceOf<T>,
    {
        /// Account endowed. \[account, value\]
        Endow(AccountId, BalanceOf),
    }
);

decl_storage! {
    trait Store for Module<T: Trait> as EvercityTransfer {

    }
}

decl_error! {
    /// Error for the Transfer module
    pub enum Error for Module<T: Trait> {
        /// Attempt to transfer more than defined limit
        TransferRestriction,
    }
}

decl_module! {
    /// Transfer module declaration.
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        type Error = Error<T>;

        fn deposit_event() = default;

        #[weight = <T as Trait>::WeightInfo::transfer()]
        fn transfer(origin,  who: T::AccountId, value: BalanceOf<T>) -> DispatchResult {
            let sender = ensure_signed(origin)?;
            ensure!(value<= T::MaximumTransferValue::get(), Error::<T>::TransferRestriction);

            T::Currency::transfer(&sender, &who, value, ExistenceRequirement::AllowDeath )?;

            T::Currency::extend_lock(EVERCITY_LOCK_ID, &who, value, WithdrawReasons::except(WithdrawReason::TransactionPayment) );
            Self::deposit_event(RawEvent::Endow(who, value));
            Ok(())
        }
    }
}
