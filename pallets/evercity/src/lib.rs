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


/// Structure, created by Emitent or Investor to receive EverUSD on her balance
/// by paying USD to Custodian. Then Custodian confirms request, adding corresponding
/// amount to mint request creator's balance
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

/// Structure, created by Emitent or Investor to burn EverUSD on her balance
/// and receive corresponding amount of USD from Custodian. 
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

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, Clone, RuntimeDebug, PartialEq)]
#[allow(non_camel_case_types)]
pub enum BondImpactType {
    POWER_GENERATED,
    CO2_EMISSIONS_REDUCTION,
}

impl Default for BondImpactType {
    fn default() -> Self {
        BondImpactType::POWER_GENERATED
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

/// Inner part of BondStruct, containing parameters, related to
/// calculation of coupon interest rate using impact data, sent to bond.
/// This part of bond data can be configured only at BondState::PREPARE
/// and cannot be changed when Bond Units sell process is started
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, Clone, Default, PartialEq, RuntimeDebug)]
pub struct BondInnerStruct<Moment, Hash> {
    // bond document hashes
    /// Merkle root hash of general purpose documents pack of bond
    pub docs_pack_root_hash_main: Hash,
    /// Merkle root hash of legal documents pack of bond
    pub docs_pack_root_hash_legal: Hash,
    /// Merkle root hash of financial documents pack of bond
    pub docs_pack_root_hash_finance: Hash,
    /// Merkle root hash of technical documents pack of bond
    pub docs_pack_root_hash_tech: Hash,

    // bond impact parameters
    /// Type of data, sent to bond each payment_period.
    /// It can be amount of power generated or CO2 emissions avoided (more types will be added)
    /// This value affects the interest_rate calculation logic
    /// (now all types have same linear dependency)
    pub impact_data_type: BondImpactType,
    /// Base value Now, all types has same interest_rate calculation logic
    /// greater the value -> lower the interest_rate and vice-versa
    pub impact_data_baseline: u64,

    // Coupon interest regulatory options
    /// Cap of impact_data value (absolute value). Values more then cap
    /// are considered equal to impact_data_max_deviation_cap
    /// when calculating coupon interest_rate depending on impact_data
    pub impact_data_max_deviation_cap: u64,
    /// Floor of impact_data value (absolute value). Values less then floor
    /// are considered equal to impact_data_max_deviation_floor
    /// when calculating coupon interest_rate depending on impact_data
    pub impact_data_max_deviation_floor: u64,
    /// Amount of seconds before end of a payment_period
    /// when Emitent should release regular impact report (confirmed by Auditor)
    pub impact_data_send_period: BondPeriod,
    /// Penalty, adding to interest rate when impact report was not
    /// released during impact_data_send_period, ppm
    pub interest_rate_penalty_for_missed_report: BondInterest,
    /// Base coupon interest rate, ppm. All changes of interest_rate
    /// during payment periods are based on this value, ppm
    pub interest_rate_base_value: BondInterest,
    /// Upper margin of interest_rate. Interest rate cannot
    /// be more than this value, ppm
    pub interest_rate_margin_cap: BondInterest,
    /// Lower margin of interest_rate. Interest rate cannot
    /// be less than this value, ppm
    pub interest_rate_margin_floor: BondInterest,
    /// Interest rate during the start_periodm when interest rate is constant
    /// (from activation to first payment period), ppm
    pub interest_rate_start_period_value: BondInterest,
    /// Period when Emitent should pay off coupon interests, sec
    pub interest_pay_period: BondPeriod,

    /// Period from activation when effective interest rate
    /// invariably equals to interest_rate_start_period_value, sec
    pub start_period: BondPeriod,

    /// This is "main" recalcualtion period of bond. Each payment_period:
    ///  - impact_data is sent to bond and confirmed by Auditor (while impact_data_send_period is active)
    ///  - coupon interest rate is recalculated for next payment_period
    ///  - required coupon interest payment is sent to bond by Emitent (while interest_pay_period is active)
    pub payment_period: BondPeriod,

