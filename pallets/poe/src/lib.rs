#![cfg_attr(not(feature = "std"), no_std)]

// Re-export pallet items so that they can be accessed from the crate namespace.
pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

pub mod weights;
pub use weights::WeightInfo;

#[frame_support::pallet]
pub mod pallet {
	pub use frame_support::pallet_prelude::*;
	pub use frame_system::pallet_prelude::*;
	pub use sp_std::prelude::*;
	use super::WeightInfo;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		#[pallet::constant]
		type MaxClaimLength: Get<u32>;

		type WeightInfo: WeightInfo;
	}

	// Pallets use events to inform users when important changes are made.
	// Event documentation should end with an array that provides descriptive names for parameters.
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Event emitted when a claim has been created.
		ClaimCreated(T::AccountId, Vec<u8>),
		/// Event emitted when a claim is revoked by the owner.
		ClaimRevoked(T::AccountId, Vec<u8>),
		/// Event emitted when a claim is transfered by the owner.
		ClaimTransfered(T::AccountId, T::AccountId, Vec<u8>),
	}

	#[pallet::error]
	pub enum Error<T> {
		/// The claim already exists.
		ClaimAlreadyExist,
		ClaimTooLong,
		/// The claim does not exist, so it cannot be revoked.
		ClaimNotExist,
		/// The claim is owned by another account, so caller can't revoke it.
		NotClaimOwner,
	}

	#[pallet::storage]
	#[pallet::getter(fn claims)]
	pub type Claims<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		BoundedVec<u8, T::MaxClaimLength>,
		(T::AccountId, T::BlockNumber),
	>;

	// Dispatchable functions allow users to interact with the pallet and invoke state changes.
	// These functions materialize as "extrinsics", which are often compared to transactions.
	// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(T::WeightInfo::create_claim(claim.len() as u32))]
		pub fn create_claim(origin: OriginFor<T>, claim: Vec<u8>) -> DispatchResult {
			// Check that the extrinsic was signed and get the signer.
			// This function will return an error if the extrinsic is not signed.
			let sender = ensure_signed(origin)?;

			let bounded_claim = BoundedVec::<u8, T::MaxClaimLength>::try_from(claim.clone())
				.map_err(|_| Error::<T>::ClaimTooLong)?;

			// Verify that the specified claim has not already been stored.
			ensure!(!Claims::<T>::contains_key(&bounded_claim), Error::<T>::ClaimAlreadyExist);

			// Get the block number from the FRAME System pallet.
			let current_block = <frame_system::Pallet<T>>::block_number();

			// Store the claim with the sender and block number.
			Claims::<T>::insert(&bounded_claim, (&sender, current_block));

			// Emit an event that the claim was created.
			Self::deposit_event(Event::ClaimCreated(sender, claim));

			Ok(())
		}

		#[pallet::weight(T::WeightInfo::revoke_claim(claim.len() as u32))]
		pub fn revoke_claim(origin: OriginFor<T>, claim: Vec<u8>) -> DispatchResult {
			// Check that the extrinsic was signed and get the signer.
			// This function will return an error if the extrinsic is not signed.
			let sender = ensure_signed(origin)?;

			let bounded_claim = BoundedVec::<u8, T::MaxClaimLength>::try_from(claim.clone())
				.map_err(|_| Error::<T>::ClaimTooLong)?;

			// Get owner of the claim, if none return an error.
			let (owner, _) = Claims::<T>::get(&bounded_claim).ok_or(Error::<T>::ClaimNotExist)?;

			// Verify that sender of the current call is the claim owner.
			ensure!(owner == sender, Error::<T>::NotClaimOwner);

			// Remove claim from storage.
			Claims::<T>::remove(&bounded_claim);

			// Emit an event that the claim was erased.
			Self::deposit_event(Event::ClaimRevoked(sender, claim));

			Ok(())
		}

		#[pallet::weight(T::WeightInfo::transfer_claim(claim.len() as u32))]
		pub fn transfer_claim(
			origin: OriginFor<T>,
			claim: Vec<u8>,
			dest: T::AccountId,
		) -> DispatchResult {
			// Check that the extrinsic was signed and get the signer.
			// This function will return an error if the extrinsic is not signed.
			let sender = ensure_signed(origin)?;

			let bounded_claim = BoundedVec::<u8, T::MaxClaimLength>::try_from(claim.clone())
				.map_err(|_| Error::<T>::ClaimTooLong)?;

			// Get owner of the claim, if none return an error.
			let (owner, _) = Claims::<T>::get(&bounded_claim).ok_or(Error::<T>::ClaimNotExist)?;

			// Verify that sender of the current call is the claim owner.
			ensure!(owner == sender, Error::<T>::NotClaimOwner);

			// Get the block number from the FRAME System pallet.
			let current_block = <frame_system::Pallet<T>>::block_number();

			Claims::<T>::insert(&bounded_claim, (&dest, current_block));

			// Emit an event that the claim was transfered.
			Self::deposit_event(Event::ClaimTransfered(sender, dest, claim));

			Ok(())
		}
	}
}
