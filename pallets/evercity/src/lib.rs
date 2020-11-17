#![cfg_attr(not(feature = "std"), no_std)]
use account::{
    is_roles_correct, EvercityAccountStructOf, EvercityAccountStructT, TokenBurnRequestStruct,
    TokenBurnRequestStructOf, TokenMintRequestStruct, TokenMintRequestStructOf, AUDITOR_ROLE_MASK,
    CUSTODIAN_ROLE_MASK, IMPACT_REPORTER_ROLE_MASK, INVESTOR_ROLE_MASK, ISSUER_ROLE_MASK,
    MANAGER_ROLE_MASK, MASTER_ROLE_MASK,
};
pub use bond::{
    period::PeriodYield, BondId, BondImpactReportStruct, BondStruct, BondStructOf, BondUnitPackage,
};
use bond::{
    AccountYield, BondInnerStructOf, BondPeriod, BondPeriodNumber, BondState, BondUnitAmount,
    BondUnitSaleLotStructOf,
};
use core::cmp::{Eq, PartialEq};
use frame_support::{
    debug, decl_error, decl_event, decl_module, decl_storage,
    dispatch::Vec,
    dispatch::{Decode, DispatchError, DispatchResult},
    ensure,
};

use frame_system::ensure_signed;
use sp_core::sp_std::cmp::min;

pub trait Trait: frame_system::Trait + pallet_timestamp::Trait {
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;
}

pub trait Expired<Moment> {
    fn is_expired(&self, now: Moment) -> bool;
}
pub type EverUSDBalance = u64;
pub type Result<T> = core::result::Result<T, DispatchError>;

pub const EVERUSD_DECIMALS: u64 = 9; // EverUSD = USD * ( 10 ^ EVERUSD_DECIMALS )
pub const EVERUSD_MAX_MINT_AMOUNT: EverUSDBalance = 60_000_000_000_000_000; // =60 million dollar
const DAY_DURATION: u32 = 86400; // seconds in 1 DAY
pub const MIN_PAYMENT_PERIOD: BondPeriod = DAY_DURATION * 7;

const TOKEN_BURN_REQUEST_TTL: u32 = DAY_DURATION as u32 * 7 * 1000;
const TOKEN_MINT_REQUEST_TTL: u32 = DAY_DURATION as u32 * 7 * 1000;
const INTEREST_RATE_YEAR: u64 = 365;
const MAX_PURGE_REQUESTS: usize = 100;
const MIN_BOND_DURATION: u32 = 1; // 1  is a minimal bond period

pub mod account;
/// Evercity project types
/// All these types must be put in CUSTOM_TYPES part of config for polkadot.js
/// to be correctly presented in DApp
pub mod bond;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

macro_rules! ensure_active {
    ($f:expr, $err:expr) => {
        match ($f) {
            Some(v) => v,
            None => {
                return $err.into();
            }
        }
    };
}

sp_api::decl_runtime_apis! {
    pub trait BondApi<AccountId:Decode, Moment:Decode, Hash:Decode> {
        fn get_bond(bond: BondId) -> BondStruct<AccountId, Moment, Hash>;
        fn get_bond_yield(bond: BondId)-> Vec<PeriodYield>;
        fn get_impact_reports(bond: BondId)->Vec<BondImpactReportStruct>;
    }
}

decl_storage! {
    trait Store for Module<T: Trait> as Evercity {
        /// Storage map for accounts, their roles and corresponding info
        AccountRegistry
            get(fn account_registry)
            config(genesis_account_registry):
            map hasher(blake2_128_concat) T::AccountId => EvercityAccountStructOf<T>; //roles, identities, balances

        /// Total supply of EverUSD token. Sum of all token balances in system
        TotalSupplyEverUSD
            get(fn total_supply_everusd):
            EverUSDBalance; // total supply of EverUSD token (u64)

        /// Storage map for EverUSD token balances
        BalanceEverUSD
            get(fn balances_everusd):
            map hasher(blake2_128_concat) T::AccountId => EverUSDBalance;

        /// Storage map for EverUSD token mint requests (see TokenMintRequestStruct)
        MintRequestEverUSD
            get(fn mint_request_everusd):
                map hasher(blake2_128_concat) T::AccountId => TokenMintRequestStructOf<T>;

        /// Storage map for EverUSD token burn requests (see TokenBurnRequestStruct)
        BurnRequestEverUSD
            get(fn burn_request_everusd):
                map hasher(blake2_128_concat) T::AccountId => TokenBurnRequestStructOf<T>;

        /// Structure for storing all platform bonds.
        /// BondId is now a ticker [u8; 8]: 8-bytes unique identifier like "MUSKPWR1" or "WINDGEN2"
        BondRegistry
            get(fn bond_registry):
                map hasher(blake2_128_concat) BondId => BondStructOf<T>;

        /// Investor's Bond units (packs of bond_units, received at the same time, belonging to Investor)
        BondUnitPackageRegistry
            get(fn bond_unit_registry):
                double_map hasher(blake2_128_concat) BondId, hasher(blake2_128_concat) T::AccountId => Vec<BondUnitPackage>;

        /// Bond coupon yield storage
        /// Every element has total bond yield of passed period recorded on accrual basis
        BondCouponYield
            get(fn bond_coupon_yield):
                map hasher(blake2_128_concat) BondId=>Vec<PeriodYield>;

        /// Bondholder's last requested coupon yield for given period and bond
        BondLastCouponYield
            get(fn bond_last_coupon_yield):
                double_map hasher(blake2_128_concat) BondId, hasher(blake2_128_concat) T::AccountId => AccountYield;

        /// Bond sale lots for each bond
        BondUnitPackageLot
            get(fn bond_unit_lots):
                double_map hasher(blake2_128_concat) BondId, hasher(blake2_128_concat) T::AccountId => Vec<BondUnitSaleLotStructOf<T>>;

        /// Bond impact report storage
        BondImpactReport
            get(fn impact_reports):
                map hasher(blake2_128_concat) BondId => Vec<BondImpactReportStruct>;
    }
}

decl_event!(
    pub enum Event<T>
    where
        AccountId = <T as frame_system::Trait>::AccountId,
        BondUnitSaleLotStruct = BondUnitSaleLotStructOf<T>, // Moment = <T as pallet_timestamp::Trait>::Moment,
    {
        /// Event documentation should end with an array that provides descriptive names for event
        /// parameters. [something, who]
        // 1: author, 2: newly added account, 3: role, 4: identity
        AccountAdd(AccountId, AccountId, u8, u64),

        // 1: author, 2:  updated account, 3: role, 4: identity
        AccountSet(AccountId, AccountId, u8, u64),

        // 1: author, 2: disabled account
        AccountDisable(AccountId, AccountId),

        // @TODO document events and add many corresponding
        // data for each Event for syschronization service
        MintRequestCreated(AccountId, EverUSDBalance),
        MintRequestRevoked(AccountId, EverUSDBalance),
        MintRequestConfirmed(AccountId, EverUSDBalance),
        MintRequestDeclined(AccountId, EverUSDBalance),

        BurnRequestCreated(AccountId, EverUSDBalance),
        BurnRequestRevoked(AccountId, EverUSDBalance),
        BurnRequestConfirmed(AccountId, EverUSDBalance),
        BurnRequestDeclined(AccountId, EverUSDBalance),
        // Bond events
        BondAdded(AccountId, BondId),
        BondChanged(AccountId, BondId),
        BondRevoked(AccountId, BondId),
        BondReleased(AccountId, BondId),
        BondActivated(AccountId, BondId, EverUSDBalance),
        BondWithdrawal(AccountId, BondId),
        BondImpactReportReceived(AccountId, BondId),
        BondRedeemed(AccountId, BondId, EverUSDBalance),
        BondBankrupted(AccountId, BondId, EverUSDBalance, EverUSDBalance),

        BondWithdrawEverUSD(AccountId, BondId, EverUSDBalance),
        BondDepositEverUSD(AccountId, BondId, EverUSDBalance),

        BondUnitSold(AccountId, BondId, u32),
        BondUnitReturned(AccountId, BondId, u32),

        BondImpactReportSent(AccountId, BondId, BondPeriodNumber, u64),
        BondImpactReportApproved(AccountId, BondId, BondPeriodNumber, u64),
        BondCouponYield(BondId, EverUSDBalance),

        BondSaleLotBid(AccountId, BondId, BondUnitSaleLotStruct),
        BondSaleLotSettle(AccountId, BondId, BondUnitSaleLotStruct),
    }
);

