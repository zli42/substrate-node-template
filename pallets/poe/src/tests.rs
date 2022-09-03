use super::*;
use crate::{mock::*, Error};
use frame_support::{assert_noop, assert_ok};


#[test]
fn creat_claim_works() {
	new_test_ext().execute_with(|| {
		let account_id = 1;
		let claim = sp_core::H256([0; 32]);

		assert_ok!(PoeModule::create_claim(Origin::signed(account_id), claim));

		assert_eq!(Claims::<Test>::get(&claim), Some((account_id, <frame_system::Pallet<Test>>::block_number())));
	});
}

#[test]
fn revoke_claim_works() {
	new_test_ext().execute_with(|| {
		let account_id = 1;
		let claim = sp_core::H256([0; 32]);

		let _ = PoeModule::create_claim(Origin::signed(account_id), claim);
		
		assert_ok!(PoeModule::revoke_claim(Origin::signed(account_id), claim));

		assert!(Claims::<Test>::try_get(&claim).is_err());
	});
}

#[test]
fn transfer_claim_works() {
	new_test_ext().execute_with(|| {
		let account_id_1 = 1;
		let account_id_2 = 2;
		let claim = sp_core::H256([0; 32]);

		let _ = PoeModule::create_claim(Origin::signed(account_id_1), claim);

		assert_ok!(PoeModule::transfer_claim(Origin::signed(account_id_1), claim, account_id_2));

		assert_eq!(Claims::<Test>::get(&claim), Some((account_id_2, <frame_system::Pallet<Test>>::block_number())));
	});
}

#[test]
fn correct_error_for_already_claimed() {
	new_test_ext().execute_with(|| {
		let account_id = 1;
		let claim = sp_core::H256([0; 32]);

		let _ = PoeModule::create_claim(Origin::signed(account_id), claim);

		assert_noop!(PoeModule::create_claim(Origin::signed(account_id), claim), Error::<Test>::AlreadyClaimed);
	});
}

#[test]
fn correct_error_for_no_such_claimed() {
	new_test_ext().execute_with(|| {
		let account_id_1 = 1;
		let account_id_2 = 2;
		let claim = sp_core::H256([0; 32]);

		assert_noop!(PoeModule::revoke_claim(Origin::signed(account_id_1), claim), Error::<Test>::NoSuchClaim);
		assert_noop!(PoeModule::transfer_claim(Origin::signed(account_id_1), claim, account_id_2), Error::<Test>::NoSuchClaim);
	});
}

#[test]
fn correct_error_for_not_claim_owner() {
	new_test_ext().execute_with(|| {
		let account_id_1 = 1;
		let account_id_2 = 2;
		let claim = sp_core::H256([0; 32]);

		let _ = PoeModule::create_claim(Origin::signed(account_id_1), claim);

		assert_noop!(PoeModule::revoke_claim(Origin::signed(account_id_2), claim), Error::<Test>::NotClaimOwner);
		assert_noop!(PoeModule::transfer_claim(Origin::signed(account_id_2), claim, account_id_1), Error::<Test>::NotClaimOwner);
	});
}
