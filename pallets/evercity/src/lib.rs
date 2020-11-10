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

use frame_support::sp_runtime::traits::AtLeast32Bit;
use sp_core::sp_std::cmp::min;
use sp_runtime::traits::{SaturatedConversion, UniqueSaturatedInto};

pub trait Trait: frame_system::Trait + pallet_timestamp::Trait {
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;
}

pub trait Expired<Moment> {
    fn is_expired(&self, now: Moment) -> bool;
}

pub type Result<T> = core::result::Result<T, DispatchError>;
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Default, Encode, Decode, RuntimeDebug)]
pub struct BondId([u8; 8]);

impl PartialEq for BondId {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl From<&str> for BondId {
    fn from(name: &str) -> BondId {
        let mut b = [0u8; 8];
        unsafe {
            core::intrinsics::copy_nonoverlapping(
                name.as_ptr(),
                b.as_mut_ptr(),
                min(8, name.len()),
            );
        }
        BondId(b)
    }
}

impl Eq for BondId {}

impl core::ops::Deref for BondId {
    type Target = [u8; 8];
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

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

pub const EVERUSD_DECIMALS: u64 = 9; // EverUSD = USD * ( 10 ^ EVERUSD_DECIMALS )
pub const EVERUSD_MAX_MINT_AMOUNT: EverUSDBalance = 60_000_000_000_000_000; // =60 million dollar

pub const MIN_PAYMENT_PERIOD: BondPeriod = DAY_DURATION * 7;

pub const TOKEN_BURN_REQUEST_TTL: u32 = DAY_DURATION as u32 * 7 * 1000;
pub const TOKEN_MINT_REQUEST_TTL: u32 = DAY_DURATION as u32 * 7 * 1000;
const INTEREST_RATE_YEAR: u64 = 3_153_600;

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

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, Clone, Default, RuntimeDebug)]
pub struct TokenMintRequestStruct<Moment> {
    pub amount: EverUSDBalance,
    pub deadline: Moment,
}

impl<Moment: core::cmp::PartialOrd> Expired<Moment> for TokenMintRequestStruct<Moment> {
    fn is_expired(&self, now: Moment) -> bool {
        self.deadline < now
    }
}

type TokenMintRequestStructOf<T> = TokenMintRequestStruct<<T as pallet_timestamp::Trait>::Moment>;

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, Clone, Default, RuntimeDebug)]
pub struct TokenBurnRequestStruct<Moment> {
    pub amount: EverUSDBalance,
    pub deadline: Moment,
}

impl<Moment: core::cmp::PartialOrd> Expired<Moment> for TokenBurnRequestStruct<Moment> {
    fn is_expired(&self, now: Moment) -> bool {
        self.deadline < now
    }
}

type TokenBurnRequestStructOf<T> = TokenBurnRequestStruct<<T as pallet_timestamp::Trait>::Moment>;

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

/// Bond period parametes type, seconds
type BondPeriod = u32;
/// The number of Bond units,
type BondUnitAmount = u32;
/// Annual coupon interest rate as 1/100000 of par value
type BondInterest = u32;
/// Bond period numerator
type BondPeriodNumber = u32;

const MIN_BOND_DURATION: u32 = 1; // 1  is a minimal bond period
const DAY_DURATION: u32 = 86400; // seconds in 1 DAY

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, Clone, Default, PartialEq, RuntimeDebug)]
pub struct BondInnerStruct<Moment, Hash> {
    // bond document hashes
    pub data_hash_main: Hash,
    pub data_hash_legal: Hash,
    pub data_hash_finance: Hash,
    pub data_hash_tech: Hash,

    // bond impact parameters
    // Impact data type can be:
    //   0 - CO2 emission.
    //   1 - electric power generated
    pub impact_data_type: u8,
    pub impact_data_baseline: u64,
    // Coupon interest regulatory options
    pub impact_data_max_deviation_cap: u64,
    pub impact_data_max_deviation_floor: u64,
    // seconds before end of a period
    // when issuer should release regular impact report
    pub impact_data_send_period: BondPeriod,
    // increase interest rate if impact report was missed
    pub interest_rate_penalty_for_missed_report: BondInterest,
    // base coupon interest rate, ppm
    pub interest_rate_base_value: BondInterest,
    // max coupon interest rate, ppm
    pub interest_rate_margin_cap: BondInterest,
    // min coupon interest rate, ppm
    pub interest_rate_margin_floor: BondInterest,
    // interest rate from activation up to start_period, ppm
    pub interest_rate_start_period_value: BondInterest,