decl_error! {
    pub enum Error for Module<T: Trait> {
        NoneValue,

        /// Account tried to use more EverUSD  than was available on the balance
        BalanceOverdraft,

        /// Account was already added and present in AccountRegistry
        AccountToAddAlreadyExists,

        /// Account not authorized(doesn't have a needed role, or doesnt present in AccountRegistry at all)
        AccountNotAuthorized,

        /// Account does not exist in AccountRegistry
        AccountNotExist,

        /// Role parameter is invalid (bit mask of available roles includes non-existent role)
        AccountRoleParamIncorrect,

        /// Account already created one mint request, only one allowed at a time(to be changed in future)
        MintRequestAlreadyExist,

        /// Mint request for given account doesnt exist
        MintRequestDoesntExist,

        /// Incorrect parameters for mint request(miant amount > MAX_MINT_AMOUNT)
        MintRequestParamIncorrect,

        /// Account already created one burn request, only one allowed at a time(to be changed in future)
        BurnRequestAlreadyExist,

        /// Mint request for given account doesnt exist
        BurnRequestDoesntExist,

        /// Incorrect parameters for mint request(mint amount > MAX_MINT_AMOUNT)
        BurnRequestParamIncorrect,

        /// Bond with same ticker already exists
        /// Every bond on the platform has unique BondId: 8 bytes, like "MUSKPWR1" or "SOLGEN02"
        BondAlreadyExists,

        /// Incorrect bond parameters (many different cases)
        // @TODO refactor this error to make it more descriptive in different cases
        BondParamIncorrect,

        /// Incorrect bond ticker provided or bond has been revoked
        BondNotFound,

        /// Requested action in bond is not permitted for this account
        BondAccessDenied,

        /// Current bond state doesn't permit the requested action
        BondStateNotPermitAction,

        /// Action requires some bond options to be properly initialized
        BondIsNotConfigured,

        /// Requested action is not allowed in current period of time
        BondOutOfOrder,
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        // Errors must be initialized if they are used by the pallet.
        type Error = Error<T>;
        const Unknown: T::AccountId = Default::default();

        // Events must be initialized if they are used by the pallet.
        fn deposit_event() = default;

        // Account management functions

        /// Method: account_disable(who: AccountId)
        /// Arguments: origin: AccountId - transaction caller
        ///            who: AccountId - account to disable
        /// Access: Master role
        ///
        /// Disables all roles of account, setting roles bitmask to 0.
        /// Accounts are not allowed to perform any actions without role,
        /// but still have its data in blockchain (to not loose related entities)
        #[weight = 10_000]
        fn account_disable(origin, who: T::AccountId) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            ensure!(Self::account_is_master(&caller), Error::<T>::AccountNotAuthorized);
            ensure!(AccountRegistry::<T>::contains_key(&who), Error::<T>::AccountNotExist);

            AccountRegistry::<T>::mutate(&who,|acc|{
                acc.roles = 0; // set no roles
            });

            Self::deposit_event(RawEvent::AccountDisable(caller, who));
            Ok(())
        }

        /// Method: account_add_with_role_and_data(origin, who: T::AccountId, role: u8, identity: u64)
        /// Arguments:  origin: AccountId - transaction caller
        ///             who: AccountId - id of account to add to accounts registry of platform
        ///             role: u8 - role(s) of account (see ALL_ROLES_MASK for allowed roles)
        ///             identity: u64 - reserved field for integration with external platforms
        /// Access: Master role
        ///
        /// Adds new account with given role(s). Roles are set as bitmask. Contains parameter
        /// "identity", planned to use in the future to connect accounts with external services like
        /// KYC providers
        #[weight = 10_000]
        fn account_add_with_role_and_data(origin, who: T::AccountId, role: u8, identity: u64) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            ensure!(Self::account_is_master(&caller), Error::<T>::AccountNotAuthorized);
            ensure!(!AccountRegistry::<T>::contains_key(&who), Error::<T>::AccountToAddAlreadyExists);
            ensure!(is_roles_correct(role), Error::<T>::AccountRoleParamIncorrect);

            let now: <T as pallet_timestamp::Trait>::Moment = <pallet_timestamp::Module<T>>::get();

            AccountRegistry::<T>::insert(&who,
                EvercityAccountStructT { roles: role, identity, create_time: now }
            );
            debug::error!("account_add_with_role_and_data: who={:?} when={:?}", who, now);

            Self::deposit_event(RawEvent::AccountAdd(caller, who, role, identity));
            Ok(())
        }

        /// Method: account_set_with_role_and_data(origin, who: T::AccountId, role: u8, identity: u64)
        /// Arguments:  origin: AccountId - transaction caller
        ///             who: AccountId - account to modify
        ///             role: u8 - role(s) of account (see ALL_ROLES_MASK for allowed roles)
        ///             identity: u64 - reserved field for integration with external platforms
        /// Access: Master role
        ///
        /// Modifies existing account, assigning new role(s) or identity to it
        #[weight = 10_000]
        fn account_set_with_role_and_data(origin, who: T::AccountId, role: u8, identity: u64) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            ensure!(Self::account_is_master(&caller), Error::<T>::AccountNotAuthorized);
            ensure!(AccountRegistry::<T>::contains_key(&who), Error::<T>::AccountNotExist);
            ensure!(is_roles_correct(role), Error::<T>::AccountRoleParamIncorrect);

            AccountRegistry::<T>::mutate(&who,|acc|{
                acc.roles |= role;
            });

            Self::deposit_event(RawEvent::AccountSet(caller, who, role, identity));
            Ok(())
        }

        // Token balances manipulation functions

        /// Method: token_mint_request_create_everusd(origin, amount_to_mint: EverUSDBalance)
        /// Arguments:  origin: AccountId - transaction caller
        ///             amount_to_mint: EverUSDBalance - amount of tokens to mint
        /// Access: Investor or Issuer role
        ///
        /// Creates a request to mint given amount of EverUSD tokens on caller's balance.
        /// Custodian account confirms request after receiving payment in USD from target account's owner
        /// It's possible to create only one request per account. Mint request has a time-to-live
        /// and becomes invalidated after it.
        #[weight = 15_000]
        fn token_mint_request_create_everusd(origin, amount_to_mint: EverUSDBalance) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            ensure!(Self::account_token_mint_burn_allowed(&caller), Error::<T>::AccountNotAuthorized);
            ensure!(amount_to_mint < EVERUSD_MAX_MINT_AMOUNT, Error::<T>::MintRequestParamIncorrect);
            // @TODO remove an existing request if it expired
            ensure!(!MintRequestEverUSD::<T>::contains_key(&caller), Error::<T>::MintRequestAlreadyExist);

            let now: <T as pallet_timestamp::Trait>::Moment = <pallet_timestamp::Module<T>>::get();
            let new_mint_request = TokenMintRequestStruct{
                amount: amount_to_mint,
                deadline: now + TOKEN_MINT_REQUEST_TTL .into(),
            };
            MintRequestEverUSD::<T>::insert(&caller, new_mint_request);

            Self::deposit_event(RawEvent::MintRequestCreated(caller, amount_to_mint));
            Ok(())
        }

        /// Method: token_mint_request_revoke_everusd(origin)
        /// Arguments: origin: AccountId - transaction caller
        /// Access: Investor or Issuer role
        ///
        /// Revokes and deletes currently existing mint request, created by caller's account
        #[weight = 5_000]
        fn token_mint_request_revoke_everusd(origin) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            ensure!(MintRequestEverUSD::<T>::contains_key(&caller), Error::<T>::MintRequestDoesntExist);
            let _amount = MintRequestEverUSD::<T>::get(&caller).amount;
            MintRequestEverUSD::<T>::remove(&caller);
            Self::deposit_event(RawEvent::MintRequestRevoked(caller, _amount));
            Ok(())
        }

        /// Method: token_mint_request_confirm_everusd(origin, who: T::AccountId, amount: EverUSDBalance)
        /// Arguments:  origin: AccountId - transaction caller
        ///             who: AccountId - target account
        ///             amount: EverUSDBalance - amount of tokens to mint, confirmed by Custodian
        /// Access: Custodian role
        ///
        /// Confirms the mint request of account, creating "amount" of tokens on its balance.
        /// (note) Amount of tokens is sent as parameter to avoid data race problem, when
        /// Custodian can confirm unwanted amount of tokens, because attacker is modified mint request
        /// while Custodian makes a decision
        #[weight = 15_000]
        fn token_mint_request_confirm_everusd(origin, who: T::AccountId, amount: EverUSDBalance) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            ensure!(Self::account_is_custodian(&caller),Error::<T>::AccountNotAuthorized);
            ensure!(MintRequestEverUSD::<T>::contains_key(&who), Error::<T>::MintRequestDoesntExist);
            let mint_request = MintRequestEverUSD::<T>::get(&who);
            let now: <T as pallet_timestamp::Trait>::Moment = <pallet_timestamp::Module<T>>::get();
            ensure!(mint_request.deadline >= now, Error::<T>::MintRequestDoesntExist);

            // add tokens to user's balance and total supply of EverUSD
            let amount_to_add = mint_request.amount;
            ensure!(amount_to_add==amount,Error::<T>::MintRequestParamIncorrect );

            Self::balance_add(&who, amount_to_add)?;

            TotalSupplyEverUSD::try_mutate(|total|->DispatchResult{
                *total = total.checked_add(amount_to_add).ok_or( Error::<T>::BalanceOverdraft )?;
                Ok(())
            })?;

            MintRequestEverUSD::<T>::remove(&who);
            Self::deposit_event(RawEvent::MintRequestConfirmed(who, amount_to_add));
            Self::purge_expired_mint_requests(now);
            Ok(())
        }

        /// Method: token_mint_request_decline_everusd(origin, who: T::AccountId)
        /// Arguments:  origin: AccountId - transaction caller
        ///             who: AccountId - target account
        /// Access: Custodian role
        ///
        /// Declines and deletes the mint request of account (Custodian)
        #[weight = 5_000]
        fn token_mint_request_decline_everusd(origin, who: T::AccountId) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            ensure!(Self::account_is_custodian(&caller),Error::<T>::AccountNotAuthorized);
            ensure!(MintRequestEverUSD::<T>::contains_key(&who), Error::<T>::MintRequestDoesntExist);
            let amount = MintRequestEverUSD::<T>::get(&who).amount;
            MintRequestEverUSD::<T>::remove(&who);
            Self::deposit_event(RawEvent::MintRequestDeclined(caller, amount));
            Ok(())
        }

        /// Method: token_burn_request_create_everusd(origin, amount_to_burn: EverUSDBalance)
        /// Arguments:  origin: AccountId - transaction caller
        ///             amount_to_burn: EverUSDBalance - amount of tokens to burn
        /// Access: Investor or Issuer role
        ///
        /// Creates a request to burn given amount of EverUSD tokens on caller's balance.
        /// Custodian account confirms request after sending payment in USD to target account's owner
        /// It's possible to create only one request per account. Burn request has a time-to-live
        /// and becomes invalidated after it.
        #[weight = 15_000]
        fn token_burn_request_create_everusd(origin, amount_to_burn: EverUSDBalance) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            ensure!(Self::account_token_mint_burn_allowed(&caller), Error::<T>::AccountNotAuthorized);
            // @TODO remove an existing request if it expired
            ensure!(!MintRequestEverUSD::<T>::contains_key(&caller), Error::<T>::MintRequestAlreadyExist);

            let current_balance = BalanceEverUSD::<T>::get(&caller);
            ensure!(amount_to_burn <= current_balance, Error::<T>::MintRequestParamIncorrect);
            let now: <T as pallet_timestamp::Trait>::Moment = <pallet_timestamp::Module<T>>::get();

            let new_burn_request = TokenBurnRequestStruct {
                amount: amount_to_burn,
                deadline: now + TOKEN_BURN_REQUEST_TTL .into(),
            };
            BurnRequestEverUSD::<T>::insert(&caller, new_burn_request);

            Self::deposit_event(RawEvent::BurnRequestCreated(caller, amount_to_burn));
            Ok(())
        }

        /// Method: token_burn_request_revoke_everusd(origin)
        /// Arguments: origin: AccountId - transaction caller
        /// Access: Investor or Issuer role
        ///
        /// Revokes and deletes currently existing burn request, created by caller's account
        #[weight = 5_000]
        fn token_burn_request_revoke_everusd(origin) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            ensure!(BurnRequestEverUSD::<T>::contains_key(&caller), Error::<T>::BurnRequestDoesntExist);
            let amount = BurnRequestEverUSD::<T>::get(&caller).amount;
            BurnRequestEverUSD::<T>::remove(&caller);
            Self::deposit_event(RawEvent::BurnRequestRevoked(caller, amount));
            Ok(())
        }

        /// Method: token_burn_request_confirm_everusd(origin, who: T::AccountId, amount: EverUSDBalance)
        /// Arguments:  origin: AccountId - transaction caller
        ///             who: AccountId - target account
        ///             amount: EverUSDBalance - amount of tokens to mint, confirmed by Custodian
        /// Access: Custodian role
        ///
        /// Confirms the burn request of account, destroying "amount" of tokens on its balance.
        #[weight = 15_000]
        fn token_burn_request_confirm_everusd(origin, who: T::AccountId, amount: EverUSDBalance) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            ensure!(Self::account_is_custodian(&caller),Error::<T>::AccountNotAuthorized);
            ensure!(BurnRequestEverUSD::<T>::contains_key(&who), Error::<T>::BurnRequestDoesntExist);
            let burn_request = BurnRequestEverUSD::<T>::get(&who);
            let now: <T as pallet_timestamp::Trait>::Moment = <pallet_timestamp::Module<T>>::get();
            ensure!(burn_request.deadline >= now, Error::<T>::BurnRequestDoesntExist);
            // remove tokens from user's balance and decrease total supply of EverUSD
            let amount_to_sub = burn_request.amount;
            // prevent unacceptable commit
            ensure!(amount_to_sub==amount, Error::<T>::MintRequestParamIncorrect );

            Self::balance_sub(&who, amount_to_sub)?;
            TotalSupplyEverUSD::mutate(|total|{
                *total-=amount_to_sub;
            });

            BurnRequestEverUSD::<T>::remove(&who);
            Self::deposit_event(RawEvent::BurnRequestConfirmed(who, amount_to_sub));
            Self::purge_expired_burn_requests(now);
            Ok(())
        }

        /// Method: token_burn_request_decline_everusd(origin, who: T::AccountId)
        /// Arguments:  origin: AccountId - transaction caller
        ///             who: AccountId - target account
        /// Access: Custodian role
        ///
        /// Declines and deletes the burn request of account (Custodian)
        #[weight = 5_000]
        fn token_burn_request_decline_everusd(origin, who: T::AccountId) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            ensure!(Self::account_is_custodian(&caller),Error::<T>::AccountNotAuthorized);
            ensure!(BurnRequestEverUSD::<T>::contains_key(&who), Error::<T>::BurnRequestDoesntExist);
            let amount = BurnRequestEverUSD::<T>::get(&who).amount;
            BurnRequestEverUSD::<T>::remove(&who);
            Self::deposit_event(RawEvent::BurnRequestDeclined(caller, amount));
            Ok(())
        }

        // Bonds handling functions

        /// Method: bond_add_new(origin, origin, bond: BondId, body: BondInnerStruct)
        /// Arguments: origin: AccountId - transaction caller
        ///            bond: BondId - bond identifier
        ///            body: BondInnerStruct
        /// Access: Issuer role
        ///
        /// Creates new bond with given BondId (8 bytes) and pack of parameters, set by BondInnerStruct.
        /// Bond is created in BondState::PREPARE, and can be modified many times until it becomes ready
        /// for next BondState::BOOKING, when most of BondInnerStruct parameters cannot be changed, and
        /// Investors can buy bond units
        #[weight = 20_000]
        fn bond_add_new(origin, bond: BondId, body: BondInnerStructOf<T> ) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            ensure!(Self::account_is_issuer(&caller),Error::<T>::AccountNotAuthorized);
            ensure!(body.is_valid(), Error::<T>::BondParamIncorrect );
            ensure!(!BondRegistry::<T>::contains_key(&bond), Error::<T>::BondAlreadyExists);

            let now: <T as pallet_timestamp::Trait>::Moment = <pallet_timestamp::Module<T>>::get();

            let item = BondStruct{
                    inner: body,

                    issuer: caller.clone(),
                    auditor: Default::default(),
                    manager: Default::default(),
                    impact_reporter: Default::default(),

                    issued_amount: 0,
                    booking_start_date: Default::default(),
                    active_start_date: Default::default(),
                    creation_date: now,
                    state: BondState::PREPARE,
                    bond_debit: 0,
                    bond_credit: 0,
                    coupon_yield: 0
            };
            BondRegistry::<T>::insert(&bond, item);
            Self::deposit_event(RawEvent::BondAdded(caller, bond));
            Ok(())
        }

        /// Method: bond_set_auditor(origin, bond: BondId, acc: T::AccountId)
        /// Arguments: origin: AccountId - transaction caller, assigner
        ///            bond: BondId - bond identifier
        ///            acc: AccountId - assignee account
        /// Access: Master role
        ///
        /// Assigns target account to be the manager of the bond. Manager can make
        /// almost the same actions with bond as Issuer, instead of most important,
        /// helping Issuer to manage bond parameters, work with documents, etc...
        #[weight = 5_000]
        fn bond_set_manager(origin, bond: BondId, acc: T::AccountId) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            // Bond Auxiliary roles can be set only by Master
            ensure!(Self::account_is_master(&caller), Error::<T>::AccountNotAuthorized);
            ensure!(Self::account_is_manager(&acc), Error::<T>::AccountRoleParamIncorrect);

            Self::with_bond(&bond, |item|{
                ensure!(
                    matches!(item.state, BondState::PREPARE ),
                    Error::<T>::BondStateNotPermitAction
                );
                item.manager = acc;
                Self::deposit_event(RawEvent::BondChanged(caller, bond ));
                Ok(())
            })
        }

        /// Method: bond_set_auditor(origin, bond: BondId, acc: T::AccountId)
        /// Arguments: origin: AccountId - transaction caller, assigner
        ///            bond: BondId - bond identifier
        ///            acc: AccountId - assignee
        /// Access: Master role
        ///
        /// Assigns target account to be the auditor of the bond. Auditor confirms
        /// impact data coming in bond, and performs other verification-related actions
        #[weight = 5_000]
        fn bond_set_auditor(origin, bond: BondId, acc: T::AccountId) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            // Bond auxiliary roles can be set only by Master
            ensure!(Self::account_is_master(&caller), Error::<T>::AccountNotAuthorized);
            ensure!(Self::account_is_auditor(&acc), Error::<T>::AccountRoleParamIncorrect);

            Self::with_bond(&bond, |item|{
                ensure!(
                    matches!(item.state, BondState::PREPARE | BondState::BOOKING),
                    Error::<T>::BondStateNotPermitAction
                );
                item.auditor = acc;
                Self::deposit_event(RawEvent::BondChanged(caller, bond ));
                Ok(())
            })
        }

        /// Method: bond_set_impact_reporter(origin, bond: BondId, acc: T::AccountId)
        /// Arguments: origin: AccountId - transaction caller, assigner
        ///            bond: BondId - bond identifier
        ///            acc: AccountId - assignee
        ///
        /// Assigns impact reporter to the bond
        /// Access: only accounts with Master role
        #[weight = 5_000]
        fn bond_set_impact_reporter(origin, bond: BondId, acc: T::AccountId) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            // Bond auxiliary roles can be set only by Master
            ensure!(Self::account_is_master(&caller), Error::<T>::AccountNotAuthorized);
            ensure!(Self::account_is_impact_reporter(&acc), Error::<T>::AccountRoleParamIncorrect);

            Self::with_bond(&bond, |item|{
                item.impact_reporter = acc;
                Self::deposit_event(RawEvent::BondChanged(caller, bond ));
                Ok(())
            })
        }

        /// Method: bond_update(origin, origin, bond: BondId, body: BondInnerStruct)
        /// Arguments: origin: AccountId - transaction caller
        ///            bond: BondId - bond identifier
        ///            body: BondInnerStruct
        ///
        /// Updates bond data. Being released bond can be changed only in  part of document hashed
        /// Access: bond issuer or bond manager
        #[weight = 10_000]
        fn bond_update(origin, bond: BondId, body: BondInnerStructOf<T>) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            ensure!(body.is_valid(), Error::<T>::BondParamIncorrect );
            // Bond can be update only by Owner or assigned Manager
            Self::with_bond(&bond, |item|{
                // preserving the bond_units_base_price value
                ensure!(
                    matches!(item.state, BondState::PREPARE | BondState::BOOKING),
                    Error::<T>::BondStateNotPermitAction
                );
                ensure!(
                    item.issuer == caller || item.manager == caller ,
                    Error::<T>::BondAccessDenied
                );
                // Financial data shell not be changed after release
                if item.state == BondState::BOOKING {
                    ensure!( item.inner.is_financial_options_eq(&body), Error::<T>::BondStateNotPermitAction );
                }
                item.inner = body;
                Self::deposit_event(RawEvent::BondChanged(caller, bond ));

                Ok(())
            })
        }

        /// Method: bond_release(origin, bond: BondId)
        /// Arguments: origin: AccountId - transaction caller
        ///            bond: BondId - bond identifier
        ///
        /// Releases the bond on the market starting presale .
        /// Marks the bond as `BOOKING` allowing investors to stake it.
        /// Access: only accounts with Master role
        // @TODO add timestamp parameter to prevent race conditions
        #[weight = 5_000]
        fn bond_release(origin, bond: BondId) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            // Bond can be released only by Master
            ensure!(Self::account_is_master(&caller), Error::<T>::AccountNotAuthorized);
            Self::with_bond(&bond, |item|{
                ensure!(item.state == BondState::PREPARE, Error::<T>::BondStateNotPermitAction);
                ensure!(item.inner.is_valid(), Error::<T>::BondParamIncorrect );

                let now = <pallet_timestamp::Module<T>>::get();
                // Ensure booking deadline is in the future
                ensure!(item.inner.mincap_deadline>now, Error::<T>::BondParamIncorrect );

                item.booking_start_date = now;
                item.state = BondState::BOOKING;
                Self::deposit_event(RawEvent::BondReleased(caller, bond ));
                Ok(())
            })
        }

        /// Method: bond_unit_package_buy(origin, bond: BondId, unit_amount: BondUnitAmount )
        /// Arguments: origin: AccountId - transaction caller
        ///            bond: BondId - bond identifier
        ///            unit_amount: BondUnitAmount - amount of bond units
        ///
        /// Bye bond units.
        /// Access: only accounts with Investor role
        // Investor loans tokens to the bond issuer by staking bond units
        #[weight = 10_000]
        fn bond_unit_package_buy(origin, bond: BondId, unit_amount: BondUnitAmount ) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            ensure!(Self::account_is_investor(&caller), Error::<T>::AccountNotAuthorized);
            Self::with_bond(&bond, |mut item|{
                ensure!(
                    matches!(item.state, BondState::BANKRUPT | BondState::ACTIVE | BondState::BOOKING),
                    Error::<T>::BondStateNotPermitAction
                );
                // issuer cannot buy his own bonds
                ensure!(item.issuer!=caller, Error::<T>::BondParamIncorrect );

                let issued_amount = unit_amount.checked_add(item.issued_amount)
                    .ok_or(Error::<T>::BondParamIncorrect)?;

                ensure!(
                    issued_amount <= item.inner.bond_units_maxcap_amount,
                    Error::<T>::BondParamIncorrect
                );

                let package_value =  item.par_value( unit_amount ) ;

                Self::balance_sub(&caller, package_value)?;

                let now = <pallet_timestamp::Module<T>>::get();

                // get the number of seconds after bond activation.
                // zero value if the bond has not activated yet
                let (acquisition,_) = item.time_passed_after_activation( now ).unwrap_or( (0,0) );
                // @FIXME assess the costs of current array struct for storing packages and
                // compare them with a more efficient way to store data
                BondUnitPackageRegistry::<T>::mutate(&bond, &caller, |packages|{
                    packages.push(
                        BondUnitPackage{
                             bond_units: unit_amount,
                             acquisition,
                             coupon_yield: 0,
                        }
                    );
                });

                item.issued_amount = issued_amount;


                if matches!(item.state, BondState::ACTIVE | BondState::BANKRUPT) {
                    item.bond_debit += package_value;
                    // in BondState::ACTIVE or BondState::BANKRUPT received everusd
                    // can be forwarded to pay off the debt
                    Self::calc_and_store_bond_coupon_yield(&bond, &mut item, now);
                    // surplus to the issuer balance
                    let free_balance = item.get_free_balance();
                    if free_balance > 0 {
                        item.bond_debit -= free_balance;
                        Self::balance_add(&item.issuer, free_balance)?;
                    }
                }else{
                    // in BondState::PREPARE just increase assets and liabilities of the Bond
                    item.increase( package_value );
                }

                Self::deposit_event(RawEvent::BondUnitSold(caller.clone(), bond, unit_amount ));

                // @FIXME
                // According to the Design document
                // the Bond can be activated only by Master.
                // Disable instant activation.

                // Activate the Bond if it raised more than minimum
                // if item.state == BondState::BOOKING && item.issued_amount >= item.inner.bond_units_mincap_amount {
                //     let now = <pallet_timestamp::Module<T>>::get();
                //     item.active_start_date = now;
                //     item.state = BondState::ACTIVE;
                //     item.timestamp = now;
                //     Self::deposit_event(RawEvent::BondActivated(caller, bond ));
                // }
                Ok(())
            })
        }

        /// Method: bond_unit_package_return(origin, bond: BondId, unit_amount: BondUnitAmount )
        /// Arguments: origin: AccountId - transaction caller
        ///            bond: BondId - bond identifier
        ///            unit_amount: BondUnitAmount - amount of bond units
        ///
        /// Gives back staked on presale bond units.
        /// Access: only accounts with Investor role who hold bond units
        // Investor gives back bond units and withdraw tokens
        #[weight = 10_000]
        fn bond_unit_package_return(origin, bond: BondId, unit_amount: BondUnitAmount ) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            ensure!(Self::account_is_investor(&caller), Error::<T>::AccountNotAuthorized);
            // Active Bond cannot be withdrawn
            Self::with_bond(&bond, |item|{
                ensure!(item.state == BondState::BOOKING, Error::<T>::BondStateNotPermitAction );
                ensure!(item.issued_amount>=unit_amount, Error::<T>::BondParamIncorrect);
                let package_value =  item.par_value( unit_amount ) ;
                ensure!(item.bond_credit>=package_value, Error::<T>::BondParamIncorrect);

                //@TODO add ability to give back part of the package
                let mut packages = BondUnitPackageRegistry::<T>::get(&bond, &caller);
                ensure!(!packages.is_empty(), Error::<T>::BondParamIncorrect);

                if let Some(index) = packages.iter().position(|item| item.bond_units == unit_amount ){
                    packages.remove( index );
                    BondUnitPackageRegistry::<T>::insert(&bond, &caller, packages);
                }else{
                    return Err( Error::<T>::BondParamIncorrect.into() );
                }

                item.decrease( package_value );
                item.issued_amount -= unit_amount;

                Self::balance_add(&caller, package_value )?;
                Self::deposit_event(RawEvent::BondUnitReturned(caller, bond, unit_amount ));

                Ok(())
            })
        }

        /// Method: bond_withdraw(origin, bond: BondId)
        /// Arguments: origin: AccountId - transaction caller
        ///            bond: BondId - bond identifier
        ///
        /// Called after the bond was released but not raised enough capacity until deadline.
        /// Access: accounts with Master role, bond issuer, or bond manager
        // Called after the Bond was released but not raised enough tokens until the deadline
        #[weight = 10_000]
        fn bond_withdraw(origin, bond: BondId) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            // Bond issuer, bond Manager, or Master can do it
            Self::with_bond(&bond, |item|{
                ensure!( item.state == BondState::BOOKING, Error::<T>::BondStateNotPermitAction );
                // Ensure the Bond raises less then bond_units_mincap_amount bond units
                ensure!(item.inner.bond_units_mincap_amount > item.issued_amount, Error::<T>::BondParamIncorrect);
                ensure!(
                    item.issuer == caller || item.manager == caller || Self::account_is_master(&caller) ,
                    Error::<T>::BondAccessDenied
                );
                let now = <pallet_timestamp::Module<T>>::get();
                // Ensure booking deadline is in the future
                ensure!(item.inner.mincap_deadline<=now, Error::<T>::BondParamIncorrect );

                item.state = BondState::PREPARE;

                assert!(item.bond_credit == item.par_value(item.issued_amount));
                // @TODO make it lazy. this implementation do much work to restore balances
                // that is too CPU and memory expensive
                // for all bondholders
                for (bondholder, package) in BondUnitPackageRegistry::<T>::iter_prefix(&bond){
                      let bondholder_total_amount: BondUnitAmount = package.iter()
                      .map(|item| item.bond_units )
                      .sum();

                      item.issued_amount -= bondholder_total_amount;

                      let transfer = item.par_value( bondholder_total_amount ) ;
                      item.decrease(transfer);

                      Self::balance_add(&bondholder, transfer)?;
                }
                assert!(item.bond_credit == 0);
                assert!(item.issued_amount==0);

                BondUnitPackageRegistry::<T>::remove_prefix(&bond);

                Self::deposit_event(RawEvent::BondWithdrawal(caller, bond ));
                Ok(())
            })
        }

        /// Method: bond_activate(origin, bond: BondId)
        /// Arguments: origin: AccountId - transaction caller
        ///            bond: BondId - bond identifier
        ///
        /// Activates the bond after it raised minimum capacity of bond units.
        /// It makes bond fund available to the issuer and stop bond  withdrawal until
        /// maturity date.
        /// Access: only accounts with Master role
        #[weight = 5_000]
        fn bond_activate(origin, bond: BondId) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            //Bond can be activated only by Master
            ensure!(Self::account_is_master(&caller), Error::<T>::AccountNotAuthorized);
            //if it's raised enough bond units during bidding process
            Self::with_bond(&bond, |item|{
                ensure!(item.state == BondState::BOOKING, Error::<T>::BondStateNotPermitAction);
                ensure!(item.inner.bond_units_mincap_amount <= item.issued_amount, Error::<T>::BondParamIncorrect);
                // auditor should be assigned before
                ensure!(item.auditor != Default::default(), Error::<T>::BondIsNotConfigured);

                let now = <pallet_timestamp::Module<T>>::get();
                item.state = BondState::ACTIVE;
                item.active_start_date = now;
                // Decrease liabilities by value of fund
                assert_eq!(item.bond_credit, item.par_value( item.issued_amount ) );
                assert!(item.bond_credit == item.bond_debit);
                item.bond_credit = 0 ;

                // create impact report struct.
                // the total number or reports is equal to the number of periods plus 1 (start period)
                let mut reports: Vec<BondImpactReportStruct> = Vec::new();
                reports.resize( ( item.inner.bond_duration + 1 ) as usize,  BondImpactReportStruct{
                    create_date: 0,
                    impact_data: 0,
                    signed: false,
                });

                BondImpactReport::insert(&bond, &reports);

                // withdraw all available bond fund
                let amount = item.bond_debit;
                Self::balance_add(&item.issuer, item.bond_debit)?;
                item.bond_debit = 0;

                Self::deposit_event(RawEvent::BondActivated(caller, bond, amount ));
                Ok(())
            })
        }

        /// Method: bond_impact_report_send(origin, bond: BondId, impact_data: u64 )
        /// Arguments: origin: AccountId - transaction caller
        ///            bond: BondId - bond identifier
        ///            impact_data: u64 - report value
        ///
        /// Releases periodic impact report
        /// Access: bond issuer or reporter assigned to the bond
        #[weight = 15_000]
        fn bond_impact_report_send(origin, bond: BondId, period: BondPeriodNumber, impact_data: u64 ) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            let now = <pallet_timestamp::Module<T>>::get();
            let moment = {
                let item = BondRegistry::<T>::get(bond);
                ensure!(item.issuer == caller || item.impact_reporter == caller, Error::<T>::BondAccessDenied );
                ensure!(Self::is_report_in_time(&item, now, period), Error::<T>::BondOutOfOrder );
                item.time_passed_after_activation( now ).map( |(moment, _period)| moment ).unwrap()
            };

            let index: usize = period as usize;
            BondImpactReport::try_mutate(&bond, |reports|->DispatchResult {

                ensure!(index < reports.len() && !reports[index].signed, Error::<T>::BondParamIncorrect );

                reports[index].create_date = moment;
                reports[index].impact_data = impact_data;

                Self::deposit_event(RawEvent::BondImpactReportSent( caller, bond, period, impact_data ));
                Ok(())
            })
        }

        /// Method: bond_impact_report_approve(origin, bond: BondId, period: u64, impact_data: u64 )
        /// Arguments: origin: AccountId - transaction caller
        ///            bond: BondId - bond identifier
        ///            period: u32 - report period starting from 0
        ///            impact_data: u64 - report value
        ///
        /// Verify report impact data by signing the report released by the bond issuer
        /// Access: only auditor assigned to the bond
        // Auditor signs impact report
        #[weight = 5_000]
        fn bond_impact_report_approve(origin, bond: BondId, period: BondPeriodNumber, impact_data: u64 ) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            ensure!(Self::account_is_auditor(&caller), Error::<T>::AccountNotAuthorized);
            let now = <pallet_timestamp::Module<T>>::get();
            {
                let item = BondRegistry::<T>::get(bond);
                ensure!(item.auditor == caller, Error::<T>::BondAccessDenied );
                ensure!(Self::is_report_in_time(&item, now, period), Error::<T>::BondOutOfOrder );
            }

            let index: usize = period as usize;
            BondImpactReport::try_mutate(&bond, |reports|->DispatchResult {

                ensure!(index < reports.len(), Error::<T>::BondParamIncorrect );
                let report = &reports[index];
                ensure!(report.create_date > 0 , Error::<T>::BondParamIncorrect);
                ensure!(!report.signed && report.impact_data == impact_data,
                 Error::<T>::BondParamIncorrect
                );

                reports[index].signed = true;

                Self::deposit_event(RawEvent::BondImpactReportApproved( caller, bond, period, impact_data ));
                Ok(())
            })
        }

        /// Method: bond_redeem(origin, bond: BondId)
        /// Arguments: origin: AccountId - transaction caller
        ///            bond: BondId - bond identifier
        ///
        /// Makes the bond reached maturity date. It requires the issuer to pay back
        /// redemption yield
        // Switch the Bond state to Finished
        #[weight = 15_000]
        fn bond_redeem(origin, bond: BondId) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            let now = <pallet_timestamp::Module<T>>::get();
            Self::with_bond(&bond, |mut item|{
                ensure!( matches!(item.state, BondState::ACTIVE|BondState::BANKRUPT), Error::<T>::BondStateNotPermitAction );

                match item.time_passed_after_activation(now){
                    Some((_, period))  if period == item.get_periods() => (),
                    _ => return Err( Error::<T>::BondOutOfOrder.into() ),
                };

                Self::calc_and_store_bond_coupon_yield(&bond, &mut item, now);
                // now bond_credit has  YTM ( yield to mature )
                let amount = item.bond_credit + item.par_value( item.issued_amount ) ;
                if amount <= item.bond_debit {
                    // withdraw free balance
                    Self::balance_add(&item.issuer, item.bond_debit - amount)?;
                }else{
                    let transfer = amount - item.bond_debit;
                    // pay off debt
                    Self::balance_sub(&item.issuer, transfer)?;
                }
                let ytm = item.bond_credit;
                item.bond_credit = 0;
                //item.coupon_yield = amount;
                item.bond_debit = amount;
                item.state = BondState::FINISHED;

                Self::deposit_event(RawEvent::BondRedeemed(caller, bond, ytm ));
                Ok(())
            })
        }

        /// Method: bond_declare_bankrupt(origin, bond: BondId)
        /// Arguments: origin: AccountId - transaction caller
        ///            bond: BondId - bond identifier
        /// Access: Master role
        ///
        /// Marks the bond as bankrupt
        #[weight = 10_000]
        fn bond_declare_bankrupt(origin, bond: BondId) -> DispatchResult {
            let caller = ensure_signed(origin)?;
             ensure!(Self::account_is_master(&caller), Error::<T>::AccountNotAuthorized);

            Self::with_bond(&bond, |mut item|{
                ensure!(item.state == BondState::ACTIVE, Error::<T>::BondStateNotPermitAction);
                ensure!(item.get_debt()>0, Error::<T>::BondOutOfOrder );
                let now = <pallet_timestamp::Module<T>>::get();
                Self::calc_and_store_bond_coupon_yield(&bond, &mut item, now);
                // @TODO refine condition
                item.state = BondState::BANKRUPT;

                Self::deposit_event(RawEvent::BondBankrupted(caller, bond, item.bond_credit, item.bond_debit ));
                Ok(())
            })
        }

        /// Method: bond_accrue_coupon_yield(origin, bond: BondId)
        /// Arguments: origin: AccountId - transaction caller
        ///            bond: BondId - bond identifier
        /// Access: any
        ///
        /// Calculates bond coupon yield
        #[weight = 25_000]
        fn bond_accrue_coupon_yield(origin, bond: BondId) -> DispatchResult {
            let _ = ensure_signed(origin)?;

            Self::with_bond(&bond, |mut item|{
                let now = <pallet_timestamp::Module<T>>::get();
                Self::calc_and_store_bond_coupon_yield(&bond, &mut item, now);
                Ok(())
            })
        }

        /// Method: bond_revoke(origin, bond: BondId)
        /// Arguments: origin: AccountId - transaction caller
        ///            bond: BondId - bond identifier
        /// Access: Bond issuer or manager assigned to the bond
        ///
        /// Cancel bond before it was issued
        /// Access: only accounts with Master role
        #[weight = 5_000]
        fn bond_revoke(origin, bond: BondId) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            // Bond can be revoked only by Owner or by Manager assigned to the Bond
            // Bond should be in Prepare state, so no bids can exist at this time

            ensure!( BondRegistry::<T>::contains_key(&bond), Error::<T>::BondNotFound );
            let item = BondRegistry::<T>::get(bond);
            ensure!(item.issuer == caller || item.manager == caller, Error::<T>::BondAccessDenied);
            ensure!(item.state == BondState::PREPARE, Error::<T>::BondStateNotPermitAction);
            assert!( BondRegistry::<T>::contains_key(bond) );
            BondRegistry::<T>::remove( &bond );

            Self::deposit_event(RawEvent::BondRevoked(caller, bond ));
            Ok(())
        }

        /// Method: bond_withdraw_everusd(origin, bond: BondId, amount: EverUSDBalance)
        /// Arguments: origin: AccountId - transaction caller
        ///            bond: BondId - bond identifier
        ///
        /// Access: Bond issuer or any investor
        ///
        /// Transfer `coupon yield` for investors or `free bond balance` of the bond fund for issuer
        /// to the caller account balance
        //  @TODO add parameter beneficiary:AccountId  who will receive coupon yield
        #[weight = 5_000]
        fn bond_withdraw_everusd(origin, bond: BondId) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            Self::with_bond(&bond, |mut item|{
                ensure!( matches!(item.state , BondState::ACTIVE | BondState::BANKRUPT | BondState::FINISHED), Error::<T>::BondStateNotPermitAction);

                let now = <pallet_timestamp::Module<T>>::get();
                Self::calc_and_store_bond_coupon_yield(&bond, &mut item, now);

                let amount: EverUSDBalance = if item.issuer == caller {
                    // issuer withdraw bond fund
                    let amount = item.get_free_balance();
                    if amount>0{
                        Self::balance_add(&item.issuer, amount)?;
                        // it's safe to do unchecked subtraction
                        item.bond_debit -= amount;
                    }
                    amount
                }else if item.state == BondState::FINISHED {
                    // investor (bondholder) withdraw principal value
                    Self::redeem_bond_units(&bond, &mut item, &caller)
                }else{
                    // investor (bondholder) withdraw coupon yield
                    Self::request_coupon_yield(&bond, &mut item, &caller)
                };

                if item.get_debt()>0 {
                    // @TODO refine condition, take into account payment period
                    item.state = BondState::BANKRUPT;
                }

                if amount>0{
                    Self::deposit_event(RawEvent::BondWithdrawEverUSD(caller, bond, amount ));
                }
                Ok(())
            })
        }

        /// Method: bond_deposit_everusd(origin, bond: BondId, amount: EverUSDBalance)
        /// Arguments: origin: AccountId - transaction caller
        ///            bond: BondId - bond identifier
        ///            amount: EverUSDBalance - the number of EverUSD  deposited to bond fund
        /// Access: Bond issuer
        ///
        /// Transfer `amount` of EverUSD tokens from issuer(caller) balance to the bond fund
        #[weight = 5_000]
        fn bond_deposit_everusd(origin, bond: BondId, amount: EverUSDBalance) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            Self::with_bond(&bond, |mut item|{
                ensure!(
                    matches!(item.state , BondState::ACTIVE | BondState::BANKRUPT),
                    Error::<T>::BondStateNotPermitAction
                );
                ensure!(item.issuer == caller, Error::<T>::BondAccessDenied);

                Self::balance_sub(&caller, amount)?;

                item.bond_debit = item.bond_debit.checked_add( amount )
                    .ok_or( Error::<T>::BondParamIncorrect )?;
                let now = <pallet_timestamp::Module<T>>::get();
                Self::calc_and_store_bond_coupon_yield(&bond, &mut item, now);
                Self::deposit_event(RawEvent::BondDepositEverUSD(caller, bond, amount ));
                Ok(())
            })
        }

        /// Method: bond_unit_lot_bid(origin, bond: BondId, lot: BondUnitSaleLotStruct)
        /// Arguments: origin: AccountId - bond unit bondholder
        ///            bond: BondId - bond identifier
        ///            lot: BondUnitSaleLotStruct - lot data
        /// Access: Bond bondholder
        ///
        /// Create sale lot
        #[weight = 5_000]
        fn bond_unit_lot_bid(origin, bond: BondId, lot: BondUnitSaleLotStructOf<T>) -> DispatchResult{
            let caller = ensure_signed(origin)?;
            let now = <pallet_timestamp::Module<T>>::get();
            ensure!(!lot.is_expired(now), Error::<T>::BondParamIncorrect);

            let packages = BondUnitPackageRegistry::<T>::get(&bond, &caller);
            // how many bond units does the caller have
            let total_bond_units: BondUnitAmount = packages.iter()
            .map(|package| package.bond_units)
            .sum();


            ensure!(total_bond_units>=lot.bond_units && lot.bond_units>0, Error::<T>::BondParamIncorrect );


            // all lots of the caller.
            let mut lots: Vec<_> = BondUnitPackageLot::<T>::get(&bond, &caller);
            // purge expired lots
            lots.retain(|lot| !lot.is_expired(now) );

            let total_bond_units_inlot: BondUnitAmount = lots.iter().map(|lot| lot.bond_units).sum();
            // prevent new bid if the caller doesn't have enough bond units
            ensure!(total_bond_units>= total_bond_units_inlot+lot.bond_units, Error::<T>::BondParamIncorrect);

            lots.push(
                lot.clone()
            );
            // save  lots
            BondUnitPackageLot::<T>::insert(&bond, &caller, lots);
            Self::deposit_event(RawEvent::BondSaleLotBid(caller, bond, lot ));
            Ok(())
        }

        /// Method: bond_unit_lot_settle(origin, bond: BondId,bondholder: AccountId, lot: BondUnitSaleLotStruct)
        /// Arguments: origin: AccountId - bond unit bondholder
        ///            bond: BondId - bond identifier
        ///            bondholder: Current bondholder of of bond
        ///            lot: BondUnitSaleLotStruct - lot data
        /// Access: Bond bondholder
        ///
        /// Buy the lot created by bond_unit_lot_bid call
        #[weight = 5_000]
        fn bond_unit_lot_settle(origin, bond: BondId, bondholder: T::AccountId, lot: BondUnitSaleLotStructOf<T>)->DispatchResult{
            let caller = ensure_signed(origin)?;
            ensure!(Self::account_is_investor(&caller), Error::<T>::AccountNotAuthorized);
            let now = <pallet_timestamp::Module<T>>::get();
            // prevent expired lots sales
            ensure!(!lot.is_expired( now ), Error::<T>::BondParamIncorrect);

            ensure!(lot.new_bondholder == Default::default() || lot.new_bondholder == caller, Error::<T>::BondParamIncorrect);
            let balance = Self::balance_everusd(&caller);
            // ensure caller has enough tokens on its balance
            ensure!(lot.amount <= balance , Error::<T>::BondParamIncorrect);

            BondUnitPackageLot::<T>::try_mutate(&bond, &bondholder, |lots|->DispatchResult{
                if let Some(index) = lots.iter().position(|item| item==&lot ){
                     lots.remove( index );
                     if !lots.is_empty() {
                        // purge expired lots
                        lots.retain( |item| !item.is_expired( now ) );
                     }
                     // @TODO optimize out access to balances
                     BondRegistry::<T>::mutate(bond, |mut item|{
                        Self::calc_and_store_bond_coupon_yield(&bond, &mut item, now);
                        Self::request_coupon_yield(&bond, &mut item, &bondholder);
                        Self::request_coupon_yield(&bond, &mut item, &caller);
                     });

                     let mut from_packages = BondUnitPackageRegistry::<T>::get(&bond, &bondholder);
                     // @TESTME try to compare sort performance with binaryheap
                     from_packages.sort_by_key(|package| core::cmp::Reverse(package.bond_units));
                     let mut to_packages = BondUnitPackageRegistry::<T>::get(&bond, &caller);

                     // transfer lot.bond_units from bondholder to caller

                     // @TODO design as separate function
                     let mut lot_bond_units = lot.bond_units;
                     while lot_bond_units > 0 {
                           // last element has smallest number of bond units
                           let mut last = from_packages.pop().ok_or( Error::<T>::BondParamIncorrect )?;
                           let (bond_units, acquisition, coupon_yield) = if last.bond_units > lot_bond_units {
                                last.bond_units -= lot_bond_units;
                                let bond_units = lot_bond_units;
                                let acquisition = last.acquisition;
                                lot_bond_units = 0;
                                from_packages.push(
                                    last
                                );
                                (bond_units, acquisition,  0)
                           }else{
                                lot_bond_units-=last.bond_units;
                                (last.bond_units, last.acquisition, last.coupon_yield)
                           };

                           to_packages.push(
                                BondUnitPackage{
                                     bond_units,
                                     acquisition,
                                     coupon_yield,
                                }
                           );
                     }
                     from_packages.shrink_to_fit();
                     // store new packages
                     BondUnitPackageRegistry::<T>::insert(&bond, &bondholder, from_packages);
                     BondUnitPackageRegistry::<T>::insert(&bond, &caller, to_packages);

                     // pay off deal
                     Self::balance_sub(&caller, lot.amount)?;
                     Self::balance_add(&bondholder, lot.amount)?;
                     Self::deposit_event(RawEvent::BondSaleLotSettle(caller, bond, lot ));
                     Ok(())
                }else{
                    Err(Error::<T>::BondParamIncorrect.into())
                }
            })
        }
    }
}

