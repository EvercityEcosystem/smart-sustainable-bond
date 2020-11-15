use crate::bond::{BondInterest, BondPeriod, BondPeriodNumber, BondStruct};
use crate::{EverUSDBalance, DAY_DURATION};
use frame_support::{
    codec::{Decode, Encode},
    sp_runtime::RuntimeDebug,
};

// ... |         period            |   ...
// --- | ------------------------- | -------------...
//     |                  |        |          |
//   start              report   reset    interest pay
//    >----------------------------< coupon accrual
// report release period  >--------<
//              coupon pay period  >----------<

pub struct PeriodDescr {
    pub start_period: BondPeriod,            // sec from activation
    pub impact_data_send_period: BondPeriod, // sec from activation
    pub payment_period: BondPeriod,          // sec from activation
    #[allow(dead_code)]
    pub interest_pay_period: BondPeriod, // sec from activation
}

impl PeriodDescr {
    pub fn duration(&self, moment: BondPeriod) -> BondPeriod {
        if moment <= self.start_period {
            (self.payment_period - self.start_period) / DAY_DURATION
        } else if moment >= self.payment_period {
            0
        } else {
            (self.payment_period - moment) / DAY_DURATION
        }
    }
}

/// Struct, storing per-period coupon_yield and effective interest_rate for given bond
#[derive(Encode, Decode, Clone, Default, PartialEq, RuntimeDebug)]
pub struct PeriodYield {
    /// bond cumulative accrued yield for this period
    pub total_yield: EverUSDBalance,
    /// bond fund to pay off coupon yield for this period
    pub coupon_yield_before: EverUSDBalance,
    /// effective interest rate for current period
    pub interest_rate: BondInterest,
}

pub struct PeriodIterator<'a, AccountId, Moment, Hash> {
    bond: &'a BondStruct<AccountId, Moment, Hash>,
    index: BondPeriodNumber,
}

impl<'a, AccountId, Moment, Hash> PeriodIterator<'a, AccountId, Moment, Hash> {
    pub fn new(bond: &'a BondStruct<AccountId, Moment, Hash>) -> Self {
        PeriodIterator { bond, index: 0 }
    }
    pub fn starts_with(
        bond: &'a BondStruct<AccountId, Moment, Hash>,
        index: BondPeriodNumber,
    ) -> Self {
        PeriodIterator { bond, index }
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