    // days from activation when effective interest rate
    // invariably equals to interest_rate_start_period_value
    pub start_period: BondPeriod,
    // interest rate change and payment period
    pub payment_period: BondPeriod,
    // seconds after every payment_period when Emitent should pay off coupon interests
    pub interest_pay_period: BondPeriod,
    // the number of periods from active_start_date until maturity date
    // bond maturity period = start_period + bond_duration * payment_period
    pub bond_duration: BondPeriodNumber,
    // seconds from maturity date until full repayment
    pub bond_finishing_period: BondPeriod,
    // mincap_amount bond units should be raised up to this date
    // otherwise bond can be withdrawn by issuer
    pub mincap_deadline: Moment,
    // minimal amount of issued bond units
    pub bond_units_mincap_amount: BondUnitAmount,
    // maximal amount of issued bond units
    pub bond_units_maxcap_amount: BondUnitAmount,
    // bond unit par value
    pub bond_units_base_price: EverUSDBalance,
}

type BondInnerStructOf<T> =
    BondInnerStruct<<T as pallet_timestamp::Trait>::Moment, <T as frame_system::Trait>::Hash>;

#[inline]
fn is_period_multiple_of_day(period: BondPeriod) -> bool {
    (period % DAY_DURATION) == 0
}

impl<Moment, Hash> BondInnerStruct<Moment, Hash> {
    /// Checks if other bond has the same financial properties
    pub fn is_financial_options_eq(&self, other: &Self) -> bool {
        self.bond_units_base_price == other.bond_units_base_price
            && self.interest_rate_base_value == other.interest_rate_base_value
            && self.interest_rate_margin_cap == other.interest_rate_margin_cap
            && self.interest_rate_margin_floor == other.interest_rate_margin_floor
            && self.impact_data_max_deviation_cap == other.impact_data_max_deviation_cap
            && self.impact_data_max_deviation_floor == other.impact_data_max_deviation_floor
            && self.bond_duration == other.bond_duration
            && self.bond_units_mincap_amount == other.bond_units_mincap_amount
            && self.bond_units_maxcap_amount == other.bond_units_maxcap_amount
            && self.impact_data_type == other.impact_data_type
            && self.impact_data_baseline == other.impact_data_baseline
            && self.interest_pay_period == other.interest_pay_period
            && self.impact_data_send_period == other.impact_data_send_period
            && self.payment_period == other.payment_period
            && self.bond_finishing_period == other.bond_finishing_period
    }
    /// Checks if bond data is valid
    pub fn is_valid(&self) -> bool {
        self.bond_units_mincap_amount > 0
            && self.bond_units_maxcap_amount >= self.bond_units_mincap_amount
            && self.payment_period >= MIN_PAYMENT_PERIOD
            && self.impact_data_send_period <= self.payment_period
            && is_period_multiple_of_day(self.payment_period)
            && is_period_multiple_of_day(self.start_period)
            && is_period_multiple_of_day(self.impact_data_send_period)
            && is_period_multiple_of_day(self.bond_finishing_period)
            && (self.start_period == 0 || self.start_period >= self.payment_period)
            && self.interest_pay_period <= self.payment_period
            && self.bond_units_base_price > 0
            && self
                .bond_units_base_price
                .saturating_mul(self.bond_units_maxcap_amount as EverUSDBalance)
                < EverUSDBalance::MAX
            && self.interest_rate_base_value >= self.interest_rate_margin_floor
            && self.interest_rate_base_value <= self.interest_rate_margin_cap
            && self.impact_data_baseline <= self.impact_data_max_deviation_cap
            && self.impact_data_baseline >= self.impact_data_max_deviation_floor
            && self.bond_duration >= MIN_BOND_DURATION
    }
}

#[cfg_attr(feature = "std", derive(Debug))]
struct PeriodDescr {
    // period: BondPeriodNumber,
    // ... |         period            |   ...
    // --- | ------------------------- | -------------...
    //     |                  |        |          |
    //   start              report   reset    interest pay
    //    >----------------------------< coupon accrual
    // report release period  >--------<
    //              coupon pay period  >----------<
    start_period: BondPeriod,            // sec from activation
    impact_data_send_period: BondPeriod, // sec from activation
    payment_period: BondPeriod,          // sec from activation
    #[allow(dead_code)]
    interest_pay_period: BondPeriod,     // sec from activation
}

impl PeriodDescr {
    fn duration(&self, moment: BondPeriod) -> BondPeriod {
        if moment <= self.start_period {
            self.payment_period - self.start_period
        } else if moment >= self.payment_period {
            0
        } else {
            (self.payment_period - moment) / DAY_DURATION * DAY_DURATION
        }
    }
}

#[derive(Encode, Decode, Clone, Default, PartialEq, RuntimeDebug)]
pub struct AccountYield {
    coupon_yield: EverUSDBalance,
    period_num: BondPeriodNumber,
}

#[derive(Encode, Decode, Clone, Default, PartialEq, RuntimeDebug)]
pub struct PeriodYield {
    // bond cumulative accrued yield
    total_yield: EverUSDBalance,
    // bond fund forwarded to pay off coupon yield
    coupon_yield_before: EverUSDBalance,
    // effective interested rate
    interest_rate: BondInterest,
}