impl<T: Trait> Module<T> {
    /// Method: account_is_master(acc: &T::AccountId) -> bool
    /// Arguments: acc: AccountId - checked account id
    ///
    /// Checks if the acc has global Master role
    pub fn account_is_master(acc: &T::AccountId) -> bool {
        AccountRegistry::<T>::contains_key(acc)
            && (AccountRegistry::<T>::get(acc).roles & MASTER_ROLE_MASK != 0)
    }

    /// Method: account_is_custodian(acc: &T::AccountId) -> bool
    /// Arguments: acc: AccountId - checked account id
    ///
    /// Checks if the acc has global Custodian role
    pub fn account_is_custodian(acc: &T::AccountId) -> bool {
        AccountRegistry::<T>::contains_key(acc)
            && (AccountRegistry::<T>::get(acc).roles & CUSTODIAN_ROLE_MASK != 0)
    }

    /// Method: account_is_issuer(acc: &T::AccountId) -> bool
    /// Arguments: acc: AccountId - checked account id
    ///
    /// Checks if the acc has global Issuer role
    pub fn account_is_issuer(acc: &T::AccountId) -> bool {
        AccountRegistry::<T>::contains_key(acc)
            && (AccountRegistry::<T>::get(acc).roles & ISSUER_ROLE_MASK != 0)
    }