    /// The number of periods from active_start_date (when bond becomes active,
    /// all periods and interest rate changes begin to work, funds become available for Emitent)
    /// until maturity date (when full bond debt must be paid).
    /// (bond maturity period = start_period + bond_duration * payment_period)
    pub bond_duration: BondPeriodNumber,

    /// Period from maturity date until full repayment.
    /// After this period bond can be moved to BondState::BANKRUPT, sec
    pub bond_finishing_period: BondPeriod,

    /// Minimal amount(mincap_amount) of bond units should be raised up to this date,
    /// otherwise bond can be withdrawn by issuer back to BondState::PREPARE
    pub mincap_deadline: Moment,
    /// Minimal amount of bond units, that should be raised
    pub bond_units_mincap_amount: BondUnitAmount,
    /// Maximal amount of bond units, that can be raised durill all bond lifetime
    pub bond_units_maxcap_amount: BondUnitAmount,

    /// Base price of Bond Unit
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

// ... |         period            |   ...
// --- | ------------------------- | -------------...
//     |                  |        |          |
//   start              report   reset    interest pay
//    >----------------------------< coupon accrual
// report release period  >--------<
//              coupon pay period  >----------<
#[cfg_attr(feature = "std", derive(Debug))]
struct PeriodDescr {
    start_period: BondPeriod,            // sec from activation
    impact_data_send_period: BondPeriod, // sec from activation
    payment_period: BondPeriod,          // sec from activation
    #[allow(dead_code)]
    interest_pay_period: BondPeriod, // sec from activation
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

/// Struct, accumulating per-account coupon_yield for each period num
#[derive(Encode, Decode, Clone, Default, PartialEq, RuntimeDebug)]
pub struct AccountYield {
    coupon_yield: EverUSDBalance,
    period_num: BondPeriodNumber,
}

/// Struct, storing per-period coupon_yield and effective interest_rate for given bond
#[derive(Encode, Decode, Clone, Default, PartialEq, RuntimeDebug)]
pub struct PeriodYield {
    /// bond cumulative accrued yield for this period
    total_yield: EverUSDBalance,
    /// bond fund to pay off coupon yield for this period
    coupon_yield_before: EverUSDBalance,
    /// effective interest rate for current period
    interest_rate: BondInterest,
}

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

/// Main bond struct, storing all data about given bond
/// Consists of:
///  - issuance-related, inner part (BondInnerStruct): financial and impact data parameters, related to issuance of bond
///  - working part: bond state, connected accounts, raised and issued amounts, dates, etc
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, Clone, Default, RuntimeDebug)]
pub struct BondStruct<AccountId, Moment, Hash> {
    pub inner: BondInnerStruct<Moment, Hash>,
    /// bond issuer account
    pub emitent: AccountId,
    // #Auxiliary roles
    /// bond manager account
    pub manager: AccountId,
    /// bond auditor
    pub auditor: AccountId,
    /// bond impact data reporter
    pub impact_reporter: AccountId,

    /// total amount of issued bond units
    pub issued_amount: BondUnitAmount,

    // #Timestamps
    /// Moment, when bond was created first time (moved to BondState::PREPARE)
    pub creation_date: Moment,
    /// Moment, when bond was opened for booking (moved to BondState::BOOKING)
    pub booking_start_date: Moment,
    /// Moment, when bond became active (moved to BondState::ACTIVE)
    pub active_start_date: Moment,

    /// Bond current state (PREPARE, BOOKING, ACTIVE, BANKRUPT, FINISHED)
    pub state: BondState,

    // #Bond ledger
    
	/// Bond fund, keeping EverUSD sent to bond
    pub bond_debit: EverUSDBalance,
    /// Bond liabilities: amount of EverUSD bond needs to pay to Bond Units bearers
    pub bond_credit: EverUSDBalance,

    // free balance is difference between bond_debit and bond_credit
    
	/// Ever-increasing coupon fund which was distributed among bondholders.
    /// Undistributed bond fund is equal to (bond_debit - coupon_yield)
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
    /// Increase bond fund (credit + debit)
    fn increase(&mut self, amount: EverUSDBalance) {
        self.bond_credit += amount;
        self.bond_debit += amount;
    }
    /// Decrease bond fund (credit + debit)
    fn decrease(&mut self, amount: EverUSDBalance) {
        self.bond_credit -= amount;
        self.bond_debit -= amount;
    }

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
    fn period_desc(&self, period: BondPeriodNumber) -> Option<PeriodDescr> {
        let mut iter = PeriodIterator::new(&self);
        iter.index = period;
        iter.next()
    }