// #[derive(Encode, Decode, Clone, Default, PartialEq, RuntimeDebug)]
// struct BondBalance {
//     // #Bond ledger
//     // free balance is a difference between bond_debit and bond_credit
//     pub bond_debit: EverUSDBalance,
//     // issuer liabilities
//     pub bond_credit: EverUSDBalance,
//     // ever-increasing coupon fund
//     amount: EverUSDBalance,
// }

struct PeriodIterator<'a, AccountId, Moment, Hash> {
    bond: &'a BondStruct<AccountId, Moment, Hash>,
    index: BondPeriodNumber,
}

impl<'a, AccountId, Moment, Hash> PeriodIterator<'a, AccountId, Moment, Hash> {
    #[allow(dead_code)]
    fn new(bond: &'a BondStruct<AccountId, Moment, Hash>) -> Self {
        PeriodIterator { bond, index: 0 }
    }
}

impl<'a, AccountId, Moment, Hash> core::iter::Iterator
    for PeriodIterator<'a, AccountId, Moment, Hash>
{
    type Item = PeriodDescr;

    fn next(&mut self) -> Option<Self::Item> {
        let inner = &self.bond.inner;
        let index = if inner.start_period > 0 {
            self.index
        } else {
            self.index + 1
        };

        if index > inner.bond_duration {
            None
        } else {
            let payment_period = inner.start_period + index * inner.payment_period;
            self.index += 1;

            // last pay period is special and lasts bond_finishing_period seconds
            let pay_period = if index == inner.bond_duration {
                inner.bond_finishing_period
            } else {
                inner.interest_pay_period
            };

            Some(PeriodDescr {
                payment_period,
                start_period: payment_period
                    - if index == 0 {
                        inner.start_period
                    } else {
                        inner.payment_period
                    },
                impact_data_send_period: payment_period - inner.impact_data_send_period,
                interest_pay_period: payment_period + pay_period,
            })
        }
    }
}

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, Clone, Default, RuntimeDebug)]
pub struct BondStruct<AccountId, Moment, Hash> {
    pub inner: BondInnerStruct<Moment, Hash>,
    // bond issuer
    pub emitent: AccountId,
    // #Auxiliary roles
    pub manager: AccountId,
    pub auditor: AccountId,
    pub impact_reporter: AccountId,

    // total amount of issued bond units
    pub issued_amount: BondUnitAmount,

    // #Timestamps
    pub creation_date: Moment,
    pub booking_start_date: Moment,
    pub active_start_date: Moment,
    // Bond current state
    pub state: BondState,

    // #Bond ledger
    // everusd bond fund
    pub bond_debit: EverUSDBalance,
    // issuer liabilities
    pub bond_credit: EverUSDBalance,
    // free balance is difference between bond_debit and bond_credit
    // ever-increasing coupon fund which was distributed among bondholders
    // undistributed bond fund is equal to  (bond_debit - coupon_yield)
    coupon_yield: EverUSDBalance,
}

type BondStructOf<T> = BondStruct<
    <T as frame_system::Trait>::AccountId,
    <T as pallet_timestamp::Trait>::Moment,
    <T as frame_system::Trait>::Hash,
>;

impl<AccountId, Moment, Hash> BondStruct<AccountId, Moment, Hash> {
    /// Returns nominal value of unit_amount Bond units
    #[inline]
    fn par_value(&self, unit_amount: BondUnitAmount) -> EverUSDBalance {
        unit_amount as EverUSDBalance * self.inner.bond_units_base_price as EverUSDBalance
    }
    /// Returns bond unpaid unliabilities
    #[allow(dead_code)]
    fn get_debt(&self) -> EverUSDBalance {
        if self.bond_credit > self.bond_debit {
            self.bond_credit - self.bond_debit
        } else {
            0
        }
    }
    /// Returns the number of  tokens available for emitent
    fn get_free_balance(&self) -> EverUSDBalance {
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
    #[inline]
    fn get_periods(&self) -> BondPeriodNumber {
        if self.inner.start_period == 0 {
            self.inner.bond_duration
        } else {
            self.inner.bond_duration + 1
        }
    }

    #[allow(dead_code)]
    fn iter_periods(&self) -> PeriodIterator<'_, AccountId, Moment, Hash> {
        PeriodIterator::new(self)
    }

    /// Returns  time limits of the period
    #[allow(dead_code)]
    fn period_desc(&self, period: BondPeriodNumber) -> Option<PeriodDescr> {
        let mut iter = PeriodIterator::new(&self);
        iter.index = period;
        iter.next()
    }