    /// Method: account_is_investor(acc: &T::AccountId) -> bool
    /// Arguments: acc: AccountId - checked account id
    ///
    /// Checks if the acc has global Investor role
    pub fn account_is_investor(acc: &T::AccountId) -> bool {
        AccountRegistry::<T>::contains_key(acc)
            && (AccountRegistry::<T>::get(acc).roles & INVESTOR_ROLE_MASK != 0)
    }

    /// Method: account_is_auditor(acc: &T::AccountId) -> bool
    /// Arguments: acc: AccountId - checked account id
    ///
    /// Checks if the acc has global Auditor role
    pub fn account_is_auditor(acc: &T::AccountId) -> bool {
        AccountRegistry::<T>::contains_key(acc)
            && (AccountRegistry::<T>::get(acc).roles & AUDITOR_ROLE_MASK != 0)
    }

    /// Method: account_is_manager(acc: &T::AccountId) -> bool
    /// Arguments: acc: AccountId - checked account id
    ///
    /// Checks if the acc has global Manager role
    pub fn account_is_manager(acc: &T::AccountId) -> bool {
        AccountRegistry::<T>::contains_key(acc)
            && (AccountRegistry::<T>::get(acc).roles & MANAGER_ROLE_MASK != 0)
    }

    /// Method: account_is_impact_reporter(acc: &T::AccountId) -> bool
    /// Arguments: acc: AccountId - checked account id
    ///
    /// Checks if the acc has global Impact Reporter role
    pub fn account_is_impact_reporter(acc: &T::AccountId) -> bool {
        AccountRegistry::<T>::contains_key(acc)
            && (AccountRegistry::<T>::get(acc).roles & IMPACT_REPORTER_ROLE_MASK != 0)
    }

