use crate::{EverUSDBalance, Expired, MIN_BOND_DURATION};
#[cfg(feature = "std")]
use core::cmp::{Eq, PartialEq};
use frame_support::{
    codec::{Decode, Encode},
    sp_runtime::{
        traits::{AtLeast32Bit, SaturatedConversion, UniqueSaturatedInto},
        RuntimeDebug,
    },
};
use period::{PeriodDescr, PeriodIterator};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_core::sp_std::cmp::min;

#[cfg(test)]
pub mod ledger;
pub mod period;

// seconds in 1 DAY
pub const DEFAULT_DAY_DURATION: u32 = 86400;
pub const MIN_PAYMENT_PERIOD: BondPeriod = 7;

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

impl Eq for BondId {}

impl core::ops::Deref for BondId {
    type Target = [u8; 8];
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

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
pub type BondPeriod = u32;
/// The number of Bond units,
pub type BondUnitAmount = u32;
/// Annual coupon interest rate as 1/100000 of par value
pub type BondInterest = u32;
/// Bond period numerator
pub type BondPeriodNumber = u32;

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
    /// when Issuer should release regular impact report (confirmed by Auditor)
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
    /// Period when Issuer should pay off coupon interests, sec
    pub interest_pay_period: BondPeriod,

    /// Period from activation when effective interest rate
    /// invariably equals to interest_rate_start_period_value, sec
    pub start_period: BondPeriod,

    /// This is "main" recalcualtion period of bond. Each payment_period:
    ///  - impact_data is sent to bond and confirmed by Auditor (while impact_data_send_period is active)
    ///  - coupon interest rate is recalculated for next payment_period
    ///  - required coupon interest payment is sent to bond by Issuer (while interest_pay_period is active)
    pub payment_period: BondPeriod,

    /// The number of periods from active_start_date (when bond becomes active,
    /// all periods and interest rate changes begin to work, funds become available for Issuer)
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

pub type BondInnerStructOf<T> =
    BondInnerStruct<<T as pallet_timestamp::Trait>::Moment, <T as frame_system::Trait>::Hash>;

#[inline]
fn is_period_multiple_of_day(period: BondPeriod, day_duration: BondPeriod) -> bool {
    (period % day_duration) == 0
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
    pub fn is_valid(&self, day_duration: BondPeriod) -> bool {
        self.bond_units_mincap_amount > 0
            && self.bond_units_maxcap_amount >= self.bond_units_mincap_amount
            && self.payment_period >= MIN_PAYMENT_PERIOD * day_duration
            && self.impact_data_send_period <= self.payment_period
            && is_period_multiple_of_day(self.payment_period, day_duration)
            && is_period_multiple_of_day(self.start_period, day_duration)
            && is_period_multiple_of_day(self.impact_data_send_period, day_duration)
            && is_period_multiple_of_day(self.bond_finishing_period, day_duration)
            && is_period_multiple_of_day(self.interest_pay_period, day_duration)
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

/// Main bond struct, storing all data about given bond
/// Consists of:
///  - issuance-related, inner part (BondInnerStruct): financial and impact data parameters, related to issuance of bond
///  - working part: bond state, connected accounts, raised and issued amounts, dates, etc
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, Clone, Default, PartialEq, RuntimeDebug)]
pub struct BondStruct<AccountId, Moment, Hash> {
    pub inner: BondInnerStruct<Moment, Hash>,
    /// bond issuer account
    pub issuer: AccountId,
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
    pub coupon_yield: EverUSDBalance,
}

pub type BondStructOf<T> = BondStruct<
    <T as frame_system::Trait>::AccountId,
    <T as pallet_timestamp::Trait>::Moment,
    <T as frame_system::Trait>::Hash,
>;

impl<AccountId, Moment, Hash> BondStruct<AccountId, Moment, Hash> {
    /// Returns nominal value of unit_amount Bond units
    #[inline]
    pub fn par_value(&self, unit_amount: BondUnitAmount) -> EverUSDBalance {
        unit_amount as EverUSDBalance * self.inner.bond_units_base_price as EverUSDBalance
    }
    /// Returns bond unpaid unliabilities
    pub fn get_debt(&self) -> EverUSDBalance {
        if self.bond_credit > self.bond_debit {
            self.bond_credit - self.bond_debit
        } else {
            0
        }
    }
    /// Returns the number of  tokens available for issuer
    pub fn get_free_balance(&self) -> EverUSDBalance {
        if self.bond_debit > self.bond_credit {
            self.bond_debit - self.bond_credit
        } else {
            0
        }
    }
    /// Increase bond fund (credit + debit)
    pub fn increase(&mut self, amount: EverUSDBalance) {
        self.bond_credit += amount;
        self.bond_debit += amount;
    }
    /// Decrease bond fund (credit + debit)
    pub fn decrease(&mut self, amount: EverUSDBalance) {
        self.bond_credit -= amount;
        self.bond_debit -= amount;
    }

