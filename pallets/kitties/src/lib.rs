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
	use frame_support::transactional;
	use frame_system::pallet_prelude::*;
	use sp_io::hashing::blake2_128;

	type DNA = [u8; 16];
	type AccountOf<T> = <T as frame_system::Config>::AccountId;
	type BalanceOf<T> =
		<<T as Config>::KittyCurrency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

	#[derive(Clone, Encode, Decode, PartialEq, TypeInfo, MaxEncodedLen)]
	#[scale_info(skip_type_params(T))]
	pub struct Kitty<T: Config> {
		pub dna: DNA,
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
	pub type KittiesOwned<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		BoundedVec<DNA, T::MaxKittiesOwned>,
		ValueQuery,
	>;

	#[pallet::storage]
	pub type Kitties<T> = StorageMap<_, Blake2_128Concat, DNA, Kitty<T>>;

	#[pallet::storage]
	pub type KittiesCount<T> = StorageValue<_, u32, ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		KittyCreated { kitty: DNA, owner: T::AccountId },
		KittyBred { kitty: DNA, owner: T::AccountId },
		KittyTransferred { from: T::AccountId, to: T::AccountId, kitty: DNA },
	}

	#[pallet::error]
	pub enum Error<T> {
		NotEnoughBalance,
		DuplicateKitty,
		KittiesCountOverFlow,
		ExceedMaxKittiesOwned,
		KittyNotExists,
		NotOwner,
		SameKitties,
		TransferToSelf,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(10_000)]
		#[transactional]
		pub fn create_kitty(origin: OriginFor<T>) -> DispatchResult {
			let owner = ensure_signed(origin)?;

			let price = T::KittyPrice::get();
			ensure!(T::KittyCurrency::can_reserve(&owner, price), Error::<T>::NotEnoughBalance);

			let count = KittiesCount::<T>::get()
				.checked_add(1)
				.ok_or(Error::<T>::KittiesCountOverFlow)?;
			ensure!(count <= T::MaxKittiesCount::get(), Error::<T>::KittiesCountOverFlow);

			let dna = Self::gen_random_value(&owner);
			ensure!(!Kitties::<T>::contains_key(dna), Error::<T>::DuplicateKitty);

			let kitty = Kitty::<T> { dna, price, owner: owner.clone() };

			KittiesOwned::<T>::try_append(owner.clone(), dna)
				.map_err(|_| Error::<T>::ExceedMaxKittiesOwned)?;
			T::KittyCurrency::reserve(&owner, price).map_err(|_| Error::<T>::NotEnoughBalance)?;
			Kitties::<T>::insert(dna, kitty);
			KittiesCount::<T>::put(count);

			Self::deposit_event(Event::KittyCreated { kitty: dna, owner });

			Ok(())
		}

		#[pallet::weight(10_000)]
		#[transactional]
		pub fn breed_kitty(origin: OriginFor<T>, parent_1: DNA, parent_2: DNA) -> DispatchResult {
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

			ensure!(!Kitties::<T>::contains_key(dna), Error::<T>::DuplicateKitty);

			let kitty = Kitty::<T> { dna, price, owner: owner.clone() };

			KittiesOwned::<T>::try_append(owner.clone(), dna)
				.map_err(|_| Error::<T>::ExceedMaxKittiesOwned)?;
			T::KittyCurrency::reserve(&owner, price).map_err(|_| Error::<T>::NotEnoughBalance)?;
			Kitties::<T>::insert(dna, kitty);
			KittiesCount::<T>::put(count);

			Self::deposit_event(Event::KittyBred { kitty: dna, owner });

			Ok(())
		}

		#[pallet::weight(10_000)]
		#[transactional]
		pub fn transfer_kitty(
			origin: OriginFor<T>,
			to: T::AccountId,
			dna: [u8; 16],
		) -> DispatchResult {
			let from = ensure_signed(origin)?;

			ensure!(from != to, Error::<T>::TransferToSelf);

			let mut kitty = Kitties::<T>::get(dna).ok_or(Error::<T>::KittyNotExists)?;

			ensure!(kitty.owner == from, Error::<T>::NotOwner);

			ensure!(T::KittyCurrency::can_reserve(&to, kitty.price), Error::<T>::NotEnoughBalance);

			KittiesOwned::<T>::try_append(to.clone(), dna)
				.map_err(|_| Error::<T>::ExceedMaxKittiesOwned)?;
			T::KittyCurrency::reserve(&to, kitty.price)
				.map_err(|_| Error::<T>::NotEnoughBalance)?;
			KittiesOwned::<T>::try_mutate(&from, |owned| {
				if let Some(ind) = owned.iter().position(|&id| id == dna) {
					owned.swap_remove(ind);
					return Ok(());
				} else {
					return Err(());
				}
			})
			.map_err(|_| Error::<T>::KittyNotExists)?;
			T::KittyCurrency::unreserve(&from, kitty.price);
			kitty.owner = to.clone();
			Kitties::<T>::insert(dna, kitty);

			Self::deposit_event(Event::KittyTransferred { from, to, kitty: dna });

			Ok(())
		}
	}
	
	impl<T: Config> Pallet<T> {
		fn gen_random_value(sender: &T::AccountId) -> DNA {
			let payload = (
				T::KittyRandomness::random_seed(),
				&sender,
				<frame_system::Pallet<T>>::extrinsic_index(),
			);
			payload.using_encoded(blake2_128)
		}
	}
}