    /// Method: account_token_mint_burn_allowed(acc: &T::AccountId) -> bool
    /// Arguments: acc: AccountId - checked account id
    ///
    /// Checks if the acc can create burn and mint tokens requests
    pub fn account_token_mint_burn_allowed(acc: &T::AccountId) -> bool {
        const ALLOWED_ROLES_MASK: u8 = INVESTOR_ROLE_MASK | ISSUER_ROLE_MASK;
        AccountRegistry::<T>::contains_key(acc)
            && (AccountRegistry::<T>::get(acc).roles & ALLOWED_ROLES_MASK != 0)
    }
    /// Method: balance_everusd(acc: &T::AccountId) -> EverUSDBalance
    /// Arguments: acc: AccountId - account id
    ///
    /// Returns account's balance as the number of EverUSD tokens
    pub fn balance_everusd(acc: &T::AccountId) -> EverUSDBalance {
        BalanceEverUSD::<T>::get(acc)
    }

    /// Method: total_supply() -> EverUSDBalance
    /// Arguments: none
    ///
    /// Returns the total number of EverUSD tokens supplied by the custodian
    #[cfg(test)]
    pub fn total_supply() -> EverUSDBalance {
        TotalSupplyEverUSD::get()
    }

    /// Method: get_bond(bond: BondId) -> bond: BondId) -> BondStruct
    /// Arguments: bond: BondId - bond unique identifier
    ///
    ///  Returns bond structure if found
    pub fn get_bond(bond: &BondId) -> BondStructOf<T> {
        BondRegistry::<T>::get(bond)
    }

