#![cfg_attr(not(feature = "std"), no_std)]
use core::cmp::{Eq, PartialEq};
use frame_support::{
    codec::{Decode, Encode},
    debug, decl_error, decl_event, decl_module, decl_storage,
    dispatch::Vec,
    dispatch::{DispatchError, DispatchResult},
    ensure,
    sp_runtime::RuntimeDebug,
};
use frame_system::ensure_signed;

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_core::H256;

use sp_runtime::traits::{SaturatedConversion, UniqueSaturatedInto};

pub trait Trait: frame_system::Trait + pallet_timestamp::Trait {
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;
}

pub type Result<T> = core::result::Result<T, DispatchError>;
#[derive(Clone, Copy, Default, Encode, Decode, RuntimeDebug)]
pub struct BondId([u8; 8]);

impl PartialEq for BondId {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Eq for BondId {}

impl core::ops::Deref for BondId {
    type Target = [u8; 8];
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

//impl WrapperTypeEncode for BondId {}

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

pub const MASTER_ROLE_MASK: u8 = 1u8;
pub const CUSTODIAN_ROLE_MASK: u8 = 2u8;
pub const EMITENT_ROLE_MASK: u8 = 4u8;
pub const INVESTOR_ROLE_MASK: u8 = 8u8;
pub const AUDITOR_ROLE_MASK: u8 = 16u8;
pub const MANAGER_ROLE_MASK: u8 = 32u8;
pub const IMPACT_REPORTER_ROLE_MASK: u8 = 64u8;

pub const ALL_ROLES_MASK: u8 = MASTER_ROLE_MASK
    | CUSTODIAN_ROLE_MASK
    | EMITENT_ROLE_MASK
    | INVESTOR_ROLE_MASK
    | AUDITOR_ROLE_MASK
    | MANAGER_ROLE_MASK
    | IMPACT_REPORTER_ROLE_MASK;

#[inline]
pub const fn is_roles_correct(roles: u8) -> bool {
    // max value of any roles combinations
    roles <= ALL_ROLES_MASK
}

pub const EVERUSD_DECIMALS: u64 = 10;
pub const EVERUSD_MAX_MINT_AMOUNT: EverUSDBalance = 10_000_000_000_000; //1_000_000_000u64 * EVERUSD_DECIMALS;

pub const MIN_RESET_PERIOD: BondPeriod = DAY_DURATION * 7;

pub const TOKEN_BURN_DEADLINE: u32 = DAY_DURATION as u32 * 7 * 1000;
pub const TOKEN_MINT_DEADLINE: u32 = DAY_DURATION as u32 * 7 * 1000;

const MAX_PURGE_REQUESTS: usize = 100;

/// Evercity project types
/// All these types must be put in CUSTOM_TYPES part of config for polkadot.js
/// to be correctly presented in DApp

pub type EverUSDBalance = u64;

/// Structures, specific for each role
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, Clone, Default, RuntimeDebug)]
pub struct EvercityAccountStructT<Moment> {
    pub roles: u8,
    pub identity: u64,
    pub create_time: Moment,
}

type EvercityAccountStructOf<T> = EvercityAccountStructT<<T as pallet_timestamp::Trait>::Moment>;

//impl<T:Trait> EncodeLike<(u8, u64, u64)> for EvercityAccountStruct<T> {}

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, Clone, Default, RuntimeDebug)]
pub struct MintRequestStruct<Moment> {
    pub amount: EverUSDBalance,
    pub deadline: Moment,
}

type MintRequestStructOf<T> = MintRequestStruct<<T as pallet_timestamp::Trait>::Moment>;
// impl EncodeLike<EverUSDBalance> for MintRequestStruct {}

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, Clone, Default, RuntimeDebug)]
pub struct BurnRequestStruct<Moment> {
    pub amount: EverUSDBalance,
    pub deadline: Moment,
}
// impl EncodeLike<EverUSDBalance> for BurnRequestStruct {}
type BurnRequestStructOf<T> = BurnRequestStruct<<T as pallet_timestamp::Trait>::Moment>;

/// Bond state
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, Clone, RuntimeDebug, PartialEq)]
pub enum BondState {
    PREPARE,
    BOOKING,
    ACTIVE,
    BANKRUPT,
    FINISHED,
}

impl Default for BondState {
    fn default() -> Self {
        BondState::PREPARE
    }
}

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, Clone, PartialEq, RuntimeDebug)]
pub enum BondPayPeriod {
    DAY,
    WEEK,
    MONTH,
    QUARTER,
    HYEAR,
    YEAR,
    YEAR2,
    YEAR5,
}

impl Default for BondPayPeriod {
    fn default() -> Self {
        BondPayPeriod::DAY
    }
}

impl BondPayPeriod {
    #[allow(dead_code)]
    fn days(&self) -> u32 {
        match self {
            Self::DAY => 1,
            Self::WEEK => 7,
            Self::MONTH => 30,
            Self::QUARTER => 91,
            Self::HYEAR => 182,
            Self::YEAR => 365,
            Self::YEAR2 => 730,
            Self::YEAR5 => 1825,
        }
    }
}

/// Bond period parametes type, seconds
type BondPeriod = u32;
/// The number of Bond units,
type BondUnitAmount = u32;
/// Annual coupon interest rate as 1/100000 of par value
type BondInterest = u32;

const MIN_BOND_DURATION: u32 = 1; // 1  is a minimal bond period
const DAY_DURATION: u32 = 8760; // seconds in 1 DAY

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, Clone, Default, PartialEq, RuntimeDebug)]
pub struct BondInnerStruct<Moment> {
    //pub ticker: [u8; 8],
    pub data_hash_main: H256,
    pub data_hash_legal: H256,
    pub data_hash_finance: H256,
    pub data_hash_tech: H256,
    // Not used in MVP
    pub bond_category: u8,

    // bond impact parameters
    // Can take:
    //   0 - the amount of CO2.
    //   1 - electric power generated
    pub impact_data_type: u8,
    pub impact_baseline: u64,
    // Coupon interest regulatory options
    pub impact_max_deviation_cap: u64,
    pub impact_max_deviation_floor: u64,
    // increase interest rate if
    pub missed_report_penalty: u16,