	// @TODO rename this method to calc_effective_interest_rate()
    /// Calculate coupon effective interest rate using impact_data
	/// This method moves interest_rate up and down when good or bad impact_data
	/// is sent to bond
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

/// Pack of bond units, bought at given time, belonging to given Bearer
/// Created when performed a deal to aquire bond uints (booking, buy from bond, buy from market)
#[derive(Encode, Decode, Clone, Default, PartialEq, RuntimeDebug)]
pub struct BondUnitPackage<Moment> {
    /// amount of bond units
    bond_units: BondUnitAmount,
    /// acquisition moment (seconds after bond start date)
    acquisition: BondPeriod,
    /// paid coupon yield
    coupon_yield: EverUSDBalance,
	/// moment of creation
    create_date: Moment,
}
type BondUnitPackageOf<T> = BondUnitPackage<<T as pallet_timestamp::Trait>::Moment>;

/// Struct with impact_data sent to bond. In the future can become
/// more complicated for other types of impact_data and processing logic. 
/// Field "signed" is set to true by Auditor, when impact_data is verified.
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, Clone, Default, RuntimeDebug)]
pub struct BondImpactReportStruct<Moment> {
    create_date: Moment,
    impact_data: u64,
    signed: bool,
}

type BondImpactReportStructOf<T> = BondImpactReportStruct<<T as pallet_timestamp::Trait>::Moment>;
/// Struct, representing pack of bond units for sale
/// Can include target bearer (to sell bond units only to given person)
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, Clone, Default, Eq, PartialEq, RuntimeDebug)]
pub struct BondUnitSaleLotStruct<AccountId, Moment> {
    /// Sale lot is available for buy only before this deadline
    deadline: Moment,
    /// If set (can be empty) - then buying of this lot is possible
    /// only for new_bondholder
    new_bondholder: AccountId,
    /// Amount of bond units to sell
    bond_units: BondUnitAmount,
    /// Total price of this lot
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
                double_map hasher(blake2_128_concat) BondId, hasher(blake2_128_concat) T::AccountId => Vec<BondUnitPackageOf<T>>;

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
        BondActivated(AccountId, BondId),
        BondWithdrawal(AccountId, BondId),
        BondImpactReportReceived(AccountId, BondId),
        BondRedeemed(AccountId, BondId),
        BondBankrupted(AccountId, BondId),

        BondWithdrawEverUSD(AccountId, BondId, EverUSDBalance),
        BondDepositEverUSD(AccountId, BondId, EverUSDBalance),

        BondSale(AccountId, BondId, u32),
        BondGiveBack(AccountId, BondId, u32),

        BondImpactReportIssued(AccountId, BondId),
        BondImpactReportSigned(AccountId, BondId),
        BondCouponYield(BondId, EverUSDBalance),

        BondNewSaleLot(AccountId, BondId, BondUnitSaleLotStruct),
    }
);