    #[cfg(test)]
    pub fn bond_check_invariant(bond: &BondId) -> bool {
        let (bond_units, coupon_yield) = BondUnitPackageRegistry::<T>::iter_prefix_values(bond)
            .fold((0, 0), |acc, packages| {
                packages.iter().fold(acc, |acc, package| {
                    (acc.0 + package.bond_units, acc.1 + package.coupon_yield)
                })
            });
        let bond = BondRegistry::<T>::get(bond);

        bond.issued_amount == bond_units && bond.coupon_yield == coupon_yield
    }

    #[cfg(test)]
    pub fn bond_holder_packages(bond: &BondId, bondholder: &T::AccountId) -> Vec<BondUnitPackage> {
        BondUnitPackageRegistry::<T>::get(bond, bondholder)
    }

    pub fn bond_impact_data(bond: &BondId) -> Vec<BondImpactReportStruct> {
        BondImpactReport::get(bond)
    }

    #[allow(dead_code)]
    fn purge_expired_bondunit_lots(_before: T::Moment) {
        //@TODO remove or implement
    }

    #[cfg(test)]
    fn bond_packages(id: &BondId) -> std::collections::HashMap<T::AccountId, Vec<BondUnitPackage>>
    where
        <T as frame_system::Trait>::AccountId: std::hash::Hash,
    {
        BondUnitPackageRegistry::<T>::iter_prefix(id).collect()
    }

