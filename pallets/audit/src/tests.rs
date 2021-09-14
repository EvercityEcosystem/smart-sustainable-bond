use crate::{Error, mock::*};
use crate::H256;
use frame_support::{assert_err, assert_noop, assert_ok, dispatch::{
		DispatchResult, 
		DispatchError, 
		Vec,
}};


#[test]
fn it_works_for_create_new_file() {
	new_test_ext().execute_with(|| {
		let tag = vec![40, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
		let filehash = H256::from([0x66; 32]);
		let owner = 3;

		let create_file_result = Audit::create_new_file(Origin::signed(owner), tag.clone(), filehash);
		let file = Audit::get_file_by_id(1);

		assert_ok!(create_file_result, ());
		assert_eq!(owner, file.owner);
		assert_eq!(1, file.id);
		assert_eq!(filehash, file.versions[0].filehash);
		assert_eq!(1, file.versions.len());
		assert_eq!(0, file.auditors.len());
	});
}

#[test]
fn it_works_for_create_new_file_increment_version() {
	new_test_ext().execute_with(|| {
		let tag = vec![40, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
		let filehash = H256::from([0x66; 32]);
		let owner1 = 1;
		let owner2 = 1;

		let _ = Audit::create_new_file(Origin::signed(owner1), tag.clone(), filehash);
		let _ = Audit::create_new_file(Origin::signed(owner2), tag.clone(), filehash);
		let file1 = Audit::get_file_by_id(1);
		let file2 = Audit::get_file_by_id(2);

		assert_eq!(owner1, file1.owner);
		assert_eq!(1, file1.id);
		assert_eq!(owner2, file2.owner);
		assert_eq!(2, file2.id);
	});
}

#[test]
fn it_fails_for_create_new_file_incorrect_file_input() {
	new_test_ext().execute_with(|| {
		let tag = Vec::new();
		let filehash = H256::from([0x66; 32]);
		let owner = 3;

		let create_file_result = Audit::create_new_file(Origin::signed(owner), tag.clone(), filehash);		
		let file = Audit::get_file_by_id(1);

		assert_ne!(create_file_result, DispatchResult::Ok(()));
		assert_eq!(0, file.owner);
	});
}

#[test]
fn it_works_assign_auditor() {
	new_test_ext().execute_with(|| {
		let tag = vec![40, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
		let filehash = H256::from([0x66; 32]);
		let account_id = 1;

		let create_file_result = Audit::create_new_file(Origin::signed(1), tag, filehash);
		let assign_auditor_result = Audit::assign_auditor(Origin::signed(1), 1, account_id);
		let file = Audit::get_file_by_id(1);

		assert_ok!(create_file_result, ());
		assert_ok!(assign_auditor_result, ());
		assert_eq!(1, file.auditors.len());
		assert_eq!(account_id, file.auditors[0]);
	});
}

#[test]
fn it_works_delete_auditor() {
	new_test_ext().execute_with(|| {
		let tag = vec![40, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
		let filehash = H256::from([0x66; 32]);
		let account_id = 2;

		let create_file_result = Audit::create_new_file(Origin::signed(1), tag.clone(), filehash);
		let assign_auditor_result = Audit::assign_auditor(Origin::signed(1), 1, account_id);

		// Check file state before delete
		let file_with_auditor = Audit::get_file_by_id(1);
		let delete_auditor_result = Audit::delete_auditor(Origin::signed(1), 1, account_id);

		// Check file state after delete
		let file_without_auditor = Audit::get_file_by_id(1);

		assert_ok!(create_file_result, ());
		assert_ok!(assign_auditor_result, ());
		assert_ok!(delete_auditor_result, ());
		assert_eq!(1, file_with_auditor.auditors.len());
		assert_eq!(account_id, file_with_auditor.auditors[0]);
		assert_eq!(0, file_without_auditor.auditors.len());
	});
}

#[test]
fn it_fails_delete_auditor_no_auditors() {
	new_test_ext().execute_with(|| {
		let tag = vec![40, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
		let filehash = H256::from([0x66; 32]);

		let account_id = 1;
		let _ = Audit::create_new_file(Origin::signed(1), tag.clone(), filehash);

		// First - try to delete unexisting auditor 
		let delete_auditor_result_no_auditors = Audit::delete_auditor(Origin::signed(1), 1, account_id);

		// Second - try to delete unexisting auditor after delete:
		let _ = Audit::assign_auditor(Origin::signed(1), 1, account_id);
		let _ = Audit::delete_auditor(Origin::signed(1), 1, account_id);
		let delete_auditor_result_after_delete = Audit::delete_auditor(Origin::signed(1), 1, account_id);

		assert_ne!(delete_auditor_result_no_auditors, DispatchResult::Ok(()));
		assert_ne!(delete_auditor_result_after_delete, DispatchResult::Ok(()));
	});
}


#[test]
fn it_works_sign_latest_version() {
	new_test_ext().execute_with(|| {
		let tag = vec![40, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
		let filehash = H256::from([0x66; 32]);
		let account_id = 1;

		let _ = Audit::create_new_file(Origin::signed(1), tag, filehash);
		let assign_auditor_result = Audit::assign_auditor(Origin::signed(1), 1, account_id);
		let sign_latest_version_result = Audit::sign_latest_version(Origin::signed(1), 1);
		let _ = Audit::sign_latest_version(Origin::signed(1), 1);
		let file = Audit::get_file_by_id(1);

		assert_ok!(assign_auditor_result, ());
		assert_ok!(sign_latest_version_result, ());
		assert_eq!(1, file.versions.last().unwrap().signatures.len());
	});
}

#[test]
fn it_fail_sign_latest_version_not_an_auditor() {
	new_test_ext().execute_with(|| {
		let tag = vec![40, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
		let filehash = H256::from([0x66; 32]);

		let _ = Audit::create_new_file(Origin::signed(1), tag, filehash);
		let sign_latest_version_result = Audit::sign_latest_version(Origin::signed(1), 1);
		let file = Audit::get_file_by_id(1);

		assert_ne!(sign_latest_version_result, DispatchResult::Ok(()));
		// Assert that no sign has been added
		assert_eq!(0, file.versions.last().unwrap().signatures.len());
	});
}