#![allow(unused_imports)]
use crate::mock::*;
use crate::{AccountRegistry, Error, Event, EverUSDBalance, Module, BondStructOf, BondInnerStructOf,
            AUDITOR_ROLE_MASK, CUSTODIAN_ROLE_MASK, EMITENT_ROLE_MASK, INVESTOR_ROLE_MASK, MASTER_ROLE_MASK,
            BondPayPeriod, DAY_DURATION};
use frame_support::{assert_noop, assert_ok, dispatch::DispatchResult};
use frame_system::Trait;

type Evercity = Module<TestRuntime>;
type Timestamp = pallet_timestamp::Module<TestRuntime>;
type BondInnerStruct = BondInnerStructOf<TestRuntime>;
type BondStruct = BondStructOf<TestRuntime>;
type RuntimeError = Error::<TestRuntime>;
type AccountId = <TestRuntime as frame_system::Trait>::AccountId;

//////////////////////////////////////////////////////////////////////////////////////////////////////////
// Test uses pack of accounts, pre-set in new_test_ext in mock.rs:
// (1, EvercityAccountStruct { roles: MASTER_ROLE_MASK,     identity: 10u64}), // MASTER    (accountId: 1)
// (2, EvercityAccountStruct { roles: CUSTODIAN_ROLE_MASK,  identity: 20u64}), // CUSTODIAN (accountID: 2)
// (3, EvercityAccountStruct { roles: EMITENT_ROLE_MASK,    identity: 30u64}), // EMITENT   (accountID: 3)
// (4, EvercityAccountStruct { roles: INVESTOR_ROLE_MASK,   identity: 40u64}), // INVESTOR  (accountId: 4)
// (5, EvercityAccountStruct { roles: AUDITOR_ROLE_MASK,    identity: 50u64}), // AUDITOR   (accountId: 5)
// (101+ : some external accounts
//////////////////////////////////////////////////////////////////////////////////////////////////////////

const CUSTODIAN_ID: AccountId = 2;

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
        Timestamp::set_timestamp(12345);

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
        assert_eq!(Evercity::account_is_emitent(&2), false);
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
        <pallet_timestamp::Module<TestRuntime>>::set_timestamp(12345);
        assert_noop!(
            Evercity::account_add_with_role_and_data(
                Origin::signed(2),
                101,
                MASTER_ROLE_MASK,
                88u64
            ),
            RuntimeError::AccountNotAuthorized
        );

        assert_noop!(
            Evercity::account_set_with_role_and_data(
                Origin::signed(3),
                3,
                EMITENT_ROLE_MASK,
                88u64
            ),
            RuntimeError::AccountNotAuthorized
        );
    });
}

// mint tokens

#[test]
fn it_token_mint_create_with_confirm() {
    const ACCOUNT: AccountId = 4; // INVESTOR
    new_test_ext().execute_with(|| {
        assert_ok!(Evercity::token_mint_request_create_everusd(
            Origin::signed(ACCOUNT), // INVESTOR
            100000
        ));

        assert_ok!(Evercity::token_mint_request_confirm_everusd(
            Origin::signed(CUSTODIAN_ID),
            ACCOUNT,
            100000
        ));
    });
}

#[test]
fn it_token_mint_create_with_revoke() {
    const ACCOUNT: AccountId = 4; // INVESTOR
    new_test_ext().execute_with(|| {
        assert_ok!(Evercity::token_mint_request_create_everusd(
            Origin::signed(ACCOUNT), // INVESTOR
            100000
        ));

        assert_ok!(Evercity::token_mint_request_revoke_everusd(Origin::signed(
            ACCOUNT
        ),));

        assert_noop!(
            Evercity::token_mint_request_confirm_everusd(
                Origin::signed(CUSTODIAN_ID), ACCOUNT, 100000
            ),
            RuntimeError::MintRequestDoesntExist
        );
    });
}

#[test]
fn it_token_mint_create_with_decline() {
    const ACCOUNT: AccountId = 4; // INVESTOR
    new_test_ext().execute_with(|| {
        assert_ok!(Evercity::token_mint_request_create_everusd(
            Origin::signed(ACCOUNT),
            100000
        ));

        assert_ok!(Evercity::token_mint_request_decline_everusd(
            Origin::signed(CUSTODIAN_ID),
            ACCOUNT
        ));

        assert_noop!(
            Evercity::token_mint_request_revoke_everusd(Origin::signed(ACCOUNT)),
            RuntimeError::MintRequestDoesntExist
        );
    });
}

#[test]
fn it_token_mint_create_denied() {
    const ACCOUNT: AccountId = 5; // AUDITOR
    new_test_ext().execute_with(|| {
        assert_noop!(
            Evercity::token_mint_request_create_everusd(Origin::signed(ACCOUNT), 100000),
            RuntimeError::AccountNotAuthorized
        );
    });
}

#[test]
fn it_token_mint_create_hasty() {
    const ACCOUNT: AccountId = 4; // INVESTOR
    new_test_ext().execute_with(|| {
        assert_ok!(Evercity::token_mint_request_create_everusd(
            Origin::signed(ACCOUNT),
            100000
        ));

        assert_noop!(
            Evercity::token_mint_request_create_everusd(Origin::signed(ACCOUNT), 10),
            RuntimeError::MintRequestAlreadyExist
        );
    });
}

#[test]
fn it_token_mint_create_toolarge() {
    const ACCOUNT: AccountId = 4;
    new_test_ext().execute_with(|| {
        assert_noop!(
            Evercity::token_mint_request_create_everusd(
                Origin::signed(ACCOUNT), // INVESTOR
                crate::EVERUSD_MAX_MINT_AMOUNT + 1
            ),
            RuntimeError::MintRequestParamIncorrect
        );
    });
}

