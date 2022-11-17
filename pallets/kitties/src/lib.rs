#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::pallet_prelude::*;
	use frame_support::traits::{Currency, Randomness, ReservableCurrency};
	use frame_system::pallet_prelude::*;

	type KittyDNA = [u8; 16];
	type AccountOf<T> = <T as frame_system::Config>::AccountId;
	type BalanceOf<T> =
		<<T as Config>::KittyCurrency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

	#[derive(Clone, Encode, Decode, PartialEq, TypeInfo, MaxEncodedLen, RuntimeDebug)]
	#[scale_info(skip_type_params(T))]
	pub struct Kitty<T: Config> {
		pub dna: KittyDNA,
		pub price: BalanceOf<T>,
		pub owner: AccountOf<T>,
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		type KittyCurrency: ReservableCurrency<Self::AccountId>;
		type KittyRandomness: Randomness<Self::Hash, Self::BlockNumber>;

		#[pallet::constant]
		type KittyPrice: Get<BalanceOf<Self>>;

		#[pallet::constant]
		type MaxKittiesOwned: Get<u32>;

		#[pallet::constant]
		type MaxKittiesCount: Get<u32>;
	}

	#[pallet::storage]
	#[pallet::getter(fn kitties_owned)]
	pub type KittiesOwned<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		BoundedVec<KittyDNA, T::MaxKittiesOwned>,
		ValueQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn kitties)]
	pub type Kitties<T: Config> = StorageMap<_, Blake2_128Concat, KittyDNA, Kitty<T>>;

	#[pallet::storage]
	#[pallet::getter(fn kitties_count)]
	pub type KittiesCount<T: Config> = StorageValue<_, u32, ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A new kitty was successfully created. [kitty, owner]
		KittyCreated(KittyDNA, T::AccountId),
		/// A new kitty was successfully bred. [kitty, owner]
		KittyBred(KittyDNA, T::AccountId),
		/// A kitty was successfully transferred. [from, to, kitty]
		KittyTransferred(T::AccountId, T::AccountId, KittyDNA),
		/// The price of a kitty was successfully set. [kitty, price]
		PriceSet(KittyDNA, Option<BalanceOf<T>>),
		/// A kitty was successfully sold. [seller, buyer, kitty, price]
		Sold(T::AccountId, T::AccountId, KittyDNA, BalanceOf<T>),
	}

	#[pallet::error]
	pub enum Error<T> {
		/// An account don't have enough balance.
		NotEnoughBalance,
		/// An account may only own `MaxKittiesOwned` kitties.
		ExceedMaxKittiesOwned,
		/// This kitty already exists!
		SameKitties,
		/// An overflow has occurred!
		KittiesCountOverFlow,
		/// This kitty does not exist!
		KittyNotExists,
		/// You are not the owner of this kitty.
		NotOwner,
		/// Trying to transfer or buy a kitty from oneself.
		TransferToSelf,
		/// Ensures that the buying price is greater than the asking price.
		BidPriceTooLow,
		/// This kitty is not for sale.
		NotForSale,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(10_000)]
		pub fn create_kitty(origin: OriginFor<T>) -> DispatchResult {
			let owner = ensure_signed(origin)?;

			let price = T::KittyPrice::get();
			ensure!(T::KittyCurrency::can_reserve(&owner, price), Error::<T>::NotEnoughBalance);

			let count = KittiesCount::<T>::get()
				.checked_add(1)
				.ok_or(Error::<T>::KittiesCountOverFlow)?;
			ensure!(count <= T::MaxKittiesCount::get(), Error::<T>::KittiesCountOverFlow);

			let dna = Self::gen_random_value(&owner);
			ensure!(!Kitties::<T>::contains_key(dna), Error::<T>::SameKitties);

			let kitty = Kitty::<T> { dna, price, owner: owner.clone() };

			KittiesOwned::<T>::try_append(owner.clone(), dna)
				.map_err(|_| Error::<T>::ExceedMaxKittiesOwned)?;
			T::KittyCurrency::reserve(&owner, price).map_err(|_| Error::<T>::NotEnoughBalance)?;
			Kitties::<T>::insert(dna, kitty);
			KittiesCount::<T>::put(count);

			Self::deposit_event(Event::KittyCreated(dna, owner));

			Ok(())
		}

		#[pallet::weight(10_000)]
		pub fn breed_kitty(
			origin: OriginFor<T>,
			parent_1: KittyDNA,
			parent_2: KittyDNA,
		) -> DispatchResult {
			let owner = ensure_signed(origin)?;

			let count = KittiesCount::<T>::get()
				.checked_add(1)
				.ok_or(Error::<T>::KittiesCountOverFlow)?;
			ensure!(count <= T::MaxKittiesCount::get(), Error::<T>::KittiesCountOverFlow);

			let kitty_1 = Kitties::<T>::get(parent_1).ok_or(Error::<T>::KittyNotExists)?;
			let kitty_2 = Kitties::<T>::get(parent_2).ok_or(Error::<T>::KittyNotExists)?;

			ensure!(kitty_1.owner == owner, Error::<T>::NotOwner);
			ensure!(kitty_2.owner == owner, Error::<T>::NotOwner);

			ensure!(kitty_1.dna != kitty_2.dna, Error::<T>::SameKitties);

			let price = T::KittyPrice::get();
			ensure!(T::KittyCurrency::can_reserve(&owner, price), Error::<T>::NotEnoughBalance);

			let selector = Self::gen_random_value(&owner);
			let mut dna = [0u8; 16];
			for i in 0..dna.len() {
				dna[i] = (kitty_1.dna[i] & selector[i]) | (kitty_2.dna[i] & !selector[i]);
			}

			ensure!(!Kitties::<T>::contains_key(dna), Error::<T>::SameKitties);

			let kitty = Kitty::<T> { dna, price, owner: owner.clone() };

			KittiesOwned::<T>::try_append(owner.clone(), dna)
				.map_err(|_| Error::<T>::ExceedMaxKittiesOwned)?;
			T::KittyCurrency::reserve(&owner, price).map_err(|_| Error::<T>::NotEnoughBalance)?;
			Kitties::<T>::insert(dna, kitty);
			KittiesCount::<T>::put(count);

			Self::deposit_event(Event::KittyBred(dna, owner));

			Ok(())
		}

		#[pallet::weight(10_000)]
		pub fn transfer_kitty(
			origin: OriginFor<T>,
			to: T::AccountId,
			dna: KittyDNA,
		) -> DispatchResult {
			let from = ensure_signed(origin)?;

			ensure!(from != to, Error::<T>::TransferToSelf);

			let mut kitty = Kitties::<T>::get(dna).ok_or(Error::<T>::KittyNotExists)?;

			ensure!(kitty.owner == from, Error::<T>::NotOwner);

			ensure!(T::KittyCurrency::can_reserve(&to, kitty.price), Error::<T>::NotEnoughBalance);

			let mut to_owned = KittiesOwned::<T>::get(&to);
			to_owned.try_push(dna).map_err(|_| Error::<T>::ExceedMaxKittiesOwned)?;

			T::KittyCurrency::reserve(&to, kitty.price)
				.map_err(|_| Error::<T>::NotEnoughBalance)?;

			let mut from_owned = KittiesOwned::<T>::get(&from);
			if let Some(ind) = from_owned.iter().position(|&id| id == dna) {
				from_owned.swap_remove(ind);
			} else {
				return Err(Error::<T>::KittyNotExists.into());
			}

			T::KittyCurrency::unreserve(&from, kitty.price);
			kitty.owner = to.clone();
			Kitties::<T>::insert(dna, kitty);
			KittiesOwned::<T>::insert(&to, to_owned);
			KittiesOwned::<T>::insert(&from, from_owned);

			Self::deposit_event(Event::KittyTransferred(from, to, dna));

			Ok(())
		}

		#[pallet::weight(0)]
		pub fn set_price(
			origin: OriginFor<T>,
			kitty_id: KittyDNA,
			new_price: BalanceOf<T>,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			let mut kitty = Kitties::<T>::get(&kitty_id).ok_or(Error::<T>::KittyNotExists)?;
			ensure!(kitty.owner == sender, Error::<T>::NotOwner);

			kitty.price = new_price;
			Kitties::<T>::insert(&kitty_id, kitty);

			// Self::deposit_event(Event::PriceSet(kitty_id, new_price));

			Ok(())
		}

		#[pallet::weight(0)]
		pub fn buy_kitty(
			origin: OriginFor<T>,
			kitty_id: KittyDNA,
			bid_price: BalanceOf<T>,
		) -> DispatchResult {
			let buyer = ensure_signed(origin)?;

			Self::do_buy_kitty(kitty_id, buyer, bid_price)?;

			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		fn gen_random_value(sender: &T::AccountId) -> KittyDNA {
			let payload = (
				T::KittyRandomness::random(&b"dna"[..]).0,
				&sender,
				<frame_system::Pallet<T>>::extrinsic_index(),
				<frame_system::Pallet<T>>::block_number(),
			);
			let encoded_payload = payload.encode();
			frame_support::Hashable::blake2_128(&encoded_payload)
		}

		pub fn do_buy_kitty(
			kitty_id: KittyDNA,
			buyer: T::AccountId,
			bid_price: BalanceOf<T>,
		) -> DispatchResult {
			let mut kitty = Kitties::<T>::get(&kitty_id).ok_or(Error::<T>::KittyNotExists)?;
			let seller = kitty.owner;

			ensure!(seller != buyer, Error::<T>::TransferToSelf);
			let mut seller_owned = KittiesOwned::<T>::get(&seller);

			if let Some(ind) = seller_owned.iter().position(|&id| id == kitty_id) {
				seller_owned.swap_remove(ind);
			} else {
				return Err(Error::<T>::KittyNotExists.into());
			}

			let mut buyer_owned = KittiesOwned::<T>::get(&buyer);
			buyer_owned.try_push(kitty_id).map_err(|_| Error::<T>::ExceedMaxKittiesOwned)?;

			// if let Some(price) = kitty.price {
			// 	ensure!(bid_price >= price, Error::<T>::BidPriceTooLow);
			// 	T::Currency::transfer(
			// 		&buyer,
			// 		&seller,
			// 		price,
			// 		frame_support::traits::ExistenceRequirement::KeepAlive,
			// 	)?;
			// 	Self::deposit_event(Event::Sold(seller.clone(), buyer.clone(), kitty_id, price));
			// } else {
			// 	return Err(Error::<T>::NotForSale.into());
			// }

			kitty.owner = buyer.clone();
			// kitty.price = None;

			Kitties::<T>::insert(&kitty_id, kitty);
			KittiesOwned::<T>::insert(&buyer, buyer_owned);
			KittiesOwned::<T>::insert(&seller, seller_owned);

			Self::deposit_event(Event::KittyTransferred(seller, buyer, kitty_id));

			Ok(())
		}
	}
}