    #[inline]
    pub fn get_periods(&self) -> BondPeriodNumber {
        if self.inner.start_period == 0 {
            self.inner.bond_duration
        } else {
            self.inner.bond_duration + 1
        }
    }

    #[allow(dead_code)]
    pub fn iter_periods(&self) -> PeriodIterator<'_, AccountId, Moment, Hash> {
        PeriodIterator::new(self)
    }

    /// Returns  time limits of the period
    pub fn period_desc(&self, period: BondPeriodNumber) -> Option<PeriodDescr> {
        PeriodIterator::starts_with(&self, period).next()
    }

    // @TODO rename this method to calc_effective_interest_rate()
    /// Calculate coupon effective interest rate using impact_data
    /// This method moves interest_rate up and down when good or bad impact_data
    /// is sent to bond
    pub fn interest_rate(&self, impact_data: u64) -> BondInterest {
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
    pub fn time_passed_after_activation(
        &self,
        now: Moment,
    ) -> Option<(BondPeriod, BondPeriodNumber)> {
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

/// Struct, accumulating per-account coupon_yield for each period num
#[derive(Encode, Decode, Clone, Default, PartialEq, RuntimeDebug)]
pub struct AccountYield {
    pub coupon_yield: EverUSDBalance,
    pub period_num: BondPeriodNumber,
}

/// Pack of bond units, bought at given time, belonging to given Bearer
/// Created when performed a deal to aquire bond uints (booking, buy from bond, buy from market)
/// Bond units that bondholder acquire
#[derive(Encode, Decode, Clone, Default, PartialEq, RuntimeDebug)]
pub struct BondUnitPackage {
    /// amount of bond units
    pub bond_units: BondUnitAmount,
    /// acquisition moment (seconds after bond start date)
    pub acquisition: BondPeriod,
    /// paid coupon yield
    pub coupon_yield: EverUSDBalance,
}

// pub type BondUnitPackageOf<T> = BondUnitPackage<<T as pallet_timestamp::Trait>::Moment>;

/// Struct with impact_data sent to bond. In the future can become
/// more complicated for other types of impact_data and processing logic.
/// Field "signed" is set to true by Auditor, when impact_data is verified.
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, Clone, Default, PartialEq, RuntimeDebug)]
pub struct BondImpactReportStruct {
    pub create_date: BondPeriod,
    pub impact_data: u64,
    pub signed: bool,
}

//pub type BondImpactReportStructOf<T> =
//    BondImpactReportStruct<<T as pallet_timestamp::Trait>::Moment>;
/// Struct, representing pack of bond units for sale
/// Can include target bearer (to sell bond units only to given person)
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, Clone, Default, Eq, PartialEq, RuntimeDebug)]
pub struct BondUnitSaleLotStruct<AccountId, Moment> {
    /// Sale lot is available for buy only before this deadline
    pub deadline: Moment,
    /// If set (can be empty) - then buying of this lot is possible
    /// only for new_bondholder
    pub new_bondholder: AccountId,
    /// Amount of bond units to sell
    pub bond_units: BondUnitAmount,
    /// Total price of this lot
    pub amount: EverUSDBalance,
}

impl<AccountId, Moment: core::cmp::PartialOrd> Expired<Moment>
    for BondUnitSaleLotStruct<AccountId, Moment>
{
    fn is_expired(&self, now: Moment) -> bool {
        self.deadline < now
    }
}

pub type BondUnitSaleLotStructOf<T> = BondUnitSaleLotStruct<
    <T as frame_system::Trait>::AccountId,
    <T as pallet_timestamp::Trait>::Moment,
>;
