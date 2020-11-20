use crate::account::*;
use crate::{
    BondInnerStructOf, BondPeriod, BondStructOf, EverUSDBalance, EvercityAccountStructT, Trait,
    DEFAULT_DAY_DURATION,
};
use frame_support::{impl_outer_origin, parameter_types, weights::Weight};
use sp_core::H256;
use sp_runtime::{
    testing::Header,
    traits::{BlakeTwo256, IdentityLookup},
    Perbill,
};

impl_outer_origin! {
    pub enum Origin for TestRuntime {}
}

// Configure a mock runtime to test the pallet.
pub const MILLISECS_PER_BLOCK: u64 = 6000;
pub const SLOT_DURATION: u64 = MILLISECS_PER_BLOCK;
pub const EVERUSD_MAX_MINT_AMOUNT: EverUSDBalance = 60_000_000_000_000_000; // =60 million dollar

#[derive(Clone, Eq, PartialEq)]
pub struct TestRuntime;

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const MaximumBlockWeight: Weight = 1024;
    pub const MaximumBlockLength: u32 = 2 * 1024;
    pub const AvailableBlockRatio: Perbill = Perbill::from_percent(75);
    pub const ExistentialDeposit: u64 = 0;
}

impl frame_system::Trait for TestRuntime {
    type BaseCallFilter = ();
    type Origin = Origin;
    type Call = ();
    type Index = u64;
    type BlockNumber = u64;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = u64;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type Event = ();
    type BlockHashCount = BlockHashCount;
    type MaximumBlockWeight = MaximumBlockWeight;
    type DbWeight = ();
    type BlockExecutionWeight = ();
    type ExtrinsicBaseWeight = ();
    type MaximumExtrinsicWeight = MaximumBlockWeight;
    type MaximumBlockLength = MaximumBlockLength;
    type AvailableBlockRatio = AvailableBlockRatio;
    type Version = ();
    type PalletInfo = ();
    type AccountData = pallet_balances::AccountData<u64>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
}
parameter_types! {
    pub const BurnRequestTtl: u32 = DEFAULT_DAY_DURATION as u32 * 7 * 1000;
    pub const MintRequestTtl: u32 = DEFAULT_DAY_DURATION as u32 * 7 * 1000;
    pub const MaxMintAmount: EverUSDBalance = EVERUSD_MAX_MINT_AMOUNT;
    pub const DayDuration: BondPeriod = DEFAULT_DAY_DURATION;
}

impl Trait for TestRuntime {
    type Event = ();
    type BurnRequestTtl = BurnRequestTtl;
    type MintRequestTtl = MintRequestTtl;
    type MaxMintAmount = MaxMintAmount;
    type DayDuration = DayDuration;
}

parameter_types! {
    pub const MinimumPeriod: u64 = SLOT_DURATION / 2;
}

impl pallet_timestamp::Trait for TestRuntime {
    /// A timestamp: milliseconds since the unix epoch.
    type Moment = u64;
    type OnTimestampSet = ();
    type MinimumPeriod = MinimumPeriod;
    type WeightInfo = ();
}

parameter_types! {
    pub const MaxLocks: u32 = 50;
}

impl pallet_balances::Trait for TestRuntime {
    type Balance = u64;
    type Event = ();
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type WeightInfo = ();
    type MaxLocks = MaxLocks;
}

