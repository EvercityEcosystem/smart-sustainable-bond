use crate::{EvercityAccountStructT, Trait};
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

impl Trait for TestRuntime {
    type Event = ();
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
            (3, 97000), // EMITENT
            (4, 96000), // INVESTOR
            (5, 95000), // AUDITOR
            (6, 10000), // INVESTOR
            (7, 10000), // EMITENT
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
                    roles: crate::MASTER_ROLE_MASK,
                    identity: 10u64,
                    create_time: 0,
                },
            ),
            (
                2,
                EvercityAccountStructT::<u64> {
                    roles: crate::CUSTODIAN_ROLE_MASK,
                    identity: 20u64,
                    create_time: 0,
                },
            ),
            (
                3,
                EvercityAccountStructT::<u64> {
                    roles: crate::EMITENT_ROLE_MASK,
                    identity: 30u64,
                    create_time: 0,
                },
            ),
            (
                4,
                EvercityAccountStructT::<u64> {
                    roles: crate::INVESTOR_ROLE_MASK,
                    identity: 40u64,
                    create_time: 0,
                },
            ),
            (
                5,
                EvercityAccountStructT::<u64> {
                    roles: crate::AUDITOR_ROLE_MASK,
                    identity: 50u64,
                    create_time: 0,
                },
            ),
            (
                6,
                EvercityAccountStructT::<u64> {
                    roles: crate::INVESTOR_ROLE_MASK,
                    identity: 60u64,
                    create_time: 0,
                },
            ),
            (
                7,
                EvercityAccountStructT::<u64> {
                    roles: crate::EMITENT_ROLE_MASK,
                    identity: 70u64,
                    create_time: 0,
                },
            ),
            (
                8,
                EvercityAccountStructT::<u64> {
                    roles: crate::MANAGER_ROLE_MASK,
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