decl_error! {
    pub enum Error for Module<T: Trait> {
        NoneValue,

        /// Account tried to use more EverUSD than was available on the balance
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
        /// Arguments:  origin: AccountId - transaction caller
        ///             who: AccountId - account to modify
        ///             role: u8 - role(s) of account (see ALL_ROLES_MASK for allowed roles)
        ///             identity: u64 - reserved field for integration with external platforms
        /// Access: Master role
        ///
        /// Modifies existing account, assigning new role(s) or identity to it
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

        // Token balances manipulation functions

        /// Method: token_mint_request_create_everusd(origin, amount_to_mint: EverUSDBalance)
        /// Arguments:  origin: AccountId - transaction caller
        ///             amount_to_mint: EverUSDBalance - amount of tokens to mint 
        /// Access: Investor or Emitent role
        ///
        /// Creates a request to mint given amount of EverUSD tokens on caller's balance.
        /// Custodian account confirms request after receiving payment in USD from target account's owner 
        /// It's possible to create only one request per account. Mint request has a time-to-live
        /// and becomes invalidated after it.
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

        /// Method: token_mint_request_revoke_everusd(origin)
        /// Arguments: origin: AccountId - transaction caller
        /// Access: Investor or Emitent role
        ///
        /// Revokes and deletes currently existing mint request, created by caller's account
        #[weight = 5_000]
        fn token_mint_request_revoke_everusd(origin) -> DispatchResult {
            let _caller = ensure_signed(origin)?;
            ensure!(MintRequestEverUSD::<T>::contains_key(&_caller), Error::<T>::MintRequestDoesntExist);
            let _amount = MintRequestEverUSD::<T>::get(&_caller).amount;
            MintRequestEverUSD::<T>::remove(&_caller);
            Self::deposit_event(RawEvent::MintRequestRevoked(_caller, _amount));
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

        /// Method: token_mint_request_decline_everusd(origin, who: T::AccountId)
        /// Arguments:  origin: AccountId - transaction caller
        ///             who: AccountId - target account
        /// Access: Custodian role
        ///
        /// Declines and deletes the mint request of account (Custodian)
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

        /// Method: token_burn_request_create_everusd(origin, amount_to_burn: EverUSDBalance)
        /// Arguments:  origin: AccountId - transaction caller
        ///             amount_to_burn: EverUSDBalance - amount of tokens to burn
        /// Access: Investor or Emitent role
        ///
        /// Creates a request to burn given amount of EverUSD tokens on caller's balance.
        /// Custodian account confirms request after sending payment in USD to target account's owner 
        /// It's possible to create only one request per account. Burn request has a time-to-live
        /// and becomes invalidated after it.
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

        /// Method: token_burn_request_revoke_everusd(origin)
        /// Arguments: origin: AccountId - transaction caller
        /// Access: Investor or Emitent role
        ///
        /// Revokes and deletes currently existing burn request, created by caller's account
        #[weight = 5_000]
        fn token_burn_request_revoke_everusd(origin) -> DispatchResult {
            let _caller = ensure_signed(origin)?;
            ensure!(BurnRequestEverUSD::<T>::contains_key(&_caller), Error::<T>::BurnRequestDoesntExist);
            let _amount = BurnRequestEverUSD::<T>::get(&_caller).amount;
            BurnRequestEverUSD::<T>::remove(&_caller);
            Self::deposit_event(RawEvent::BurnRequestRevoked(_caller, _amount));
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

        /// Method: token_burn_request_decline_everusd(origin, who: T::AccountId)
        /// Arguments:  origin: AccountId - transaction caller
        ///             who: AccountId - target account
        /// Access: Custodian role
        ///
        /// Declines and deletes the burn request of account (Custodian)
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
        /// Access: Emitent role
        ///
        /// Creates new bond with given BondId (8 bytes) and pack of parameters, set by BondInnerStruct.
        /// Bond is created in BondState::PREPARE, and can be modified many times until it becomes ready
        /// for next BondState::BOOKING, when most of BondInnerStruct parameters cannot be changed, and
        /// Investors can buy bond units
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
        ///            acc: AccountId - assignee account
        /// Access: Master role
        ///
        /// Assigns target account to be the manager of the bond. Manager can make
        /// almost the same actions with bond as Emitent, instead of most important,
        /// helping Emitent to manage bond parameters, work with documents, etc...
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


                if matches!(item.state, BondState::ACTIVE | BondState::BANKRUPT) {
                    item.bond_debit += package_value;
                    // in BondState::ACTIVE or BondState::BANKRUPT received everusd
                    // can be forwarded to pay off the debt
                    Self::calc_and_store_bond_coupon_yield(&bond, &mut item, now);
                    // surplus to the emitent balance
                    let free_balance = item.get_free_balance();
                    if free_balance > 0 {
                        item.bond_debit -= free_balance;
                        Self::balance_add(&item.emitent, free_balance)?;
                    }
                }else{
                    // in BondState::PREPARE just increase assets and liabilities of the Bond
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
            {
                let item = BondRegistry::<T>::get(bond);
                ensure!(item.emitent == caller || item.impact_reporter == caller, Error::<T>::BondAccessDenied );
                ensure!(Self::is_report_in_time(&item, now, period), Error::<T>::BondOutOfOrder );
            }

            let index: usize = period as usize;
            BondImpactReport::<T>::try_mutate(&bond, |reports|->DispatchResult {

                ensure!(index < reports.len() && !reports[index].signed, Error::<T>::BondParamIncorrect );

                reports[index].create_date = now;
                reports[index].impact_data = impact_data;

                Self::deposit_event(RawEvent::BondImpactReportIssued( caller, bond ));
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
            let now = <pallet_timestamp::Module<T>>::get();
            Self::with_bond(&bond, |mut item|{
                ensure!( matches!(item.state, BondState::ACTIVE|BondState::BANKRUPT), Error::<T>::BondStateNotPermitAction );

                match item.time_passed_after_activation(now){
                    Some((_, period))  if period == item.get_periods() => (),
                    _ => return Err( Error::<T>::AccountNotAuthorized.into() ),
                };

                Self::calc_and_store_bond_coupon_yield(&bond, &mut item, now);
                // now bond_credit has total coupon yield
                // YTM = coupon yield + principal value
                let amount = item.bond_credit + item.par_value( item.issued_amount ) ;
                if amount <= item.bond_debit {
                    // withdraw free balance
                    Self::balance_add(&item.emitent, item.bond_debit - amount)?;
                    item.bond_debit = amount;
                }else{
                    let transfer = amount - item.bond_debit;
                    // pay off debt
                    Self::balance_sub(&item.emitent, transfer)?;
                    item.bond_debit+=transfer;
                    item.state = BondState::FINISHED;
                }
                item.bond_credit = amount;
                item.state = BondState::FINISHED;

                Self::deposit_event(RawEvent::BondRedeemed(caller, bond ));
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
                ensure!(item.get_debt()>0, Error::<T>::BondOutOfOrder );
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
            let _caller = ensure_signed(origin)?;

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
                ensure!( matches!(item.state , BondState::ACTIVE | BondState::BANKRUPT | BondState::FINISHED), Error::<T>::BondStateNotPermitAction);

                let now = <pallet_timestamp::Module<T>>::get();
                Self::calc_and_store_bond_coupon_yield(&bond, &mut item, now);

                let amount: EverUSDBalance = if item.emitent == caller {
                    // issuer withdraw bond fund
                    let amount = item.get_free_balance();
                    if amount>0{
                        Self::balance_add(&item.emitent, amount)?;
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
                ensure!(item.emitent == caller, Error::<T>::BondAccessDenied);

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
            // save  lots
            BondUnitPackageLot::<T>::insert(&bond, &caller, lots);
            Self::deposit_event(RawEvent::BondNewSaleLot(caller, bond, lot ));
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
        let reports = BondImpactReport::<T>::get(id);
        assert!(reports.len() >= period);
        // interest rate.
        // @TODO optimize calculation by using cached value stored in bond_yield struct
        let mut interest_rate = Self::calc_bond_interest_rate(bond, &reports, bond_yields.len());

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
        let bond_units = packages.iter().map(|package| package.bond_units).sum();

        let bond_yields = BondCouponYield::get(id);
        assert!(!bond_yields.is_empty());
        // calc coupon
        let mut total: EverUSDBalance = bond_yields
            .iter()
            .enumerate()
            .map(|(i, bond_yield)| {
                let period_desc = bond.period_desc(i as BondPeriodNumber).unwrap();
                packages
                    .iter()
                    .map(|package| {
                        package.bond_units as EverUSDBalance * bond.inner.bond_units_base_price
                            / 1000000_u64
                            * period_desc.duration(package.acquisition) as EverUSDBalance
                            / INTEREST_RATE_YEAR
                            * bond_yield.interest_rate as EverUSDBalance
                    })
                    .sum::<EverUSDBalance>()
            })
            .sum::<EverUSDBalance>();
        // substrate paid coupon
        total -= packages
            .iter()
            .map(|package| package.coupon_yield)
            .sum::<EverUSDBalance>();
        // add principal value
        total += bond.par_value(bond_units);
        Self::balance_add(bondholder, total).unwrap();

        total
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