    // base coupon interest rate, ppm
    pub bond_base_interest_rate: BondInterest,
    // max coupon interest rate, ppm
    pub bond_interest_margin_cap: BondInterest,
    // min coupon interest rate, ppm
    pub bond_interest_margin_floor: BondInterest,
    // interest rate from activation up to start_period, ppm
    pub start_period_interest_rate: BondInterest,

    // Days from activation when effective interest rate is equal to start_period_interest_rate
    pub start_period: BondPeriod,
    //
    pub reset_period: BondPeriod,
    // seconds after every reset_period when Emitent should pay off coupon interests
    pub interest_pay_period: BondPeriod,
    // mincap_amount bond units should be raised up to this date
    // otherwise bond will be withdrawn
    pub mincap_deadline: Moment,
    // seconds before end of a period
    pub report_period: BondPeriod,
    // the number of periods from active_start_date until maturity date
    // bond maturity period = start_period + bond_duration * reset_period
    pub bond_duration: u32,
    // seconds from maturity date until full repayment
    pub bond_finishing_period: BondPeriod,
    // minimal amount of issued bond units
    pub mincap_amount: BondUnitAmount,
    // maximal amount of issued bond units
    pub maxcap_amount: BondUnitAmount,
    // bond unit par value
    pub base_price: BondUnitAmount,
}

impl<Moment> BondInnerStruct<Moment> {
    /// Checks if other bond has the same financial properties
    pub fn is_financial_options_eq(&self, other: &Self) -> bool {
        self.bond_category == other.bond_category
            && self.impact_data_type == other.impact_data_type
            && self.impact_baseline == other.impact_baseline
            && self.impact_max_deviation_cap == other.impact_max_deviation_cap
            && self.bond_base_interest_rate == other.bond_base_interest_rate
            && self.bond_interest_margin_cap == other.bond_interest_margin_cap
            && self.bond_interest_margin_floor == other.bond_interest_margin_floor
            && self.interest_pay_period == other.interest_pay_period
            && self.report_period == other.report_period
            && self.bond_duration == other.bond_duration
            && self.bond_finishing_period == other.bond_finishing_period
            && self.mincap_amount == other.mincap_amount
            && self.maxcap_amount == other.maxcap_amount
            && self.base_price == other.base_price
    }
    /// Checks if bond data is valid
    pub fn is_valid(&self) -> bool {
        self.mincap_amount > 0
            && self.maxcap_amount >= self.mincap_amount
            && self.reset_period >= MIN_RESET_PERIOD
            && self.report_period <= self.reset_period
            && self.interest_pay_period <= self.reset_period
            && self.base_price > 0
            && self.bond_base_interest_rate >= self.bond_interest_margin_floor
            && self.bond_base_interest_rate <= self.bond_interest_margin_cap
            && self.impact_baseline <= self.impact_max_deviation_cap
            && self.impact_baseline >= self.impact_max_deviation_floor
            && self.bond_duration >= MIN_BOND_DURATION
            && self.bond_category == 0 // MVP accept only  0 - all investor categories allowed
    }
}

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, Clone, Default, RuntimeDebug)]
pub struct BondStruct<AccountId, Moment> {
    pub inner: BondInnerStruct<Moment>,
    // Bond issuer
    pub emitent: AccountId,
    // Auxiliary roles
    pub manager: AccountId,
    pub auditor: AccountId,
    pub impact_reporter: AccountId,
    // Transition data
    // Total amount of issued bond units
    pub issued_amount: BondUnitAmount,

    // Timestamps
    pub booking_start_date: Moment,
    pub active_start_date: Moment,
    pub creation_date: Moment,
    // Bond current state
    pub state: BondState,
    // Bond state modification/change time
    pub last_updated: Moment,
    pub period: u16,

    // Bond ledger
    pub bond_debit: EverUSDBalance,
    // Emitent liabilities
    pub bond_credit: EverUSDBalance,
}

impl<AccountId, Moment> BondStruct<AccountId, Moment> {
    /// Returns nominal value of unit_amount Bond units
    #[inline]
    fn par_value(&self, unit_amount: BondUnitAmount) -> EverUSDBalance {
        unit_amount as EverUSDBalance * self.inner.base_price as EverUSDBalance
    }
    /// Returns bond liabilities
    #[allow(dead_code)]
    fn get_debt(&self) -> EverUSDBalance {
        if self.bond_credit > self.bond_debit {
            self.bond_credit - self.bond_debit
        } else {
            0
        }
    }
    /// Returns the number of  tokens available for emitent
    fn get_balance(&self) -> EverUSDBalance {
        if self.bond_debit > self.bond_credit {
            self.bond_debit - self.bond_credit
        } else {
            0
        }
    }
    /// Increase bond fund
    fn increase(&mut self, amount: EverUSDBalance) {
        self.bond_credit += amount;
        self.bond_debit += amount;
    }
    /// Decrease bond fund
    fn decrease(&mut self, amount: EverUSDBalance) {
        self.bond_credit -= amount;
        self.bond_debit -= amount;
    }

    #[allow(dead_code)]
    /// Calculate coupon effective interest rate
    fn interest_rate(&self, impact_data: u64) -> BondInterest {
        let inner = &self.inner;

        if impact_data >= inner.impact_max_deviation_cap {
            inner.bond_interest_margin_floor
        } else if impact_data <= inner.impact_max_deviation_floor {
            inner.bond_interest_margin_cap
        } else if impact_data == inner.impact_baseline {
            inner.bond_base_interest_rate
        } else if impact_data > inner.impact_baseline {
            inner.bond_base_interest_rate
                - ((impact_data - inner.impact_baseline) as u128
                    * (inner.bond_base_interest_rate - inner.bond_interest_margin_floor) as u128
                    / (inner.impact_max_deviation_cap - inner.impact_baseline) as u128)
                    as BondInterest
        } else {
            inner.bond_base_interest_rate
                + ((inner.impact_baseline - impact_data) as u128
                    * (inner.bond_interest_margin_cap - inner.bond_base_interest_rate) as u128
                    / (inner.impact_baseline - inner.impact_max_deviation_floor) as u128)
                    as BondInterest
        }
    }
}