pub type System = frame_system::Module<TestRuntime>;
// pub type Evercity = Module<TestRuntime>;
// pub type Balances = pallet_balances::Module<TestRuntime>;

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = frame_system::GenesisConfig::default()
        .build_storage::<TestRuntime>()
        .unwrap();
    pallet_balances::GenesisConfig::<TestRuntime> {
        // Provide some initial balances
        balances: vec![
            (1, 99000), // MASTER
            (2, 98000), // CUSTODIAN
            (3, 97000), // ISSUER
            (4, 96000), // INVESTOR
            (5, 95000), // AUDITOR
            (6, 10000), // INVESTOR
            (7, 10000), // ISSUER
            (8, 10000), // MANAGER
            (9, 10000),
            (101, 1000), // random guy
            (102, 1000), // random guy
            (103, 1000), // random guy
        ],
    }
    .assimilate_storage(&mut t)
    .unwrap();

    super::GenesisConfig::<TestRuntime> {
        // Accounts for tests
        genesis_account_registry: vec![
            (
                1,
                EvercityAccountStructT::<u64> {
                    roles: MASTER_ROLE_MASK,
                    identity: 10u64,
                    create_time: 0,
                },
            ),
            (
                2,
                EvercityAccountStructT::<u64> {
                    roles: CUSTODIAN_ROLE_MASK,
                    identity: 20u64,
                    create_time: 0,
                },
            ),
            (
                3,
                EvercityAccountStructT::<u64> {
                    roles: ISSUER_ROLE_MASK,
                    identity: 30u64,
                    create_time: 0,
                },
            ),
            (
                4,
                EvercityAccountStructT::<u64> {
                    roles: INVESTOR_ROLE_MASK,
                    identity: 40u64,
                    create_time: 0,
                },
            ),
            (
                5,
                EvercityAccountStructT::<u64> {
                    roles: AUDITOR_ROLE_MASK,
                    identity: 50u64,
                    create_time: 0,
                },
            ),
            (
                6,
                EvercityAccountStructT::<u64> {
                    roles: INVESTOR_ROLE_MASK,
                    identity: 60u64,
                    create_time: 0,
                },
            ),
            (
                7,
                EvercityAccountStructT::<u64> {
                    roles: ISSUER_ROLE_MASK | INVESTOR_ROLE_MASK,
                    identity: 70u64,
                    create_time: 0,
                },
            ),
            (
                8,
                EvercityAccountStructT::<u64> {
                    roles: MANAGER_ROLE_MASK,
                    identity: 80u64,
                    create_time: 0,
                },
            ),
        ]
        .to_vec(),
    }
    .assimilate_storage(&mut t)
    .unwrap();

    t.into()
}

type BondInnerStruct = BondInnerStructOf<TestRuntime>;
type BondStruct = BondStructOf<TestRuntime>;

pub fn get_test_bond() -> BondStruct {
    BondStruct {
        inner: BondInnerStruct {
            docs_pack_root_hash_main: Default::default(),
            docs_pack_root_hash_legal: Default::default(),
            docs_pack_root_hash_finance: Default::default(),
            docs_pack_root_hash_tech: Default::default(),

            impact_data_type: Default::default(),
            impact_data_baseline: 20000_u64,
            impact_data_max_deviation_cap: 30000_u64,
            impact_data_max_deviation_floor: 14000_u64,
            interest_rate_penalty_for_missed_report: 400, // +0.4%

            interest_rate_base_value: 2000,   // 2.0%
            interest_rate_margin_cap: 4000,   // 4.0%
            interest_rate_margin_floor: 1000, // 1.0%
            interest_rate_start_period_value: 1900,
            start_period: 120 * DEFAULT_DAY_DURATION,
            payment_period: 30 * DEFAULT_DAY_DURATION, // every month (30 days)
            interest_pay_period: 7 * DEFAULT_DAY_DURATION, // up to 7 days after the new period started
            mincap_deadline: (20 * DEFAULT_DAY_DURATION * 1000) as u64,
            impact_data_send_period: 10 * DEFAULT_DAY_DURATION, // 10 days before next period
            bond_duration: 12,                                  //
            bond_finishing_period: 14 * DEFAULT_DAY_DURATION,

            bond_units_mincap_amount: 1000,
            bond_units_maxcap_amount: 1800,
            bond_units_base_price: 4_000_000_000_000,
        },

        issuer: 0,
        manager: 0,
        auditor: 0,
        impact_reporter: 0,

        issued_amount: 0,
        booking_start_date: Default::default(),
        active_start_date: Default::default(),
        creation_date: Default::default(),
        state: Default::default(),

        bond_debit: 0,
        bond_credit: 0,
        coupon_yield: 0,
    }
}
