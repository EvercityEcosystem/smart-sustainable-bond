use crate::{Module, Trait};
use pallet_balances;
use sp_core::H256;
use frame_support::{impl_outer_origin, parameter_types, weights::Weight};
use sp_runtime::{
	traits::{BlakeTwo256, IdentityLookup}, testing::Header, Perbill,
};

impl_outer_origin! {
	pub enum Origin for TestRuntime {}
}

// Configure a mock runtime to test the pallet.

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
	type ModuleToIndex = ();
	type AccountData = pallet_balances::AccountData<u64>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
    
}

impl Trait for TestRuntime {
	type Event = ();
}

impl pallet_balances::Trait for TestRuntime {                                                                                        
    type Balance = u64;                                                                                                       
    type Event = ();                                                                                                   
    type DustRemoval = ();                                                                                                    
    type ExistentialDeposit = ExistentialDeposit;                                                                             
    type AccountStore = System;                                                                          
    type WeightInfo = ();                                                                                                     
}

pub type System = frame_system::Module<TestRuntime>;
pub type Evercity = Module<TestRuntime>;
pub type Balances = pallet_balances::Module<TestRuntime>;

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = frame_system::GenesisConfig::default().build_storage::<TestRuntime>().unwrap();
	pallet_balances::GenesisConfig::<TestRuntime> {                                                                                  
        // Provide some initial balances                                                                                      
        balances: vec![ (1, 13000),
						(2, 11000),
						(3, 1000),
						(4, 3000),
						(5, 19000)
					  ],                                                            
    }                                                                                                                         
    .assimilate_storage(&mut t)                                                                                               
    .unwrap();
	
	/*
	crate::GenesisConfig {
		genesis_account_registry: vec![]
	}                                                                                                   
    .assimilate_storage(&mut t)                                                                            
    .unwrap();
	*/
    
	t.into()
}