    #[allow(dead_code)]
    /// Calculate coupon effective interest rate
    fn interest_rate(&self, impact_data: u64) -> BondInterest {
        let inner = &self.inner;

        if impact_data >= inner.impact_data_max_deviation_cap {
            inner.interest_rate_margin_floor
        } else if impact_data <= inner.impact_data_max_deviation_floor {
            inner.interest_rate_margin_cap
        } else if impact_data == inner.impact_data_baseline {
            inner.interest_rate_base_value
        } else if impact_data > inner.impact_data_baseline {
            inner.interest_rate_base_value
                - ((impact_data - inner.impact_data_baseline) as u128
                    * (inner.interest_rate_base_value - inner.interest_rate_margin_floor) as u128
                    / (inner.impact_data_max_deviation_cap - inner.impact_data_baseline) as u128)
                    as BondInterest
        } else {
            inner.interest_rate_base_value
                + ((inner.impact_data_baseline - impact_data) as u128
                    * (inner.interest_rate_margin_cap - inner.interest_rate_base_value) as u128
                    / (inner.impact_data_baseline - inner.impact_data_max_deviation_floor) as u128)
                    as BondInterest
        }
    }
}

impl<AccountId, Moment: UniqueSaturatedInto<u64> + AtLeast32Bit + Copy, Hash>
    BondStruct<AccountId, Moment, Hash>
{
    fn time_passed_after_activation(&self, now: Moment) -> Option<(BondPeriod, BondPeriodNumber)> {
        if !matches!(self.state, BondState::ACTIVE | BondState::BANKRUPT)
            || now < self.active_start_date
        {
            None
        } else {
            // gets the number or seconds since the bond was activated
            let moment = (now - self.active_start_date).saturated_into::<u64>() / 1000_u64;
            if moment >= u32::MAX as u64 {
                return None;
            }
            let moment = moment as u32;
            if moment < self.inner.start_period {
                Some((moment, 0))
            } else {
                let has_start_period: BondPeriodNumber =
                    if self.inner.start_period > 0 { 1 } else { 0 };
                let period = min(
                    ((moment - self.inner.start_period) / self.inner.payment_period)
                        as BondPeriodNumber,
                    self.inner.bond_duration,
                );

                Some((moment, period + has_start_period))
            }
        }
    }
}

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

#[derive(Encode, Decode, Clone, Default, PartialEq, RuntimeDebug)]
pub struct BondUnitPackage<Moment> {
    bond_units: BondUnitAmount,
    acquisition: BondPeriod,
    coupon_yield: EverUSDBalance,
    create_date: Moment,
}
type BondUnitPackageOf<T> = BondUnitPackage<<T as pallet_timestamp::Trait>::Moment>;

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, Clone, Default, RuntimeDebug)]
pub struct BondImpactReportStruct<Moment> {
    create_date: Moment,
    impact_data: u64,
    signed: bool,
}

type BondImpactReportStructOf<T> = BondImpactReportStruct<<T as pallet_timestamp::Trait>::Moment>;

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, Clone, Default, Eq, PartialEq, RuntimeDebug)]
pub struct BondUnitSaleLotStruct<AccountId, Moment> {
    deadline: Moment,
    new_bondholder: AccountId,
    bond_units: BondUnitAmount,
    amount: EverUSDBalance,
}

impl<AccountId, Moment: core::cmp::PartialOrd> Expired<Moment>
    for BondUnitSaleLotStruct<AccountId, Moment>
{
    fn is_expired(&self, now: Moment) -> bool {
        self.deadline < now
    }
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
                map hasher(blake2_128_concat) T::AccountId => TokenMintRequestStructOf<T>;

        // Same as MintRequest, but for burning EverUSD tokens, paying to creator in USD
        // In future these actions can require different data, so it's separate structure
        // than mint request
        BurnRequestEverUSD
            get(fn burn_request_everusd):
                map hasher(blake2_128_concat) T::AccountId => TokenBurnRequestStructOf<T>;

        // Structure for storing bonds
        BondRegistry
            get(fn bond_registry):
                map hasher(blake2_128_concat) BondId => BondStructOf<T>;

        // Bearer's Bond units
        BondUnitPackageRegistry
            get(fn bond_unit_registry):
                double_map hasher(blake2_128_concat) BondId, hasher(blake2_128_concat) T::AccountId => Vec<BondUnitPackageOf<T>>;

        // Bond coupon yield storage
        // Every element has total bond yield of passed period recorded on accrual basis
        BondCouponYield
            get(fn bond_coupon_yield):
                map hasher(blake2_128_concat) BondId=>Vec<PeriodYield>;

        // BondLastCouponPeriod
        //     get(fn bond_last_period):
        //         double_map hasher(blake2_128_concat) BondId, hasher(blake2_128_concat) T::AccountId => BondPeriodNumber;

        // Contains last requested by a bondholder period and bond fund value
        BondLastCouponYield
            get(fn bond_last_coupon_yield):
                double_map hasher(blake2_128_concat) BondId, hasher(blake2_128_concat) T::AccountId => AccountYield;

        // BondBalance
        //     get(fn bond_balance):
        //         map hasher(blake2_128_concat) BondId=>BondBalance;

        // Bond sale lots
        BondUnitPackageLot
            get(fn bond_unit_lots):
                double_map hasher(blake2_128_concat) BondId, hasher(blake2_128_concat) T::AccountId => Vec<BondUnitSaleLotStructOf<T>>;

        // BondUnitPackageLock
        //     get(fn bond_unit_lock):
        //         double_map hasher(blake2_128_concat) BondId, hasher(blake2_128_concat) T::AccountId => (BondUnitAmount,BondUnitAmount)

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
        BondUnitSaleLotStruct = BondUnitSaleLotStructOf<T>, // Moment = <T as pallet_timestamp::Trait>::Moment,
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
        BondCouponYield(AccountId, BondId),

        BondNewSaleLot(AccountId, BondId, BondUnitSaleLotStruct),
    }
);

