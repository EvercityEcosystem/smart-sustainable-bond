use crate::mock::*;
use crate::{
    AccountRegistry, Error, Event, EvercityAccountStruct, Module, AUDITOR_ROLE_MASK,
    CUSTODIAN_ROLE_MASK, EMITENT_ROLE_MASK, INVESTOR_ROLE_MASK, MASTER_ROLE_MASK,
};
use frame_support::{assert_noop, assert_ok};

pub type Evercity = Module<TestRuntime>;

//////////////////////////////////////////////////////////////////////////////////////////////////////////
// Test uses pack of accounts, pre-set in new_test_ext in mock.rs:
// (1, EvercityAccountStruct { roles: MASTER_ROLE_MASK,     identity: 10u64}), // MASTER    (accountId: 1)
// (2, EvercityAccountStruct { roles: CUSTODIAN_ROLE_MASK,  identity: 20u64}), // CUSTODIAN (accountID: 2)
// (3, EvercityAccountStruct { roles: EMITENT_ROLE_MASK,    identity: 30u64}), // EMITENT   (accountID: 3)
// (4, EvercityAccountStruct { roles: INVESTOR_ROLE_MASK,   identity: 40u64}), // INVESTOR  (accountId: 4)
// (5, EvercityAccountStruct { roles: AUDITOR_ROLE_MASK,    identity: 50u64}), // AUDOTOR   (accountId: 5)
// (101+ : some external accounts
//////////////////////////////////////////////////////////////////////////////////////////////////////////

#[test]
fn it_returns_true_for_correct_role_checks() {
    new_test_ext().execute_with(|| {
        assert_eq!(Evercity::account_is_master(&1), true);
        assert_eq!(Evercity::account_is_custodian(&2), true);
        assert_eq!(Evercity::account_is_emitent(&3), true);
        assert_eq!(Evercity::account_is_investor(&4), true);
        assert_eq!(Evercity::account_is_auditor(&5), true);
    });
}

#[test]
fn it_returns_false_for_incorrect_role_checks() {
    new_test_ext().execute_with(|| {
        // Dispatch a signed extrinsic.
        //assert_ok!(AccountRegistry::insert(Origin::signed(1), EvercityAccountStruct {roles: 1u8, identity: 67u64}));
        // Read pallet storage and assert an expected result.
        assert_eq!(Evercity::account_is_auditor(&1), false);
        assert_eq!(Evercity::account_is_emitent(&2), false);
        assert_eq!(Evercity::account_is_investor(&3), false);
        assert_eq!(Evercity::account_is_custodian(&4), false);
        assert_eq!(Evercity::account_is_master(&5), false);
    });
}

#[test]
fn it_adds_new_account_with_correct_roles() {
    new_test_ext().execute_with(|| {
        assert_ok!(Evercity::account_add_with_role_and_data(
            Origin::signed(1),
            101,
            MASTER_ROLE_MASK,
            88u64
        ));
        assert_eq!(Evercity::account_is_master(&101), true);
        assert_eq!(Evercity::account_is_investor(&101), false);

        assert_ok!(Evercity::account_add_with_role_and_data(
            Origin::signed(1),
            102,
            AUDITOR_ROLE_MASK,
            89u64
        ));
        assert_eq!(Evercity::account_is_master(&102), false);
        assert_eq!(Evercity::account_is_auditor(&102), true);
    });
}
#[test]
fn it_correctly_sets_new_role_to_existing_account() {
    new_test_ext().execute_with(|| {
        // add new role to existing account (alowed only for master)
        assert_eq!(Evercity::account_is_emitent(&3), true);
        assert_ok!(Evercity::account_set_with_role_and_data(
            Origin::signed(1),
            3,
            AUDITOR_ROLE_MASK,
            88u64
        ));
        assert_eq!(Evercity::account_is_emitent(&3), true);
        assert_eq!(Evercity::account_is_auditor(&3), true);
        assert_eq!(Evercity::account_is_investor(&3), false);

        assert_eq!(Evercity::account_is_custodian(&2), true);
        assert_ok!(Evercity::account_set_with_role_and_data(
            Origin::signed(1),
            2,
            EMITENT_ROLE_MASK,
            89u64
        ));
        assert_eq!(Evercity::account_is_custodian(&2), true);
        assert_eq!(Evercity::account_is_emitent(&2), true);
    });
}

#[test]
fn it_denies_add_and_set_roles_for_non_master() {
    new_test_ext().execute_with(|| {
        // trying to add account form non-master account
        assert_noop!(
            Evercity::account_add_with_role_and_data(
                Origin::signed(2),
                101,
                MASTER_ROLE_MASK,
                88u64
            ),
            Error::<TestRuntime>::AccountNotAuthorized
        );

        assert_noop!(
            Evercity::account_set_with_role_and_data(
                Origin::signed(3),
                3,
                EMITENT_ROLE_MASK,
                88u64
            ),
            Error::<TestRuntime>::AccountNotAuthorized
        );
    });
}

// [TODO] check add and set with account without MASTER role

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