impl<AccountId, Moment: UniqueSaturatedInto<u64> + Copy> BondStruct<AccountId, Moment> {
    /// Returns current reset_period
    /// It takes 0 for date from active_start_date  up to active_start_date + start_period
    #[allow(dead_code)]
    fn period(&self, moment: Moment) -> u32 {
        let active_start_date: u64 = self.active_start_date.saturated_into::<u64>();
        let moment: u64 = moment.saturated_into::<u64>();

        if active_start_date >= moment {
            return 0;
        }
        // @TODO handle overflow  (upper limit of period is self.inner.bond_duration)

        let seconds_from_start = ((moment - active_start_date) / 1000) as u32;
        if seconds_from_start <= self.inner.start_period {
            return 0;
        }

        seconds_from_start / self.inner.reset_period
    }

    // @TODO limit upper value
    // fn is_active(&self, moment: Moment)->bool{
    //     moment >= self.active_start_date
    // }
}

#[derive(Encode, Decode, Clone, Default, PartialEq, RuntimeDebug)]
pub struct BondUnitPackage<Moment> {
    bond_units: BondUnitAmount,
    acquisition_date: Moment,
    // bearer: AccountId
}
type BondUnitPackageOf<T> = BondUnitPackage<<T as pallet_timestamp::Trait>::Moment>;
type BondInnerStructOf<T> = BondInnerStruct<<T as pallet_timestamp::Trait>::Moment>;
type BondStructOf<T> =
    BondStruct<<T as frame_system::Trait>::AccountId, <T as pallet_timestamp::Trait>::Moment>;

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, Clone, Default, RuntimeDebug)]
pub struct BondImpactReportStruct<Moment> {
    create_date: Moment,
    impact_data: u64,
    signed: bool,
}

type BondImpactReportStructOf<T> = BondImpactReportStruct<<T as pallet_timestamp::Trait>::Moment>;

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, Clone, Default, RuntimeDebug)]
pub struct BondUnitSaleLotStruct<AccountId, Moment> {
    deadline: Moment,
    new_bearer: AccountId,
    bond_units: BondUnitAmount,
    amount: EverUSDBalance,
}

type BondUnitSaleLotStructOf<T> = BondUnitSaleLotStruct<
    <T as frame_system::Trait>::AccountId,
    <T as pallet_timestamp::Trait>::Moment,
>;

decl_storage! {
    trait Store for Module<T: Trait> as Evercity {
        AccountRegistry
            get(fn account_registry)
            config(genesis_account_registry):
            map hasher(blake2_128_concat) T::AccountId => EvercityAccountStructOf<T>; //roles, identities, balances

        // Token balances storages.
        // Evercity tokens cannot be transferred
        // Only mint/burn by Custodian accounts, invested/redeemed by Investor, paid by Emitent, etc...
        TotalSupplyEverUSD
            get(fn total_supply_everusd):
            EverUSDBalance; // total supply of EverUSD token (u64)

        BalanceEverUSD
            get(fn balances_everusd):
            map hasher(blake2_128_concat) T::AccountId => EverUSDBalance;

        // Structure, created by Emitent or Investor to receive EverUSD on her balance
        // She pays USD to Custodian and Custodian confirms request, adding corresponding
        // amount to mint request creator's balance
        MintRequestEverUSD
            get(fn mint_request_everusd):
                map hasher(blake2_128_concat) T::AccountId => MintRequestStructOf<T>;

        // Same as MintRequest, but for burning EverUSD tokens, paying to creator in USD
        // In future these actions can require different data, so it's separate structure
        // than mint request
        BurnRequestEverUSD
            get(fn burn_request_everusd):
                map hasher(blake2_128_concat) T::AccountId => BurnRequestStructOf<T>;

        // Structure for storing bonds
        BondRegistry
            get(fn bond_registry):
                map hasher(blake2_128_concat) BondId => BondStructOf<T>;

        // Bearer's Bond units
        BondUnitPackageRegistry
            get(fn bond_unit_registry):
                double_map hasher(blake2_128_concat) BondId, hasher(blake2_128_concat) T::AccountId => Vec<BondUnitPackageOf<T>>;

        // Bond sale lots
        BondUnitPackageLot
            get(fn bond_unit_lots):
                double_map hasher(blake2_128_concat) BondId, hasher(blake2_128_concat) T::AccountId => Vec<BondUnitSaleLotStructOf<T>>;

        // Bond impact report storage
        BondImpactReport
            get(fn impact_reports):
                map hasher(blake2_128_concat) BondId => Vec<BondImpactReportStructOf<T>>;
    }
}

decl_event!(
    pub enum Event<T>
    where
        AccountId = <T as frame_system::Trait>::AccountId,
        // Moment = <T as pallet_timestamp::Trait>::Moment,
    {
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

        BurnRequestCreated(AccountId, EverUSDBalance),
        BurnRequestRevoked(AccountId, EverUSDBalance),
        BurnRequestConfirmed(AccountId, EverUSDBalance),
        BurnRequestDeclined(AccountId, EverUSDBalance),
        // Bond events
        BondAdded(AccountId, BondId),
        BondChanged(AccountId, BondId),
        BondRevoked(AccountId, BondId),
        BondReleased(AccountId, BondId),
        BondActivated(AccountId, BondId),
        BondWithdrawal(AccountId, BondId),
        BondImpactReportReceived(AccountId, BondId),
        BondFinished(AccountId, BondId),
        BondBankrupted(AccountId, BondId),

        BondWithdrawEverUSD(AccountId, BondId, EverUSDBalance),
        BondDepositEverUSD(AccountId, BondId, EverUSDBalance),

        BondSale(AccountId, BondId, u32),
        BondGiveBack(AccountId, BondId, u32),

        BondImpactReportIssued(AccountId, BondId),
        BondImpactReportSigned(AccountId, BondId),
    }
);