decl_error! {
    pub enum Error for Module<T: Trait> {
        NoneValue,
        /// Account tried to use more EverUSD  than was available on the balance
        BalanceOverdraft,

        /// Account was already added and present in mapping
        AccountToAddAlreadyExists,

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

        /// Bond with same ticker already exists
        BondAlreadyExists,

        /// Incorrect bond data
        BondParamIncorrect,

        /// Incorrect bond ticker
        BondNotFound,

        /// Bond access rules do not permit the requested action
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

        /// Account management functions

        /// Method: account_disable(who: AccountId)
        /// Arguments: who: AccountId
        /// Access: Master role
        ///
        /// Disables access to platform. Disable all roles, account is not allowed to perform any actions
        /// but still have her data in blockchain (to not loose related entities)
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

            AccountRegistry::<T>::mutate(&who,|acc|{
                acc.roles |= role;
            });

            Self::deposit_event(RawEvent::AccountSet(_caller, who, role, identity));
            Ok(())
        }

        /// Token balances manipulation functions

        /// Creates mint request to mint given amount of tokens on address of caller(emitent or investor)
        #[weight = 15_000]
        fn token_mint_request_create_everusd(origin, amount_to_mint: EverUSDBalance) -> DispatchResult {
            let _caller = ensure_signed(origin)?;
            ensure!(Self::account_token_mint_burn_allowed(&_caller), Error::<T>::AccountNotAuthorized);
            ensure!(amount_to_mint < EVERUSD_MAX_MINT_AMOUNT, Error::<T>::MintRequestParamIncorrect);
            // @TODO remove an existing request if it expired
            ensure!(!MintRequestEverUSD::<T>::contains_key(&_caller), Error::<T>::MintRequestAlreadyExist);

            let now: <T as pallet_timestamp::Trait>::Moment = <pallet_timestamp::Module<T>>::get();
            let new_mint_request = TokenMintRequestStruct{
                amount: amount_to_mint,
                deadline: now + TOKEN_MINT_REQUEST_TTL .into(),
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
            // @TODO remove an existing request if it expired
            ensure!(!MintRequestEverUSD::<T>::contains_key(&_caller), Error::<T>::MintRequestAlreadyExist);

            let _current_balance = BalanceEverUSD::<T>::get(&_caller);
            ensure!(amount_to_burn <= _current_balance, Error::<T>::MintRequestParamIncorrect);
            let now: <T as pallet_timestamp::Trait>::Moment = <pallet_timestamp::Module<T>>::get();

            let new_burn_request = TokenBurnRequestStruct {
                amount: amount_to_burn,
                deadline: now + TOKEN_BURN_REQUEST_TTL .into(),
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

            Self::balance_sub(&who, amount_to_sub)?;
            TotalSupplyEverUSD::mutate(|total|{
                *total-=amount_to_sub;
            });

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
                    bond_debit: 0, //Default::default(),
                    bond_credit: 0, //Default::default(),
                    coupon_yield: 0
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
            // Bond Auxiliary roles can be set only by Master
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
                // @TODO add ability to update the Bond in BOOKING state
                // preserving the bond_units_base_price value
                ensure!(
                    matches!(item.state, BondState::PREPARE | BondState::BOOKING),
                    Error::<T>::BondStateNotPermitAction
                );
                ensure!(
                    item.emitent == caller || item.manager == caller ,
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

        /// Method: bond_unit_package_take(origin, bond: BondId, unit_amount: BondUnitAmount )
        /// Arguments: origin: AccountId - transaction caller
        ///            bond: BondId - bond identifier
        ///            unit_amount: BondUnitAmount - amount of bond units
        ///
        /// Bye bond units.
        /// Access: only accounts with Investor role
        // Investor loans tokens to the bond issuer by staking bond units
        #[weight = 10_000]
        fn bond_unit_package_take(origin, bond: BondId, unit_amount: BondUnitAmount ) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            ensure!(Self::account_is_investor(&caller), Error::<T>::AccountNotAuthorized);
            Self::with_bond(&bond, |mut item|{
                ensure!(
                    item.state==BondState::ACTIVE || item.state == BondState::BOOKING,
                    Error::<T>::BondStateNotPermitAction
                );
                // issuer cannot buy his own bonds
                ensure!(item.emitent!=caller, Error::<T>::BondParamIncorrect );

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
                        BondUnitPackageOf::<T>{
                             bond_units: unit_amount,
                             acquisition,
                             coupon_yield: 0,
                             create_date: now,
                        }
                    );
                });

                item.issued_amount = issued_amount;

                // everusd received can be forwarded to pay off the debt
                if item.state==BondState::ACTIVE {
                    item.bond_debit += package_value;

                    Self::calc_and_store_bond_coupon_yield(&bond, &mut item, now);

                    let free_balance = item.get_free_balance();
                    if free_balance > 0 {
                        item.bond_debit -= free_balance;
                        Self::balance_add(&item.emitent, free_balance)?;
                    }
                }else{
                    // increase assets and liabilities of the Bond
                    item.increase( package_value );
                }

                Self::deposit_event(RawEvent::BondSale(caller.clone(), bond, unit_amount ));

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
                ensure!( item.state == BondState::BOOKING, Error::<T>::BondStateNotPermitAction );
                // Ensure the Bond raises less then bond_units_mincap_amount bond units
                ensure!(item.inner.bond_units_mincap_amount > item.issued_amount, Error::<T>::BondParamIncorrect);
                ensure!(
                    item.emitent == caller || item.manager == caller || Self::account_is_master(&caller) ,
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
                let mut reports: Vec<BondImpactReportStructOf<T>> = Vec::new();
                reports.resize( ( item.inner.bond_duration + 1 ) as usize,  BondImpactReportStruct{
                    create_date: Default::default(),
                    impact_data: 0,
                    signed: false,
                });

                BondImpactReport::<T>::insert(&bond, &reports);

                // withdraw all available bond fund
                Self::balance_add(&item.emitent, item.bond_debit)?;
                item.bond_debit = 0;

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
        fn bond_impact_report_send(origin, bond: BondId, period: BondPeriodNumber, impact_data: u64 ) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            let now = <pallet_timestamp::Module<T>>::get();
            {
                let item = BondRegistry::<T>::get(bond);
                ensure!(item.emitent == caller || item.impact_reporter == caller, Error::<T>::BondAccessDenied );
                ensure!(Self::is_report_in_time(&item, now, period), Error::<T>::BondOutOfOrder );
            }

            let index: usize = period as usize;
            //@TODO verify period time interval
            BondImpactReport::<T>::try_mutate(&bond, |reports|->DispatchResult {

                ensure!(index < reports.len() && !reports[index].signed, Error::<T>::BondParamIncorrect );

                reports[index].create_date = now;
                reports[index].impact_data = impact_data;

                Self::deposit_event(RawEvent::BondImpactReportIssued( caller, bond ));
                Ok(())

            })
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
        fn bond_sign_impact_report(origin, bond: BondId, period: BondPeriodNumber, impact_data: u64 ) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            ensure!(Self::account_is_auditor(&caller), Error::<T>::AccountNotAuthorized);
            let now = <pallet_timestamp::Module<T>>::get();
            {
                let item = BondRegistry::<T>::get(bond);
                ensure!(item.auditor == caller, Error::<T>::BondAccessDenied );
                ensure!(Self::is_report_in_time(&item, now, period), Error::<T>::BondOutOfOrder );
            }

            //let now = <pallet_timestamp::Module<T>>::get();
            //@TODO verify period
            let index: usize = period as usize;
            BondImpactReport::<T>::try_mutate(&bond, |reports|->DispatchResult {
                ensure!(index < reports.len() && !reports[index].signed, Error::<T>::BondParamIncorrect );
                ensure!(reports[index].impact_data == impact_data, Error::<T>::BondParamIncorrect );

                reports[index].signed = true;

                Self::deposit_event(RawEvent::BondImpactReportSigned( caller, bond ));

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
            //@TODO validate access rules
            Self::with_bond(&bond, |item|{
                //@TODO ensure the bond pool has enough assets to pay off all debts
                //@TODO ensure `now` is not less than bond maturity date
                item.state = BondState::FINISHED;

                //@TODO pay off debts

                // remove all sale lots
                BondUnitPackageLot::<T>::remove_prefix(&bond);
                Self::deposit_event(RawEvent::BondFinished(caller, bond ));
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

            Self::with_bond(&bond, |item|{
                ensure!(item.state == BondState::ACTIVE, Error::<T>::BondStateNotPermitAction);
                // @TODO refine condition
                item.state = BondState::BANKRUPT;

                Self::deposit_event(RawEvent::BondBankrupted(caller, bond ));
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
            let caller = ensure_signed(origin)?;

            Self::with_bond(&bond, |mut item|{
                let now = <pallet_timestamp::Module<T>>::get();
                if Self::calc_and_store_bond_coupon_yield(&bond, &mut item, now){
                    Self::deposit_event(RawEvent::BondCouponYield(caller, bond ));
                }
                // @TODO check bankrupt status
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
            ensure!(item.emitent == caller || item.manager == caller, Error::<T>::BondAccessDenied);
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
                ensure!(item.state == BondState::ACTIVE, Error::<T>::BondStateNotPermitAction);

                let now = <pallet_timestamp::Module<T>>::get();
                Self::calc_and_store_bond_coupon_yield(&bond, &mut item, now);

                let amount: EverUSDBalance = if item.emitent == caller { // issuer branch
                    let amount = item.get_free_balance();
                    if amount>0{
                        Self::balance_add(&item.emitent, amount)?;
                        // it's safe to do unchecked subtraction
                        item.bond_debit -= amount;
                    }
                    amount
                }else{ // investor (bondholder) branch
                    Self::request_coupon_yield(&bond, &mut item, &caller)
                };
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
                ensure!(item.emitent == caller, Error::<T>::BondAccessDenied);

                Self::balance_sub(&caller, amount)?;

                item.bond_debit = item.bond_debit.checked_add( amount )
                    .ok_or( Error::<T>::BondParamIncorrect )?;
                let now = <pallet_timestamp::Module<T>>::get();
                Self::calc_and_store_bond_coupon_yield(&bond, &mut item, now);
                // everusd should  be forwarded to pay off the debt at first
                // if item.bond_credit > item.coupon_yield && item.bond_debit > item.coupon_yield {
                //     item.coupon_yield = min(item.bond_credit, item.bond_debit);
                // }

                Self::deposit_event(RawEvent::BondDepositEverUSD(caller, bond, amount ));
                Ok(())
            })
        }

        /// Method: bond_unit_sale(origin, bond: BondId, lot: BondUnitSaleLotStruct)
        /// Arguments: origin: AccountId - bond unit bondholder
        ///            bond: BondId - bond identifier
        ///            lot: BondUnitSaleLotStruct - lot data
        /// Access: Bond bondholder
        ///
        /// Create sale lot
        #[weight = 5_000]
        fn bond_unit_sale(origin, bond: BondId, lot: BondUnitSaleLotStructOf<T>) -> DispatchResult{
            let caller = ensure_signed(origin)?;
            let packages = BondUnitPackageRegistry::<T>::get(&bond, &caller);
            // how many bond units does the caller have
            let total_bond_units: BondUnitAmount = packages.iter()
            .map(|package| package.bond_units)
            .sum();

            ensure!(total_bond_units>=lot.bond_units, Error::<T>::BondParamIncorrect );

            let now = <pallet_timestamp::Module<T>>::get();
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
            // save active lots
            BondUnitPackageLot::<T>::insert(&bond, &caller, lots);
            Self::deposit_event(RawEvent::BondNewSaleLot(caller, bond, lot ));
            Ok(())
        }

        /// Method: bond_unit_close_deal(origin, bond: BondId,bondholder: AccountId, lot: BondUnitSaleLotStruct)
        /// Arguments: origin: AccountId - bond unit bondholder
        ///            bond: BondId - bond identifier
        ///            bondholder: Current bondholder of of bond
        ///            lot: BondUnitSaleLotStruct - lot data
        /// Access: Bond bondholder
        ///
        /// Buy the lot created by bond_unit_sale call
        #[weight = 5_000]
        fn bond_unit_close_deal(origin, bond: BondId, bondholder: T::AccountId, lot: BondUnitSaleLotStructOf<T>)->DispatchResult{
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
                     // @TODO distribute coupon yield for caller and bondholder
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

                     // @TODO arrange as separate function
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
                                BondUnitPackageOf::<T>{
                                     bond_units,
                                     acquisition,
                                     coupon_yield,
                                     create_date: now,
                                }
                           );
                     }
                     from_packages.shrink_to_fit();
                     // store new packages
                     BondUnitPackageRegistry::<T>::insert(&bond, &bondholder, from_packages);
                     BondUnitPackageRegistry::<T>::insert(&bond, &bondholder, to_packages);

                     // pay off deal
                     Self::balance_sub(&caller, lot.amount)?;
                     Self::balance_add(&bondholder, lot.amount)?;

                     Ok(())
                }else{
                    Err(Error::<T>::BondParamIncorrect.into())
                }
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
    #[cfg(test)]
    pub fn get_bond(bond: &BondId) -> BondStructOf<T> {
        BondRegistry::<T>::get(bond)
    }

    #[cfg(test)]
    pub fn bond_packages(bond: &BondId, bondholder: &T::AccountId) -> Vec<BondUnitPackageOf<T>> {
        BondUnitPackageRegistry::<T>::get(bond, bondholder)
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

    #[allow(dead_code)]
    fn purge_expired_bondunit_lots(_before: T::Moment) {}

    #[cfg(test)]
    fn get_coupon_yields(id: &BondId) -> Vec<PeriodYield> {
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
        // period should be ended up before we can calc it
        if bond_yields.len() >= period {
            // term hasn't come yet (if period=0 )
            // or current period has been calculated
            return false;
        }

        // @TODO refactor. use `mutate` method instead  of get+insert
        let reports = BondImpactReport::<T>::get(id);
        assert!(reports.len() >= period);
        // interest rate.
        // @TODO optimize calculation by using cached value stored in bond_yield struct
        let mut interest_rate = Self::calc_bond_interest_rate(bond, &reports, bond_yields.len());
        // get last accrued coupon yield
        let mut total_yield = bond_yields
            .last()
            .map(|period_yield| period_yield.total_yield)
            .unwrap_or(0);

        while bond_yields.len() < period {
            // calculate yield for period equal to bond_yields.len()
            let period_coupon_yield: EverUSDBalance =
                match bond.period_desc(bond_yields.len() as BondPeriodNumber) {
                    Some(period_desc) => {
                        // for every bond bondholder
                        BondUnitPackageRegistry::<T>::iter_prefix(id)
                            .map(|(_bondholder, packages)| {
                                // for every package
                                packages
                                    .iter()
                                    .map(|package| {
                                        // @TODO use checked arithmetics
                                        package.bond_units as EverUSDBalance
                                            * bond.inner.bond_units_base_price
                                            / 1000000_u64
                                            * period_desc.duration(package.acquisition)
                                                as EverUSDBalance
                                            / INTEREST_RATE_YEAR
                                            * interest_rate as EverUSDBalance
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

            let report: &BondImpactReportStructOf<T> = &reports[bond_yields.len()];
            interest_rate = if report.signed {
                bond.interest_rate(report.impact_data)
            } else {
                // if report missed add penalty rate value to the interest rate value of the previous period
                min(
                    bond.inner.interest_rate_margin_cap,
                    interest_rate + bond.inner.interest_rate_penalty_for_missed_report,
                )
            };
        }
        // save current liability in bond_credit field
        bond.bond_credit = total_yield;
        BondCouponYield::insert(id, bond_yields);
        true
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
        //let last = ensure_active!(bond_yields.last(), 0 );
        let current_coupon_yield = min(bond.bond_debit, bond.bond_credit);
        // @TODO replace with `mutate` method
        let mut last_bondholder_coupon_yield = BondLastCouponYield::<T>::get(id, bondholder);
        assert!(current_coupon_yield >= last_bondholder_coupon_yield.coupon_yield);
        assert!(bond_yields.len() > last_bondholder_coupon_yield.period_num as usize);

        if last_bondholder_coupon_yield.coupon_yield == current_coupon_yield {
            // no more coupon yield
            return 0;
        }
        let mut coupon_yield = 0;
        for (i, bond_yield) in bond_yields
            .iter()
            .enumerate()
            .skip(last_bondholder_coupon_yield.period_num as usize)
        {
            let new_coupon_yield = if i == bond_yields.len() - 1 {
                current_coupon_yield - last_bondholder_coupon_yield.coupon_yield
            } else {
                let cy = last_bondholder_coupon_yield.coupon_yield;
                last_bondholder_coupon_yield.coupon_yield = bond_yields[i + 1].total_yield;
                bond_yields[i + 1].total_yield - cy
            };

            if new_coupon_yield > 0 {
                let period_desc = bond.period_desc(i as BondPeriodNumber).unwrap();
                let total_yield = bond_yield.total_yield
                    - if i == 0 {
                        0
                    } else {
                        bond_yields[i - 1].total_yield
                    };

                assert!(new_coupon_yield <= total_yield);

                BondUnitPackageRegistry::<T>::mutate(id, &bondholder, |packages| {
                    for package in packages.iter_mut() {
                        let t = package.bond_units as EverUSDBalance
                            * bond.inner.bond_units_base_price
                            / 1000000_u64
                            * period_desc.duration(package.acquisition) as EverUSDBalance
                            / INTEREST_RATE_YEAR
                            * bond_yield.interest_rate as EverUSDBalance;

                        let package_coupon_yield = new_coupon_yield * t / total_yield;
                        coupon_yield += package_coupon_yield;

                        package.coupon_yield += package_coupon_yield;
                    }
                });
            }
        }
        bond.coupon_yield += coupon_yield;
        last_bondholder_coupon_yield.period_num = (bond_yields.len() - 1) as BondPeriodNumber;

        BondLastCouponYield::<T>::insert(id, &bondholder, last_bondholder_coupon_yield);

        coupon_yield
    }

    /// Returns effective coupon interest rate for `period`
    /// common complexity O(1), O(N) in worst case then no reports was released
    pub fn calc_bond_interest_rate(
        bond: &BondStructOf<T>,
        reports: &[BondImpactReportStructOf<T>],
        period: usize,
    ) -> BondInterest {
        assert!(reports.len() >= period);

        let mut missed_periods = 0;
        let mut interest: BondInterest = bond.inner.interest_rate_start_period_value;
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
        BondImpactReport::<T>::try_mutate(&bond, |reports| -> DispatchResult {
            let index = period as usize;

            reports[index].signed = true;
            reports[index].impact_data = impact_data;

            Ok(())
        })
    }
}
