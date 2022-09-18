use super::*;
use crate::{mock::*, Error};
use frame_support::{assert_noop, assert_ok, traits::Get};

#[test]
fn create_kitty_should_work() {
	new_test_ext().execute_with(|| {
		let owner = 1;
		assert_ok!(KittiesModule::create_kitty(Origin::signed(owner)));

		let kitties_owned = KittiesOwned::<Test>::get(owner);
		assert_eq!(kitties_owned.len(), 1);
		let dna = kitties_owned[0];

		let price: u128 = <Test as Config>::KittyPrice::get();
		assert_eq!(<Test as Config>::KittyCurrency::reserved_balance(owner), price);

		let kitty = Kitties::<Test>::get(dna).unwrap();
		assert_eq!(dna, kitty.dna);
		assert_eq!(price, kitty.price);
		assert_eq!(owner, kitty.owner);

		assert_eq!(KittiesCount::<Test>::get(), 1);

		// Check that multiple create_kitty calls work in the same block.
		// Increment extrinsic index to add entropy for DNA
		frame_system::Pallet::<Test>::set_extrinsic_index(1);
		assert_ok!(KittiesModule::create_kitty(Origin::signed(owner)));
	});
}

#[test]
fn breed_kitty_should_work() {
	new_test_ext().execute_with(|| {
		let owner = 1;
		assert_ok!(KittiesModule::create_kitty(Origin::signed(owner)));

		frame_system::Pallet::<Test>::set_extrinsic_index(1);
		assert_ok!(KittiesModule::create_kitty(Origin::signed(owner)));

		let kitties_owned = KittiesOwned::<Test>::get(owner);
		let dna_1 = kitties_owned[0];
		let dna_2 = kitties_owned[1];

		assert_ok!(KittiesModule::breed_kitty(Origin::signed(owner), dna_1, dna_2));

		let new_kitties_owned = KittiesOwned::<Test>::get(owner);
		let cnt = new_kitties_owned.len();
		assert_eq!(cnt, 3);

		let price: u128 = <Test as Config>::KittyPrice::get();
		assert_eq!(<Test as Config>::KittyCurrency::reserved_balance(owner), price * 3);

		let dna = kitties_owned[1];
		let kitty = Kitties::<Test>::get(dna).unwrap();
		assert_eq!(dna, kitty.dna);
		assert_eq!(price, kitty.price);
		assert_eq!(owner, kitty.owner);

		for (i, &v) in dna.iter().enumerate() {
			assert!(v == dna_1[i] || v == dna_2[i]);
		}

		assert_eq!(KittiesCount::<Test>::get(), 3);
	});
}

#[test]
fn transfer_kitty_should_work() {
	new_test_ext().execute_with(|| {
		let owner_1 = 1;
		let owner_2 = 2;
		assert_ok!(KittiesModule::create_kitty(Origin::signed(owner_1)));
		let kitties_owned = KittiesOwned::<Test>::get(owner_1);
		let dna = kitties_owned[0];

		assert_ok!(KittiesModule::transfer_kitty(Origin::signed(owner_1), owner_2, dna));

		assert_eq!(KittiesOwned::<Test>::get(owner_1).len(), 0);
		assert_eq!(KittiesOwned::<Test>::get(owner_2).len(), 1);

		let price: u128 = <Test as Config>::KittyPrice::get();
		assert_eq!(<Test as Config>::KittyCurrency::reserved_balance(owner_1), 0);
		assert_eq!(<Test as Config>::KittyCurrency::reserved_balance(owner_2), price);

		let kitty = Kitties::<Test>::get(dna).unwrap();
		assert_eq!(dna, kitty.dna);
		assert_eq!(price, kitty.price);
		assert_eq!(owner_2, kitty.owner);
	});
}

#[test]
fn create_kitty_should_fail() {
	new_test_ext().execute_with(|| {
		let mut extrinsic_index = 1;

		let owner_4 = 4;
		assert_noop!(
			KittiesModule::create_kitty(Origin::signed(owner_4)),
			Error::<Test>::NotEnoughBalance
		);

		let owner_1 = 1;
		assert_ok!(KittiesModule::create_kitty(Origin::signed(owner_1)));
		assert_noop!(
			KittiesModule::create_kitty(Origin::signed(owner_1)),
			Error::<Test>::DuplicateKitty
		);

		let max_kitties_owned: u32 = <Test as Config>::MaxKittiesOwned::get();
		for _ in 0..max_kitties_owned - 1 {
			frame_system::Pallet::<Test>::set_extrinsic_index(extrinsic_index);
			extrinsic_index += 1;
			assert_ok!(KittiesModule::create_kitty(Origin::signed(owner_1)));
		}

		let owner_2 = 2;
		for _ in 0..<Test as Config>::MaxKittiesOwned::get() {
			frame_system::Pallet::<Test>::set_extrinsic_index(extrinsic_index);
			extrinsic_index += 1;
			assert_ok!(KittiesModule::create_kitty(Origin::signed(owner_2)));
		}

		frame_system::Pallet::<Test>::set_extrinsic_index(extrinsic_index);
		extrinsic_index += 1;
		assert_noop!(
			KittiesModule::create_kitty(Origin::signed(owner_2)),
			Error::<Test>::ExceedMaxKittiesOwned
		);

		let owner_3 = 3;
		frame_system::Pallet::<Test>::set_extrinsic_index(extrinsic_index);
		extrinsic_index += 1;
		assert_ok!(KittiesModule::create_kitty(Origin::signed(owner_3)));
		frame_system::Pallet::<Test>::set_extrinsic_index(extrinsic_index);
		assert_noop!(
			KittiesModule::create_kitty(Origin::signed(owner_3)),
			Error::<Test>::KittiesCountOverFlow
		);
	});
}

