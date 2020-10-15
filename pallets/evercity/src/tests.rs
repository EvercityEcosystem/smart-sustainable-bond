use crate::{Module, Error, Event, mock::*, EvercityAccountStruct, AccountRegistry};
use frame_support::{assert_ok, assert_noop};

pub type Evercity = Module<TestRuntime>;
use crate::sp_api_hidden_includes_decl_storage::hidden_include::StorageMap;


///////////////////////////////////////////////////////////////////////////////////////////
// Test uses pack of accounts, pre-set in new_test_ext in mock.rs:                       //
// (1, EvercityAccountStruct { roles: 1u8, identity: 10u64}), // MASTER    (accountID: 1)//                              
// (2, EvercityAccountStruct { roles: 2u8, identity: 20u64}), // CUSTODIAN (accountID: 2)//                            
// (3, EvercityAccountStruct { roles: 4u8, identity: 30u64}), // EMITENT   (accountID: 3)//                              
// (4, EvercityAccountStruct { roles: 8u8, identity: 40u64}), // INVESTOR  (accountId: 4)//
///////////////////////////////////////////////////////////////////////////////////////////

#[test]
fn it_returns_true_for_correct_role_checks() {
	new_test_ext().execute_with(|| {
		assert_eq!(Evercity::account_is_master(&1), true);
		assert_eq!(Evercity::account_is_custodian(&2), true);
		assert_eq!(Evercity::account_is_emitent(&3), true);
		assert_eq!(Evercity::account_is_investor(&4), true);
	});
}

#[test]
fn it_returns_false_for_incorrect_role_checks() {
	new_test_ext().execute_with(|| {
		// Dispatch a signed extrinsic.
		//assert_ok!(AccountRegistry::insert(Origin::signed(1), EvercityAccountStruct {roles: 1u8, identity: 67u64}));
		// Read pallet storage and assert an expected result.
		assert_eq!(Evercity::account_is_investor(&1), false);
		assert_eq!(Evercity::account_is_emitent(&2), false);
		assert_eq!(Evercity::account_is_custodian(&3), false);
		assert_eq!(Evercity::account_is_master(&4), false);
	});
}

#[test]
fn it_works() {
	new_test_ext().execute_with(|| {
		// Dispatch a signed extrinsic.
		//assert_ok!(AccountRegistry::insert(Origin::signed(1), EvercityAccountStruct {roles: 1u8, identity: 67u64}));
		// Read pallet storage and assert an expected result.
		assert!(true);
	});
}


/*
#[test]
fn correct_error_for_none_value() {
	new_test_ext().execute_with(|| {
		// Ensure the expected error is thrown when no value is present.
		assert_noop!(
			TemplateModule::cause_error(Origin::signed(1)),
			Error::<Test>::NoneValue
		);
	});
}
*/