decl_error! {
    pub enum Error for Module<T: Trait> {
        NoneValue,

        BalanceOverdraft,

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

        /// Account already created one burn request, only one allowed at a time(to be changed in future)
        BurnRequestAlreadyExist,

        /// Mint request for given account doesnt exist
        BurnRequestDoesntExist,

        /// Incorrect parameters for mint request(miant amount > MAX_MINT_AMOUNT)
        BurnRequestParamIncorrect,

        /// Dublicate bond ticker
        BondAlreadyExists,

        /// Incorrect bond data
        BondParamIncorrect,

        /// Incorrect bond ticker
        BondNotFound
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

            Self::deposit_event(RawEvent::AccountDisable(_caller, who));
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

            //let _now = <frame_system::Module<T>>::block_number();
            let now: <T as pallet_timestamp::Trait>::Moment = <pallet_timestamp::Module<T>>::get();

            AccountRegistry::<T>::insert(&who,
                EvercityAccountStructT { roles: role, identity, create_time: now }
            );
            debug::error!("account_add_with_role_and_data: who={:?} when={:?}", who, now);

            Self::deposit_event(RawEvent::AccountAdd(_caller, who, role, identity));
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

            let mut account = AccountRegistry::<T>::get(&who);
            account.roles |= role;

            AccountRegistry::<T>::insert(&who,
                account
            );

            Self::deposit_event(RawEvent::AccountSet(_caller, who, role, identity));
            Ok(())
        }

        /// Token balances manipulation functions

        /// Creates mint request to mint given amount of tokens on address of caller(emitent or investor)
        #[weight = 15_000]
        fn token_mint_request_create_everusd(origin, amount_to_mint: EverUSDBalance) -> DispatchResult {
            let _caller = ensure_signed(origin)?;
            ensure!(Self::account_token_mint_burn_allowed(&_caller), Error::<T>::AccountNotAuthorized);
            ensure!(!MintRequestEverUSD::<T>::contains_key(&_caller), Error::<T>::MintRequestAlreadyExist);
            ensure!(amount_to_mint < EVERUSD_MAX_MINT_AMOUNT, Error::<T>::MintRequestParamIncorrect);

            let now: <T as pallet_timestamp::Trait>::Moment = <pallet_timestamp::Module<T>>::get();
            let new_mint_request = MintRequestStruct {
                amount: amount_to_mint,
                deadline: now + TOKEN_MINT_DEADLINE.into(),
            };
            MintRequestEverUSD::<T>::insert(&_caller, new_mint_request);

            Self::deposit_event(RawEvent::MintRequestCreated(_caller, amount_to_mint));
            Ok(())
        }

        #[weight = 5_000]
        fn token_mint_request_revoke_everusd(origin) -> DispatchResult {
            let _caller = ensure_signed(origin)?;
            ensure!(MintRequestEverUSD::<T>::contains_key(&_caller), Error::<T>::MintRequestDoesntExist);
            let _amount = MintRequestEverUSD::<T>::get(&_caller).amount;
            MintRequestEverUSD::<T>::remove(&_caller);
            Self::deposit_event(RawEvent::MintRequestRevoked(_caller, _amount));
            Ok(())
        }

        /// Token balances manipulation functions
        #[weight = 15_000]
        fn token_mint_request_confirm_everusd(origin, who: T::AccountId, amount: EverUSDBalance) -> DispatchResult {
            let _caller = ensure_signed(origin)?;
            ensure!(Self::account_is_custodian(&_caller),Error::<T>::AccountNotAuthorized);
            ensure!(MintRequestEverUSD::<T>::contains_key(&who), Error::<T>::MintRequestDoesntExist);
            let mint_request = MintRequestEverUSD::<T>::get(&who);
            let now: <T as pallet_timestamp::Trait>::Moment = <pallet_timestamp::Module<T>>::get();
            ensure!(mint_request.deadline >= now, Error::<T>::MintRequestDoesntExist);

            // add tokens to user's balance and total supply of EverUSD
            let amount_to_add = mint_request.amount;
            ensure!(amount_to_add==amount,Error::<T>::MintRequestParamIncorrect );

            let total_supply = TotalSupplyEverUSD::get();
            let new_everusd_balance = BalanceEverUSD::<T>::get(&who) + amount_to_add;
            TotalSupplyEverUSD::set(total_supply + amount_to_add);
            BalanceEverUSD::<T>::insert(&who, new_everusd_balance);

            MintRequestEverUSD::<T>::remove(&who);
            Self::deposit_event(RawEvent::MintRequestConfirmed(who, amount_to_add));
            Self::purge_expired_mint_requests(now);
            Ok(())
        }

        #[weight = 5_000]
        fn token_mint_request_decline_everusd(origin, who: T::AccountId) -> DispatchResult {
            let _caller = ensure_signed(origin)?;
            ensure!(Self::account_is_custodian(&_caller),Error::<T>::AccountNotAuthorized);
            ensure!(MintRequestEverUSD::<T>::contains_key(&who), Error::<T>::MintRequestDoesntExist);
            let _amount = MintRequestEverUSD::<T>::get(&who).amount;
            MintRequestEverUSD::<T>::remove(&who);
            Self::deposit_event(RawEvent::MintRequestDeclined(_caller, _amount));
            Ok(())
        }

        /// Burn tokens
        /// Creates mint request to mint given amount of tokens on address of caller(emitent or investor)
        #[weight = 15_000]
        fn token_burn_request_create_everusd(origin, amount_to_burn: EverUSDBalance) -> DispatchResult {
            let _caller = ensure_signed(origin)?;
            ensure!(Self::account_token_mint_burn_allowed(&_caller), Error::<T>::AccountNotAuthorized);
            ensure!(!MintRequestEverUSD::<T>::contains_key(&_caller), Error::<T>::MintRequestAlreadyExist);

            let _current_balance = BalanceEverUSD::<T>::get(&_caller);
            ensure!(amount_to_burn <= _current_balance, Error::<T>::MintRequestParamIncorrect);
            let now: <T as pallet_timestamp::Trait>::Moment = <pallet_timestamp::Module<T>>::get();

            let new_burn_request = BurnRequestStruct {
                amount: amount_to_burn,
                deadline: now + TOKEN_BURN_DEADLINE.into(),
            };
            BurnRequestEverUSD::<T>::insert(&_caller, new_burn_request);

            Self::deposit_event(RawEvent::BurnRequestCreated(_caller, amount_to_burn));
            Ok(())
        }

        #[weight = 5_000]
        fn token_burn_request_revoke_everusd(origin) -> DispatchResult {
            let _caller = ensure_signed(origin)?;
            ensure!(BurnRequestEverUSD::<T>::contains_key(&_caller), Error::<T>::BurnRequestDoesntExist);
            let _amount = BurnRequestEverUSD::<T>::get(&_caller).amount;
            BurnRequestEverUSD::<T>::remove(&_caller);
            Self::deposit_event(RawEvent::BurnRequestRevoked(_caller, _amount));
            Ok(())
        }

        /// Token balances manipulation functions
        #[weight = 15_000]
        fn token_burn_request_confirm_everusd(origin, who: T::AccountId, amount: EverUSDBalance) -> DispatchResult {
            let _caller = ensure_signed(origin)?;
            ensure!(Self::account_is_custodian(&_caller),Error::<T>::AccountNotAuthorized);
            ensure!(BurnRequestEverUSD::<T>::contains_key(&who), Error::<T>::BurnRequestDoesntExist);
            let burn_request = BurnRequestEverUSD::<T>::get(&who);
            let now: <T as pallet_timestamp::Trait>::Moment = <pallet_timestamp::Module<T>>::get();
            ensure!(burn_request.deadline >= now, Error::<T>::BurnRequestDoesntExist);
            // remove tokens from user's balance and decrease total supply of EverUSD
            let amount_to_sub = burn_request.amount;
            // prevent unacceptable commit
            ensure!(amount_to_sub==amount, Error::<T>::MintRequestParamIncorrect );

            let total_supply = TotalSupplyEverUSD::get();
            let new_everusd_balance = BalanceEverUSD::<T>::get(&who) - amount_to_sub;
            TotalSupplyEverUSD::set(total_supply - amount_to_sub);
            BalanceEverUSD::<T>::insert(&who, new_everusd_balance);

            BurnRequestEverUSD::<T>::remove(&who);
            Self::deposit_event(RawEvent::BurnRequestConfirmed(who, amount_to_sub));
            Self::purge_expired_burn_requests(now);
            Ok(())
        }

        #[weight = 5_000]
        fn token_burn_request_decline_everusd(origin, who: T::AccountId) -> DispatchResult {
            let _caller = ensure_signed(origin)?;
            ensure!(Self::account_is_custodian(&_caller),Error::<T>::AccountNotAuthorized);
            ensure!(BurnRequestEverUSD::<T>::contains_key(&who), Error::<T>::BurnRequestDoesntExist);
            let _amount = BurnRequestEverUSD::<T>::get(&who).amount;
            BurnRequestEverUSD::<T>::remove(&who);
            Self::deposit_event(RawEvent::BurnRequestDeclined(_caller, _amount));
            Ok(())
        }

        // Bonds handling functions

        /// Method: bond_add_new(origin, origin, bond: BondId, body: BondInnerStruct)
        /// Arguments: origin: AccountId - transaction caller
        ///            bond: BondId - bond identifier
        ///            body: BondInnerStruct
        ///
        /// Create new bond.
        /// Access: only accounts with Emitent role
        #[weight = 20_000]
        fn bond_add_new(origin, bond: BondId, body: BondInnerStructOf<T> ) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            ensure!(Self::account_is_emitent(&caller),Error::<T>::AccountNotAuthorized);
            ensure!(body.is_valid(), Error::<T>::BondParamIncorrect );
            ensure!(!BondRegistry::<T>::contains_key(&bond), Error::<T>::BondAlreadyExists);

            let now: <T as pallet_timestamp::Trait>::Moment = <pallet_timestamp::Module<T>>::get();

            let item = BondStruct{
                    inner: body,

                    emitent: caller,
                    auditor: Default::default(),
                    manager: Default::default(),
                    impact_reporter: Default::default(),

                    issued_amount: 0,
                    booking_start_date: Default::default(),
                    active_start_date: Default::default(),
                    creation_date: now,
                    state: BondState::PREPARE,
                    last_updated: now,
                    bond_debit: Default::default(),
                    bond_credit: Default::default(),
                    period: 0,
            };
            BondRegistry::<T>::insert(&bond, item);
            Ok(())
        }

        /// Method: bond_set_auditor(origin, bond: BondId, acc: T::AccountId)
        /// Arguments: origin: AccountId - transaction caller, assigner
        ///            bond: BondId - bond identifier
        ///            acc: AccountId - assignee
        ///
        /// Assigns manager to the bond
        /// Access: only accounts with Master role
        #[weight = 5_000]
        fn bond_set_manager(origin, bond: BondId, acc: T::AccountId) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            // Bond Auxiliary roles can be set only by Master
            ensure!(Self::account_is_master(&caller), Error::<T>::AccountNotAuthorized);
            ensure!(Self::account_is_manager(&acc), Error::<T>::AccountRoleParamIncorrect);

            Self::with_bond(&bond, |item|{
                ensure!(
                    matches!(item.state, BondState::PREPARE ),
                    Error::<T>::BondParamIncorrect
                );
                item.last_updated = <pallet_timestamp::Module<T>>::get();
                item.manager = acc;
                Self::deposit_event(RawEvent::BondChanged(caller, bond ));
                Ok(())
            })
        }

        /// Method: bond_set_auditor(origin, bond: BondId, acc: T::AccountId)
        /// Arguments: origin: AccountId - transaction caller, assigner
        ///            bond: BondId - bond identifier
        ///            acc: AccountId - assignee
        ///
        /// Assigns auditor to the bond
        /// Access: only accounts with Master role
        #[weight = 5_000]
        fn bond_set_auditor(origin, bond: BondId, acc: T::AccountId) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            // Bond Auxiliary roles can be set only by Master
            ensure!(Self::account_is_master(&caller), Error::<T>::AccountNotAuthorized);
            ensure!(Self::account_is_auditor(&acc), Error::<T>::AccountRoleParamIncorrect);

            Self::with_bond(&bond, |item|{
                ensure!(
                    matches!(item.state, BondState::PREPARE | BondState::BOOKING),
                    Error::<T>::BondParamIncorrect
                );
                item.last_updated = <pallet_timestamp::Module<T>>::get();
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
            // Bond Auxiliary roles can be set only by Master
            ensure!(Self::account_is_master(&caller), Error::<T>::AccountNotAuthorized);
            ensure!(Self::account_is_impact_reporter(&acc), Error::<T>::AccountRoleParamIncorrect);

            Self::with_bond(&bond, |item|{
                item.last_updated = <pallet_timestamp::Module<T>>::get();
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
                // @TODO add ability to update the Bond in BOOKING state
                // preserving the base_price value
                ensure!(
                    matches!(item.state, BondState::PREPARE | BondState::BOOKING),
                    Error::<T>::BondParamIncorrect
                );
                ensure!(
                    item.emitent == caller || item.manager == caller ,
                    Error::<T>::AccountNotAuthorized
                );
                // Financial data shell not be changed after release
                if item.state == BondState::BOOKING {
                    ensure!( item.inner.is_financial_options_eq(&body), Error::<T>::BondParamIncorrect );
                }

                let now = <pallet_timestamp::Module<T>>::get();

                item.inner = body;
                item.last_updated = now;

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
                ensure!(item.state == BondState::PREPARE, Error::<T>::BondParamIncorrect);
                ensure!(item.inner.is_valid(), Error::<T>::BondParamIncorrect );

                let now = <pallet_timestamp::Module<T>>::get();
                // Ensure booking deadline is in the future
                ensure!(item.inner.mincap_deadline>now, Error::<T>::BondParamIncorrect );

                item.last_updated = now;
                item.booking_start_date = now;
                item.state = BondState::BOOKING;
                Self::deposit_event(RawEvent::BondReleased(caller, bond ));
                Ok(())
            })
        }

        /// Method: bond_unit_take_package(origin, bond: BondId, unit_amount: BondUnitAmount )
        /// Arguments: origin: AccountId - transaction caller
        ///            bond: BondId - bond identifier
        ///            unit_amount: BondUnitAmount - amount of bond units
        ///
        /// Bye bond units.
        /// Access: only accounts with Investor role
        // Investor loans tokens to the bond issuer by staking bond units
        #[weight = 10_000]
        fn bond_unit_take_package(origin, bond: BondId, unit_amount: BondUnitAmount ) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            ensure!(Self::account_is_investor(&caller), Error::<T>::AccountNotAuthorized);
            Self::with_bond(&bond, |item|{
                ensure!(
                    item.state==BondState::ACTIVE || item.state == BondState::BOOKING,
                    Error::<T>::BondParamIncorrect
                );

                let (result,ovf) = unit_amount.overflowing_add(item.issued_amount);
                ensure!(!ovf, Error::<T>::BondParamIncorrect);

                ensure!(
                    result <= item.inner.maxcap_amount,
                    Error::<T>::BondParamIncorrect
                );

                let package_value =  item.par_value( unit_amount ) ;

                Self::balance_sub(&caller, package_value)?;

                let now = <pallet_timestamp::Module<T>>::get();
                let mut packages = BondUnitPackageRegistry::<T>::get(&bond, &caller);
                packages.push(
                    BondUnitPackageOf::<T>{
                         bond_units: unit_amount,
                         acquisition_date: now,
                    }
                );
                BondUnitPackageRegistry::<T>::insert(&bond, &caller, packages);

                item.issued_amount = result;
                // increase assets and liabilities of the Bond
                item.increase( package_value );
                item.last_updated = now;
                Self::deposit_event(RawEvent::BondSale(caller.clone(), bond, unit_amount ));

                // @FIXME
                // According to the Design document
                // the Bond can be activated only by Master.
                // Disable instant activation.

                // Activate the Bond if it raised more than minimum
                // if item.state == BondState::BOOKING && item.issued_amount >= item.inner.mincap_amount {
                //     let now = <pallet_timestamp::Module<T>>::get();
                //     item.active_start_date = now;
                //     item.state = BondState::ACTIVE;
                //     item.timestamp = now;
                //     Self::deposit_event(RawEvent::BondActivated(caller, bond ));
                // }
                Ok(())
            })
        }

        /// Method: bond_unit_give_back_package(origin, bond: BondId, unit_amount: BondUnitAmount )
        /// Arguments: origin: AccountId - transaction caller
        ///            bond: BondId - bond identifier
        ///            unit_amount: BondUnitAmount - amount of bond units
        ///
        /// Gives back staked on presale bond units.
        /// Access: only accounts with Investor role who hold bond units
        // Investor gives back bond units and withdraw tokens
        #[weight = 10_000]
        fn bond_unit_give_back_package(origin, bond: BondId, unit_amount: BondUnitAmount ) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            ensure!(Self::account_is_investor(&caller), Error::<T>::AccountNotAuthorized);
            // Active Bond cannot be withdrawn
            Self::with_bond(&bond, |item|{
                ensure!(item.state == BondState::BOOKING, Error::<T>::BondParamIncorrect );
                ensure!(item.issued_amount<=unit_amount, Error::<T>::BondParamIncorrect);
                let package_value =  item.par_value( unit_amount ) ;
                ensure!(item.bond_credit>=package_value, Error::<T>::BondParamIncorrect);

                //@TODO make available to give back part of the package
                let mut packages = BondUnitPackageRegistry::<T>::get(&bond, &caller);
                ensure!(!packages.is_empty(), Error::<T>::BondParamIncorrect);
                if let Some(index) = packages.iter().position(|item| item.bond_units == unit_amount ){
                    packages.remove( index );
                    BondUnitPackageRegistry::<T>::insert(&bond, &caller, packages);
                }else{
                    return Err( Error::<T>::BondParamIncorrect.into() );
                }
                let now = <pallet_timestamp::Module<T>>::get();
                item.decrease( package_value );
                item.issued_amount -= unit_amount;
                item.last_updated = now;

                Self::balance_add(&caller, package_value )?;

                Self::deposit_event(RawEvent::BondGiveBack(caller, bond, unit_amount ));

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
                ensure!( item.state ==BondState::BOOKING, Error::<T>::BondParamIncorrect );
                // Ensure the Bond raises less then mincap_amount bond units
                ensure!(item.inner.mincap_amount > item.issued_amount, Error::<T>::BondParamIncorrect);
                ensure!(
                    item.emitent == caller || item.manager == caller || Self::account_is_master(&caller) ,
                    Error::<T>::AccountNotAuthorized
                );
                let now = <pallet_timestamp::Module<T>>::get();
                // Ensure booking deadline is in the future
                ensure!(item.inner.mincap_deadline<=now, Error::<T>::BondParamIncorrect );


                item.last_updated = now;
                item.state = BondState::PREPARE;

                //@TODO verify withdrawal all bookings and cancel bidding
                //@FIXME
                //ensure!(item.bond_credit == item.inner.base_price * item.issued_amount, Error::<T>::BondParamIncorrect);

                // for all bearers
                for (bearer, package) in BondUnitPackageRegistry::<T>::iter_prefix(&bond){
                      let bearer_total_amount: BondUnitAmount = package.iter()
                      .map(|item| item.bond_units )
                      .sum();

                      item.issued_amount -= bearer_total_amount;

                      let transfer = item.par_value( bearer_total_amount ) ;
                      item.decrease(transfer);

                      Self::balance_add(&bearer, transfer)?;
                }

                BondUnitPackageRegistry::<T>::remove_prefix(&bond);

                ensure!(item.bond_credit==0, Error::<T>::BondParamIncorrect);
                ensure!(item.issued_amount==0, Error::<T>::BondParamIncorrect);

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
                ensure!(item.state == BondState::BOOKING, Error::<T>::BondParamIncorrect);
                ensure!(item.inner.mincap_amount <= item.issued_amount, Error::<T>::BondParamIncorrect);

                let now = <pallet_timestamp::Module<T>>::get();
                item.state = BondState::ACTIVE;
                item.last_updated = now;
                item.active_start_date = now;
                // Decrease liabilities by value of the Bond main body
                item.bond_credit -=  item.par_value( item.issued_amount ) ;
                // @TODO assert item.credit == 0
                Self::deposit_event(RawEvent::BondActivated(caller, bond ));
                Ok(())
            })
        }

        /// Method: bond_issue_impact_report(origin, bond: BondId, impact_data: u64 )
        /// Arguments: origin: AccountId - transaction caller
        ///            bond: BondId - bond identifier
        ///            impact_data: u64 - report value
        ///
        /// Releases periodic impact report
        /// Access: bond issuer or reporter assigned to the bond
        #[weight = 15_000]
        fn bond_release_impact_report(origin, bond: BondId, impact_data: u64 ) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            ensure!(Self::check_with_bond(&bond, |item|{
                item.emitent == caller || item.impact_reporter == caller
            }), Error::<T>::AccountNotAuthorized );

            let now = <pallet_timestamp::Module<T>>::get();
            //@TODO verify period
            let mut reports = BondImpactReport::<T>::get(&bond);
            reports.push( BondImpactReportStructOf::<T>{
                create_date: now,
                impact_data,
                signed: false,
            });
            BondImpactReport::<T>::insert(&bond, reports );
            Self::deposit_event(RawEvent::BondImpactReportIssued( caller, bond ));
            Ok(())
        }

        /// Method: bond_sign_impact_report(origin, bond: BondId, period: u64, impact_data: u64 )
        /// Arguments: origin: AccountId - transaction caller
        ///            bond: BondId - bond identifier
        ///            period: u32 - report period starting from 0
        ///            impact_data: u64 - report value
        ///
        /// Verify report impact data by signing the report released by the bond issuer
        /// Access: only auditor assigned to the bond
        // Auditor signs impact report
        #[weight = 5_000]
        fn bond_sign_impact_report(origin, bond: BondId, period: u32, impact_data: u64 ) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            ensure!(Self::account_is_auditor(&caller), Error::<T>::AccountNotAuthorized);
            ensure!(Self::check_with_bond(&bond, |item|{
                item.auditor == caller
            }), Error::<T>::AccountNotAuthorized );

            //let now = <pallet_timestamp::Module<T>>::get();
            //@TODO verify period

            let mut reports = BondImpactReport::<T>::get(&bond);

            if let Some(last) = reports.last_mut() {
                ensure!( last.impact_data == impact_data, Error::<T>::BondParamIncorrect );
                last.signed = true;
            }else{
                return Err( Error::<T>::BondParamIncorrect.into() );
            }

            BondImpactReport::<T>::insert(&bond, reports );
            Self::deposit_event(RawEvent::BondImpactReportSigned( caller, bond ));
            Ok(())
        }

        /// Method: bond_finish(origin, bond: BondId)
        /// Arguments: origin: AccountId - transaction caller
        ///            bond: BondId - bond identifier
        ///
        /// Makes the bond reached maturity date. It requires the issuer to pay back
        /// redemption yield
        // Switch the Bond state to Finished
        #[weight = 15_000]
        fn bond_finish(origin, bond: BondId) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            //@TODO validate access rules
            Self::with_bond(&bond, |item|{
                let now = <pallet_timestamp::Module<T>>::get();

                //@TODO ensure the bond pool has enough assets to pay off all debts
                //@TODO ensure `now` is not less than bond maturity date
                item.state = BondState::FINISHED;
                item.last_updated = now;

                //@TODO pay off debts

                Self::deposit_event(RawEvent::BondFinished(caller, bond ));
                Ok(())
            })
        }

        /// Method: bond_declare_bankrupt(origin, bond: BondId)
        /// Arguments: origin: AccountId - transaction caller
        ///            bond: BondId - bond identifier
        /// Access: any
        ///
        /// Marks the bond as bankrupt
        #[weight = 10_000]
        fn bond_declare_bankrupt(origin, bond: BondId) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            //@TODO validate access rules
            Self::with_bond(&bond, |item|{
                let now = <pallet_timestamp::Module<T>>::get();
                ensure!(item.state == BondState::ACTIVE, Error::<T>::BondParamIncorrect);
                item.state = BondState::BANKRUPT;
                item.last_updated = now;

                Self::deposit_event(RawEvent::BondBankrupted(caller, bond ));
                Ok(())
            })
        }

        // @TODO
        // Pay off coupon
        // Pay off main body

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
            ensure!(item.emitent == caller || item.manager == caller, Error::<T>::BondParamIncorrect);
            ensure!(item.state == BondState::PREPARE, Error::<T>::BondParamIncorrect);
            BondRegistry::<T>::remove( &bond );

            Self::deposit_event(RawEvent::BondRevoked(caller, bond ));
            Ok(())

        }

        /// Method: bond_withdraw_everusd(origin, bond: BondId, amount: EverUSDBalance)
        /// Arguments: origin: AccountId - transaction caller
        ///            bond: BondId - bond identifier
        ///            amount: EverUSDBalance - the number of EverUSD  withdrawal from bond fund
        /// Access: Bond issuer
        ///
        /// Transfer `amount` of EverUSD tokens  from the bond fund to the caller balance
        /// `amount` cannot exceed the bond unencumbered balance
        #[weight = 5_000]
        fn bond_withdraw_everusd(origin, bond: BondId, amount: EverUSDBalance) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            Self::with_bond(&bond, |item|{
                ensure!(item.state == BondState::ACTIVE, Error::<T>::BondParamIncorrect);
                ensure!(item.emitent == caller, Error::<T>::BondParamIncorrect);
                ensure!(item.get_balance() >= amount, Error::<T>::BondParamIncorrect);

                Self::balance_add(&caller, amount)?;
                item.bond_debit -= amount;

                Self::deposit_event(RawEvent::BondWithdrawEverUSD(caller, bond, amount ));
                Ok(())
            })
        }

        /// Method: bond_deposit_everusd(origin, bond: BondId, amount: EverUSDBalance)
        /// Arguments: origin: AccountId - transaction caller
        ///            bond: BondId - bond identifier
        ///            amount: EverUSDBalance - the number of EverUSD  deposited to bond fund
        /// Access: Bond issuer
        ///
        /// Transfer `amount` of EverUSD tokens from caller balance to the bond fund
        #[weight = 5_000]
        fn bond_deposit_everusd(origin, bond: BondId, amount: EverUSDBalance) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            Self::with_bond(&bond, |item|{
                ensure!(
                    matches!(item.state , BondState::ACTIVE | BondState::BANKRUPT),
                    Error::<T>::BondParamIncorrect
                );
                ensure!(item.emitent == caller, Error::<T>::BondParamIncorrect);

                Self::balance_sub(&caller, amount)?;

                let (amount, ovf) = item.bond_debit.overflowing_add( amount );
                ensure!(!ovf, Error::<T>::BondParamIncorrect );
                item.bond_debit = amount;
                // @TODO pay off debts

                Self::deposit_event(RawEvent::BondDepositEverUSD(caller, bond, amount ));
                Ok(())
            })
        }

        #[weight = 5_000]
        fn bond_dummy(origin) -> DispatchResult {
            let now = <pallet_timestamp::Module<T>>::get();
            ensure!(now>0.into(), Error::<T>::BondParamIncorrect );
            Ok(())
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

    /// Method: account_is_emitent(acc: &T::AccountId) -> bool
    /// Arguments: acc: AccountId - checked account id
    ///
    /// Checks if the acc has global Emitent role
    pub fn account_is_emitent(acc: &T::AccountId) -> bool {
        AccountRegistry::<T>::contains_key(acc)
            && (AccountRegistry::<T>::get(acc).roles & EMITENT_ROLE_MASK != 0)
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
        const ALLOWED_ROLES_MASK: u8 = INVESTOR_ROLE_MASK | EMITENT_ROLE_MASK;
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

    /// Method: get_bond(bond: BondId) -> bond: BondId) -> BondStruct
    /// Arguments: bond: BondId - bond unique identifier
    ///
    ///  Returns bond structure if found
    pub fn get_bond(bond: BondId) -> BondStructOf<T> {
        BondRegistry::<T>::get(bond)
    }

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

    fn check_with_bond<F: FnOnce(&BondStructOf<T>) -> bool>(bond: &BondId, f: F) -> bool {
        if BondRegistry::<T>::contains_key(bond) {
            let item = BondRegistry::<T>::get(bond);
            f(&item)
        } else {
            false
        }
    }

    fn balance_add(who: &T::AccountId, amount: EverUSDBalance) -> DispatchResult {
        //@TODO handle overflow
        let (new_balance, ovf) = BalanceEverUSD::<T>::get(who).overflowing_add(amount);
        if ovf {
            return Err(Error::<T>::BalanceOverdraft.into());
        }
        BalanceEverUSD::<T>::insert(who, new_balance);
        Ok(())
    }

    fn balance_sub(who: &T::AccountId, amount: EverUSDBalance) -> DispatchResult {
        let balance = BalanceEverUSD::<T>::get(who);
        if balance < amount {
            return Err(Error::<T>::BalanceOverdraft.into());
        }
        BalanceEverUSD::<T>::insert(&who, balance - amount);
        Ok(())
    }

    fn purge_expired_burn_requests(before: T::Moment) {
        let to_purge: Vec<_> = BurnRequestEverUSD::<T>::iter()
            .filter(|(_, request)| request.deadline <= before)
            .map(|(acc, _)| acc)
            .take(MAX_PURGE_REQUESTS)
            .collect();

        for acc in to_purge {
            BurnRequestEverUSD::<T>::remove(acc);
        }
    }

    fn purge_expired_mint_requests(before: T::Moment) {
        let to_purge: Vec<_> = MintRequestEverUSD::<T>::iter()
            .filter(|(_, request)| request.deadline <= before)
            .map(|(acc, _)| acc)
            .take(MAX_PURGE_REQUESTS)
            .collect();

        for acc in to_purge {
            MintRequestEverUSD::<T>::remove(acc);
        }
    }

    #[allow(dead_code)]
    fn purge_expired_bondunit_lots(_before: T::Moment) {}
}