// burn tokens

fn add_token(id: AccountId, amount: EverUSDBalance) -> DispatchResult {
    Evercity::token_mint_request_create_everusd(Origin::signed(id), amount)?;

    Evercity::token_mint_request_confirm_everusd(Origin::signed(CUSTODIAN_ID), id, amount)
}

#[test]
fn it_token_burn_create_with_confirm() {
    const ACCOUNT: AccountId = 4; // INVESTOR

    new_test_ext().execute_with(|| {
        assert_ok!(add_token(ACCOUNT, 10000));

        assert_ok!(Evercity::token_burn_request_create_everusd(
            Origin::signed(ACCOUNT),
            10000
        ));

        assert_ok!(Evercity::token_burn_request_confirm_everusd(
            Origin::signed(CUSTODIAN_ID),
            ACCOUNT,
            10000
        ));
        // duplicate confirmations are not allowed
        assert_noop!(
            Evercity::token_burn_request_confirm_everusd(
                Origin::signed(CUSTODIAN_ID), ACCOUNT, 10000
            ),
            RuntimeError::BurnRequestDoesntExist
        );
    });
}

#[test]
fn it_token_burn_create_overrun() {
    const ACCOUNT: AccountId = 3; // EMITENT
    const BALANCE: EverUSDBalance = 10000;

    new_test_ext().execute_with(|| {
        assert_ok!(add_token(ACCOUNT, BALANCE));

        assert_noop!(
            Evercity::token_burn_request_create_everusd(
                Origin::signed(ACCOUNT),
                BALANCE + 1
            ),
            RuntimeError::MintRequestParamIncorrect
        );
    });
}

#[test]
fn it_token_burn_create_with_revoke() {
    const ACCOUNT: AccountId = 3; // EMITENT

    new_test_ext().execute_with(|| {
        assert_ok!(add_token(ACCOUNT, 10000));

        assert_ok!(Evercity::token_burn_request_create_everusd(
            Origin::signed(ACCOUNT),
            10000
        ));

        assert_ok!(Evercity::token_burn_request_revoke_everusd(Origin::signed(
            ACCOUNT
        ),));

        assert_noop!(
            Evercity::token_burn_request_confirm_everusd(
                Origin::signed(CUSTODIAN_ID), ACCOUNT, 10000
            ),
            RuntimeError::BurnRequestDoesntExist
        );
    });
}

#[test]
fn it_bond_test() {
    const ACCOUNT: AccountId = 3; // EMITENT
    new_test_ext().execute_with(|| {
        Timestamp::set_timestamp(12345);

        assert_ok!( Evercity::bond_dummy(Origin::signed(ACCOUNT)));
    });
}

fn get_test_bond() -> BondStruct {
    BondStruct{
        inner: BondInnerStruct{
            data_hash_main: Default::default(),
            data_hash_legal: Default::default(),
            data_hash_finance: Default::default(),
            data_hash_tech: Default::default(),

            bond_category: 0,
            impact_data_type: 0,
            impact_baseline: 20000_u64,
            impact_max_deviation_cap: 30000_u64,
            impact_max_deviation_floor: 14000_u64,
            missed_report_penalty: 400, //0.4%

            bond_base_interest_rate: 2000, // 2.0%
            bond_interest_margin_cap: 4000, // 4.0%
            bond_interest_margin_floor: 1000, //1%
            start_period_interest_rate: 1900,
            start_period: 120 * DAY_DURATION,
            reset_period: 30 * DAY_DURATION, // every month
            interest_pay_period: 7, // up to 7 days after the new period started
            mincap_deadline: (20 * DAY_DURATION * 1000) as u64,
            report_period: 10 * DAY_DURATION, // 10 days before next period
            bond_duration: 12, //
            bond_finishing_period: 30,

            mincap_amount: 1000,
            maxcap_amount: 1800,
            base_price: 4000,
        },

        emitent: 0,
        manager: 0,
        auditor: 0,
        impact_reporter: 0,

        issued_amount: 0,
        booking_start_date: Default::default(),
        active_start_date: Default::default(),
        creation_date: Default::default(),
        state: Default::default(),
        last_updated: Default::default(),
        bond_debit: 0,
        bond_credit: 0,

        period: 0,
    }
}

#[test]
fn bond_validation() {
    new_test_ext().execute_with(|| {
        let bond = get_test_bond();
        assert_eq!(bond.inner.is_valid(), true);
    });
}
fn bond_interest_min_max() {
    new_test_ext().execute_with(|| {

        let bond = get_test_bond();
        // full amplitude
        assert_eq!(bond.interest_rate(bond.inner.impact_baseline), bond.inner.bond_base_interest_rate );
        assert_eq!(bond.interest_rate(bond.inner.impact_max_deviation_cap), bond.inner.bond_interest_margin_floor);
        assert_eq!(bond.interest_rate(bond.inner.impact_max_deviation_cap+1), bond.inner.bond_interest_margin_floor);
        assert_eq!(bond.interest_rate(bond.inner.impact_max_deviation_floor), bond.inner.bond_interest_margin_cap);
        assert_eq!(bond.interest_rate(bond.inner.impact_max_deviation_floor-1), bond.inner.bond_interest_margin_cap);

        // partial amplitude
        assert_eq!(bond.interest_rate(25000_u64 ), 1500);
        assert_eq!(bond.interest_rate(29000_u64 ), 1100);

        assert_eq!(bond.interest_rate(17000_u64 ), 3000);
        assert_eq!(bond.interest_rate(15000_u64 ), 3666);
    });
}

#[test]
fn bond_start_period() {
    let bond = get_test_bond();
    assert_eq!( bond.period(bond.active_start_date), 0 );
}

// [TODO] check add and set with account without MASTER role