#[test]
fn breed_kitty_should_fail() {
	new_test_ext().execute_with(|| {
		let mut extrinsic_index = 1;

		let owner_1 = 1;
		for _ in 0..2 {
			frame_system::Pallet::<Test>::set_extrinsic_index(extrinsic_index);
			extrinsic_index += 1;
			assert_ok!(KittiesModule::create_kitty(Origin::signed(owner_1)));
		}
		let kitties_owned = KittiesOwned::<Test>::get(owner_1);
		let dna_1 = kitties_owned[0];
		let dna_2 = kitties_owned[1];

		let dna_3 = [0u8; 16];
		assert_noop!(
			KittiesModule::breed_kitty(Origin::signed(owner_1), dna_1, dna_3),
			Error::<Test>::KittyNotExists
		);

		let owner_2 = 2;
		assert_ok!(KittiesModule::create_kitty(Origin::signed(owner_2)));
		let dna_4 = KittiesOwned::<Test>::get(owner_2)[0];
		assert_noop!(
			KittiesModule::breed_kitty(Origin::signed(owner_1), dna_1, dna_4),
			Error::<Test>::NotOwner
		);

		assert_noop!(
			KittiesModule::breed_kitty(Origin::signed(owner_1), dna_1, dna_1),
			Error::<Test>::SameKitties
		);

		let owner_3 = 5;
		for _ in 0..2 {
			frame_system::Pallet::<Test>::set_extrinsic_index(extrinsic_index);
			extrinsic_index += 1;
			assert_ok!(KittiesModule::create_kitty(Origin::signed(owner_3)));
		}
		let kitties_owned_3 = KittiesOwned::<Test>::get(owner_3);
		let dna_5 = kitties_owned_3[0];
		let dna_6 = kitties_owned_3[1];
		assert_noop!(
			KittiesModule::breed_kitty(Origin::signed(owner_3), dna_5, dna_6),
			Error::<Test>::NotEnoughBalance
		);

		assert_ok!(KittiesModule::breed_kitty(Origin::signed(owner_1), dna_1, dna_2));
		assert_noop!(
			KittiesModule::breed_kitty(Origin::signed(owner_1), dna_1, dna_2),
			Error::<Test>::DuplicateKitty
		);

		for _ in 0..2 {
			frame_system::Pallet::<Test>::set_extrinsic_index(extrinsic_index);
			extrinsic_index += 1;
			assert_ok!(KittiesModule::create_kitty(Origin::signed(owner_1)));
		}
		assert_noop!(
			KittiesModule::breed_kitty(Origin::signed(owner_1), dna_1, dna_2),
			Error::<Test>::ExceedMaxKittiesOwned
		);
	});
}

#[test]
fn transfer_kitty_should_fail() {
	new_test_ext().execute_with(|| {
		let owner_1 = 1;
		let owner_2 = 2;
		let owner_4 = 4;
		assert_ok!(KittiesModule::create_kitty(Origin::signed(owner_1)));
		let kitties_owned = KittiesOwned::<Test>::get(owner_1);
		let dna = kitties_owned[0];

		assert_noop!(
			KittiesModule::transfer_kitty(Origin::signed(owner_1), owner_1, dna),
			Error::<Test>::TransferToSelf
		);

		let fake_dna = [0u8; 16];
		assert_noop!(
			KittiesModule::transfer_kitty(Origin::signed(owner_1), owner_2, fake_dna),
			Error::<Test>::KittyNotExists
		);

		let mut extrinsic_index = 1;
		for _ in 0..<Test as Config>::MaxKittiesOwned::get() {
			frame_system::Pallet::<Test>::set_extrinsic_index(extrinsic_index);
			extrinsic_index += 1;
			assert_ok!(KittiesModule::create_kitty(Origin::signed(owner_2)));
		}
		assert_noop!(
			KittiesModule::transfer_kitty(Origin::signed(owner_1), owner_2, dna),
			Error::<Test>::ExceedMaxKittiesOwned
		);

		assert_noop!(
			KittiesModule::transfer_kitty(Origin::signed(owner_1), owner_4, dna),
			Error::<Test>::NotEnoughBalance
		);
	});
}
