#![allow(clippy::from_over_into)]

use crate::*;
use frame_support::{
    assert_err, assert_ok, impl_outer_origin, parameter_types,
    sp_runtime::{testing::Header, traits::IdentityLookup, Perbill},
    weights::{
        constants::{BlockExecutionWeight, ExtrinsicBaseWeight},
        Weight,
    },
};
use sp_core::H256;

impl_outer_origin! {
        pub enum Origin for Test {}
}

pub type Balance = u64;
const UNIT: Balance = 1_000_000;

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Test;

parameter_types! {
    pub const BlockHashCount: u64 = 250;

    pub const MaximumBlockWeight: Weight = 2_000_000_000;
    pub const AvailableBlockRatio: Perbill = Perbill::from_percent(75);

    pub MaximumExtrinsicWeight: Weight = 1_000_000_000;
    pub const MaximumBlockLength: u32 = 1_000_000;
}

impl frame_system::Trait for Test {
    type Origin = Origin;
    type Index = u64;
    type BlockNumber = u64;
    type Call = ();
    type Hash = H256;
    type Hashing = frame_support::sp_runtime::traits::BlakeTwo256;
    type AccountId = u64;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type Event = ();
    type BlockHashCount = BlockHashCount;
    type Version = ();
    type PalletInfo = ();
    type AccountData = pallet_balances::AccountData<Balance>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type DbWeight = ();
    type BaseCallFilter = ();
    type SystemWeightInfo = ();
    type BlockExecutionWeight = BlockExecutionWeight;
    type ExtrinsicBaseWeight = ExtrinsicBaseWeight;
    type MaximumExtrinsicWeight = MaximumExtrinsicWeight;
    type MaximumBlockWeight = MaximumBlockWeight;
    type MaximumBlockLength = MaximumBlockLength;
    type AvailableBlockRatio = AvailableBlockRatio;
}

parameter_types! {
    pub const MaximumTransferValue: Balance = 10_000_000_000_000;
}

impl Trait for Test {
    type Event = ();
    type MaximumTransferValue = MaximumTransferValue;
    type Currency = Balances;
    type WeightInfo = ();
}

parameter_types! {
    pub const ExistentialDeposit: Balance = 0;
    pub const MaxLocks: u32 = 5;
}

impl pallet_balances::Trait for Test {
    type Balance = Balance;
    type Event = ();
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type WeightInfo = ();
    type MaxLocks = MaxLocks;
}

type Balances = pallet_balances::Module<Test>;
type System = frame_system::Module<Test>;
type EvercityTransfer = Module<Test>;

fn new_test_ext() -> frame_support::sp_io::TestExternalities {
    let mut storage = frame_system::GenesisConfig::default()
        .build_storage::<Test>()
        .unwrap();

    pallet_balances::GenesisConfig::<Test> {
        // Provide some initial balances
        balances: vec![(1, UNIT)],
    }
    .assimilate_storage(&mut storage)
    .unwrap();

    storage.into()
}

#[test]
fn test_spend_endowed_balance() {
    new_test_ext().execute_with(|| {
        assert_ok!(EvercityTransfer::transfer(Origin::signed(1), 2, 1000));
        assert_eq!(Balances::free_balance(2), 1000);
        assert_err!(
            Balances::transfer(Origin::signed(2), 3, 10),
            pallet_balances::Error::<Test, _>::LiquidityRestrictions
        );

        assert_ok!(EvercityTransfer::transfer(Origin::signed(1), 2, 1000));
        assert_eq!(Balances::free_balance(2), 2000);

        assert_ok!(Balances::transfer(Origin::signed(2), 3, 10));
    })
}
