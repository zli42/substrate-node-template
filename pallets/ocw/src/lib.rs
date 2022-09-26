#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;

	use frame_support::inherent::Vec;
	use sp_io::offchain_index;
	use sp_runtime::offchain::storage::StorageValueRef;

	#[derive(Encode, Decode, Debug)]
	struct IndexingData(Vec<u8>, u32);

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		OffchainStored(T::AccountId, u32),
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(10_000)]
		pub fn store_by_offchain_index(origin: OriginFor<T>, content: u32) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let key = Self::derived_key(frame_system::Pallet::<T>::block_number());
			let data = IndexingData(b"off-chain content: ".to_vec(), content);

			offchain_index::set(&key, &data.encode());

			Self::deposit_event(Event::OffchainStored(who, content));

			Ok(())
		}
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn offchain_worker(block_number: T::BlockNumber) {
			let key = Self::derived_key(block_number);
			let oci = StorageValueRef::persistent(&key);

			if let Ok(Some(data)) = oci.get::<IndexingData>() {
				log::info!("off-chain indexing data: {:?}", data);
			} else {
				log::info!("no off-chain indexing data retrieved.");
			}
		}
	}

	impl<T: Config> Pallet<T> {
		#[deny(clippy::clone_double_ref)]
		fn derived_key(block_number: T::BlockNumber) -> Vec<u8> {
			block_number.using_encoded(|encoded_bn| {
				b"ocw::storage::".iter().chain(encoded_bn).copied().collect::<Vec<u8>>()
			})
		}
	}
}
