use crate::{Error, Event, mock::*};
use frame_support::{assert_ok, assert_noop};

#[test]
fn it_returns_correct_role_for_master() {
	new_test_ext().execute_with(|| {
		// Dispatch a signed extrinsic.
		// assert_ok!(Evercity::(Origin::signed(1), 42));
		// Read pallet storage and assert an expected result.
		assert_eq!(Evercity::account_is_master(&1), false);
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