    /// Same as BondRegistry::<T>::mutate(bond, f).
    /// unlike BondRegistry::<T>::mutate(bond, f) `with_bond` doesn't write to storage
    /// if f call returns error or bond key doesn't exist in the registry
    fn with_bond<F: FnOnce(&mut BondStructOf<T>) -> DispatchResult>(
        bond: &BondId,
        f: F,
    ) -> DispatchResult {
        ensure!(
            BondRegistry::<T>::contains_key(bond),
            Error::<T>::BondNotFound
        );

        let mut item = BondRegistry::<T>::get(bond);
        f(&mut item)?;
        BondRegistry::<T>::insert(bond, item);
        Ok(())
    }

    /// Increase account balance by `amount` EverUSD
    fn balance_add(who: &T::AccountId, amount: EverUSDBalance) -> DispatchResult {
        BalanceEverUSD::<T>::try_mutate(who, |balance| -> DispatchResult {
            *balance = balance
                .checked_add(amount)
                .ok_or(Error::<T>::BalanceOverdraft)?;
            Ok(())
        })
    }

    /// Decrease account balance by `amount` EverUSD
    fn balance_sub(who: &T::AccountId, amount: EverUSDBalance) -> DispatchResult {
        BalanceEverUSD::<T>::try_mutate(who, |balance| -> DispatchResult {
            *balance = balance
                .checked_sub(amount)
                .ok_or(Error::<T>::BalanceOverdraft)?;
            Ok(())
        })
    }

    /// Deletes  expired burn request from the queue
    fn purge_expired_burn_requests(before: T::Moment) {
        let to_purge: Vec<_> = BurnRequestEverUSD::<T>::iter()
            .filter(|(_, request)| request.is_expired(before))
            .map(|(acc, _)| acc)
            .take(MAX_PURGE_REQUESTS)
            .collect();

        for acc in to_purge {
            BurnRequestEverUSD::<T>::remove(acc);
        }
    }

    /// Deletes  expired mint request from the queue
    fn purge_expired_mint_requests(before: T::Moment) {
        let to_purge: Vec<_> = MintRequestEverUSD::<T>::iter()
            .filter(|(_, request)| request.is_expired(before))
            .map(|(acc, _)| acc)
            .take(MAX_PURGE_REQUESTS)
            .collect();

        for acc in to_purge {
            MintRequestEverUSD::<T>::remove(acc);
        }
    }

    pub fn get_coupon_yields(id: &BondId) -> Vec<PeriodYield> {
        BondCouponYield::get(id)
    }
    /// Calculate bond coupon yield
    /// Store values in BondCouponYield value
    /// Update bond bond_credit value to the current coupon yield
    /// Returns true if new periods was processed, false otherwise
    /// common complexity O(N), where N is the number of issued bond unit packages
    fn calc_and_store_bond_coupon_yield(
        id: &BondId,
        bond: &mut BondStructOf<T>,
        now: <T as pallet_timestamp::Trait>::Moment,
    ) -> bool {
        let (_, period) = ensure_active!(bond.time_passed_after_activation(now), false);
        // here is current pay period
        let period = period as usize;
        let mut bond_yields = BondCouponYield::get(id);
        // get last accrued coupon yield
        let mut total_yield = bond_yields
            .last()
            .map(|period_yield| period_yield.total_yield)
            .unwrap_or(0);
        // period should be ended up before we can calc it
        if bond_yields.len() >= period {
            // term hasn't come yet (if period=0 )
            // or current period has been calculated
            bond.bond_credit = total_yield;
            return false;
        }

        // @TODO refactor. use `mutate` method instead  of get+insert
        let reports = BondImpactReport::get(id);
        assert!(reports.len() + 1 >= period);

        while bond_yields.len() < period {
            // index - accrued period number
            let index = bond_yields.len();
            let interest_rate = if index == 0 {
                bond.inner.interest_rate_start_period_value
            } else if reports[index - 1].signed {
                bond.interest_rate(reports[index - 1].impact_data)
            } else {
                min(
                    bond_yields[index - 1].interest_rate
                        + bond.inner.interest_rate_penalty_for_missed_report,
                    bond.inner.interest_rate_margin_cap,
                )
            };

            let package_yield = bond.inner.bond_units_base_price / 1000
                * interest_rate as EverUSDBalance
                / INTEREST_RATE_YEAR;

            // calculate yield for period equal to bond_yields.len()
            let period_coupon_yield: EverUSDBalance =
                match bond.period_desc(index as BondPeriodNumber) {
                    Some(period_desc) => {
                        // for every bond bondholder
                        BondUnitPackageRegistry::<T>::iter_prefix(id)
                            .map(|(_bondholder, packages)| {
                                // for every package
                                packages
                                    .iter()
                                    .map(|package| {
                                        // @TODO use checked arithmetics
                                        package_yield
                                            * package.bond_units as EverUSDBalance
                                            * period_desc.duration(package.acquisition)
                                                as EverUSDBalance
                                            / 100
                                    })
                                    .sum::<EverUSDBalance>()
                            })
                            .sum()
                    }
                    None => {
                        // @TODO  it's best panic instead of return false
                        return false;
                    }
                };
            let coupon_yield = min(bond.bond_debit, total_yield);
            total_yield += period_coupon_yield;

            bond_yields.push(PeriodYield {
                total_yield,
                coupon_yield_before: coupon_yield,
                interest_rate,
            });
            Self::deposit_event(RawEvent::BondCouponYield(*id, total_yield));
        }
        // save current liability in bond_credit field
        bond.bond_credit = total_yield;
        BondCouponYield::insert(id, bond_yields);
        if bond.state == BondState::BANKRUPT && bond.get_debt() == 0 {
            // restore good status
            bond.state = BondState::ACTIVE;
        }

        Self::deposit_event(RawEvent::BondCouponYield(*id, total_yield));
        true
    }

    /// Redeem bond units,  get principal value, and coupon yield in the balance
    pub fn redeem_bond_units(
        id: &BondId,
        bond: &mut BondStructOf<T>,
        bondholder: &T::AccountId,
    ) -> EverUSDBalance {
        let packages = BondUnitPackageRegistry::<T>::take(id, &bondholder);

        let bond_yields = BondCouponYield::get(id);
        assert!(!bond_yields.is_empty());
        // calc coupon yield
        let mut payable: EverUSDBalance = bond_yields
            .iter()
            .enumerate()
            .map(|(i, bond_yield)| {
                let period_desc = bond.period_desc(i as BondPeriodNumber).unwrap();
                let package_yield = bond.inner.bond_units_base_price / 1000
                    * bond_yield.interest_rate as EverUSDBalance
                    / INTEREST_RATE_YEAR;
                packages
                    .iter()
                    .map(|package| {
                        package_yield
                            * package.bond_units as EverUSDBalance
                            * period_desc.duration(package.acquisition) as EverUSDBalance
                            / 100
                    })
                    .sum::<EverUSDBalance>()
            })
            .sum::<EverUSDBalance>();

        let (bond_units, paid_yield): (BondUnitAmount, EverUSDBalance) =
            packages.iter().fold((0, 0), |acc, package| {
                (acc.0 + package.bond_units, acc.1 + package.coupon_yield)
            });
        // substrate paid coupon
        payable -= paid_yield;
        // add principal value
        payable += bond.par_value(bond_units);
        bond.coupon_yield += payable;

        Self::balance_add(bondholder, payable).unwrap();

        payable
    }

    ///
    pub fn request_coupon_yield(
        id: &BondId,
        bond: &mut BondStructOf<T>,
        bondholder: &T::AccountId,
    ) -> EverUSDBalance {
        if bond.bond_credit == 0 || bond.bond_debit == bond.coupon_yield {
            return 0;
        }

        let bond_yields = BondCouponYield::get(id);
        assert!(!bond_yields.is_empty());

        let current_coupon_yield = min(bond.bond_debit, bond.bond_credit);
        // @TODO replace with `mutate` method
        let mut last_bondholder_coupon_yield = BondLastCouponYield::<T>::get(id, bondholder);
        assert!(current_coupon_yield >= last_bondholder_coupon_yield.coupon_yield);
        assert!(bond_yields.len() > last_bondholder_coupon_yield.period_num as usize);

        if last_bondholder_coupon_yield.coupon_yield == current_coupon_yield {
            // no more accrued coupon yield
            return 0;
        }
        let mut payable = 0;

        for (i, bond_yield) in bond_yields
            .iter()
            .enumerate()
            .skip(last_bondholder_coupon_yield.period_num as usize)
        {
            let instalment = if i == bond_yields.len() - 1 {
                current_coupon_yield - last_bondholder_coupon_yield.coupon_yield
            } else {
                let cy = last_bondholder_coupon_yield.coupon_yield;
                last_bondholder_coupon_yield.coupon_yield = bond_yields[i + 1].total_yield;
                bond_yields[i + 1].total_yield - cy
            };

            let package_yield = bond.inner.bond_units_base_price / 1000
                * bond_yield.interest_rate as EverUSDBalance
                / INTEREST_RATE_YEAR;

            if instalment > 0 {
                let period_desc = bond.period_desc(i as BondPeriodNumber).unwrap();
                let accrued_yield = bond_yield.total_yield
                    - if i == 0 {
                        0
                    } else {
                        bond_yields[i - 1].total_yield
                    };

                assert!(instalment <= accrued_yield);

                BondUnitPackageRegistry::<T>::mutate(id, &bondholder, |packages| {
                    for package in packages.iter_mut() {
                        let accrued = package_yield
                            * package.bond_units as EverUSDBalance
                            * period_desc.duration(package.acquisition) as EverUSDBalance
                            / 100;

                        let package_coupon_yield = if instalment == accrued_yield {
                            accrued
                        } else {
                            (instalment as u128 * accrued as u128 / accrued_yield as u128) as u64
                        };
                        payable += package_coupon_yield;
                        package.coupon_yield += package_coupon_yield;
                        assert!(package.coupon_yield <= accrued);
                    }
                });
            }
        }
        bond.coupon_yield += payable;
        last_bondholder_coupon_yield.period_num = (bond_yields.len() - 1) as BondPeriodNumber;

        BondLastCouponYield::<T>::insert(id, &bondholder, last_bondholder_coupon_yield);
        Self::balance_add(bondholder, payable).unwrap();
        payable
    }

    /// Returns effective coupon interest rate for `period`
    /// common complexity O(1), O(N) in worst case then no reports was released
    #[cfg(test)]
    pub fn calc_bond_interest_rate(
        bond: &BondStructOf<T>,
        reports: &[BondImpactReportStruct],
        period: usize,
    ) -> bond::BondInterest {
        assert!(reports.len() >= period);

        let mut missed_periods = 0;
        let mut interest: bond::BondInterest = bond.inner.interest_rate_start_period_value;
        for report in reports[0..period].iter().rev() {
            if report.signed {
                interest = bond.interest_rate(report.impact_data);
                break;
            }
            missed_periods += 1;
        }

        min(
            bond.inner.interest_rate_margin_cap,
            interest + missed_periods * bond.inner.interest_rate_penalty_for_missed_report,
        )
    }
    /// Checks if a report comes at the right time
    fn is_report_in_time(
        bond: &BondStructOf<T>,
        now: <T as pallet_timestamp::Trait>::Moment,
        period: BondPeriodNumber,
    ) -> bool {
        // get  the number of seconds from bond activation
        let (moment, _current_period) =
            ensure_active!(bond.time_passed_after_activation(now), false);
        // impact report should be sent and signed not early than interval for send report begins
        // and not later than current period ends
        bond.period_desc(period)
            .map(|desc| moment >= desc.impact_data_send_period && moment < desc.payment_period)
            .unwrap_or(false)
    }

    #[cfg(test)]
    fn set_impact_data(
        bond: &BondId,
        period: BondPeriodNumber,
        impact_data: u64,
    ) -> DispatchResult {
        BondImpactReport::try_mutate(&bond, |reports| -> DispatchResult {
            let index = period as usize;

            reports[index].signed = true;
            reports[index].impact_data = impact_data;
            reports[index].create_date = 1; //dirty hack. test require nonzero value

            Ok(())
        })
    }

    #[cfg(test)]
    fn evercity_balance() -> bond::ledger::EvercityBalance {
        let account: EverUSDBalance = BalanceEverUSD::<T>::iter_values().sum();
        let bond_fund: EverUSDBalance = BondRegistry::<T>::iter_values()
            .map(|bond| bond.bond_debit - bond.coupon_yield)
            .sum();

        bond::ledger::EvercityBalance {
            supply: TotalSupplyEverUSD::get(),
            account,
            bond_fund,
        }
    }
}
