#![cfg_attr(not(feature = "std"), no_std)]

/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// <https://substrate.dev/docs/en/knowledgebase/runtime/frame>
pub use pallet::*;
use frame_support::sp_runtime::{MultiSignature, AccountId32};
use codec::{Decode, Encode};
use sp_std::prelude::*;
pub use pallet::*;
use frame_support::sp_runtime::traits::{Verify, IdentifyAccount};
use sp_std::marker::PhantomData;
use xcm::v0::{
	MultiLocation::{X1,X2,Null},
};

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

type BalanceOf = u128;

type ParaId = u32;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::{dispatch::DispatchResultWithPostInfo, pallet_prelude::*};
	use frame_system::pallet_prelude::*;
	use super::*;
	use crate::{ BalanceOf};
	use xcm::v0::{Xcm, OriginKind, MultiLocation, Junction, SendXcm};
	use frame_system::Origin;
	use frame_support::traits::ReservableCurrency;

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		type Origin: From<Origin<Self>> + IsType<<Self as frame_system::Config>::Origin>;

		/// The overarching call type; we assume sibling chains use the same type.
		type Call: From<Call<Self>> + Encode;

		type XcmSender: SendXcm;

		type SendXcmOrigin: EnsureOrigin<<Self as frame_system::Config>::Origin, Success=MultiLocation>;

		type Currency: ReservableCurrency<Self::AccountId>;

	}


	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[derive(Encode, Decode)]
	pub enum CrowdloanPalletCall{
		#[codec(index = 66)] // the index should match the position of the module in `construct_runtime!`
		CrowdloanContribute(ContributeCall),
	}

	#[derive(Debug, PartialEq, Encode, Decode)]
	pub struct Contribution {
		#[codec(compact)]
		index: ParaId,
		contributor: AccountId32,
		#[codec(compact)]
		value: BalanceOf,
	}

	#[derive(Encode, Decode)]
	pub enum ContributeCall {
		#[codec(index = 4)] // the index should match the position of the dispatchable in the target pallet
		Contribute(Contribution),
	}

	// The pallet's runtime storage items.
	// https://substrate.dev/docs/en/knowledgebase/runtime/storage
	#[pallet::storage]
	#[pallet::getter(fn something)]
	// Learn more about declaring storage items:
	// https://substrate.dev/docs/en/knowledgebase/runtime/storage#declaring-storage-items
	pub type Something<T> = StorageValue<_, u32>;

	// Pallets use events to inform users when important changes are made.
	// https://substrate.dev/docs/en/knowledgebase/runtime/events
	#[pallet::event]
	#[pallet::metadata(T::AccountId = "AccountId")]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Event documentation should end with an array that provides descriptive names for event
		/// parameters. [something, who]
		SomethingStored(u32, T::AccountId),
		SendingSuccess(),
		SendingError(),
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// Error names should be descriptive.
		NoneValue,
		/// Errors should have helpful documentation associated with them.
		StorageOverflow,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

	// Dispatchable functions allows users to interact with the pallet and invoke state changes.
	// These functions materialize as "extrinsics", which are often compared to transactions.
	// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// An example dispatchable that takes a singles value as a parameter, writes the value to
		/// storage and emits an event. This function must be dispatched by a signed extrinsic.
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
		pub fn do_something(origin: OriginFor<T>, something: u32) -> DispatchResultWithPostInfo {
			// Check that the extrinsic was signed and get the signer.
			// This function will return an error if the extrinsic is not signed.
			// https://substrate.dev/docs/en/knowledgebase/runtime/origin
			let who = ensure_signed(origin)?;

			// Update storage.
			<Something<T>>::put(something);

			// Emit an event.
			Self::deposit_event(Event::SomethingStored(something, who));
			// Return a successful DispatchResultWithPostInfo
			Ok(().into())
		}

		/// An example dispatchable that may throw a custom error.
		#[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1))]
		pub fn cause_error(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
			let _who = ensure_signed(origin)?;

			// Read a value from storage.
			match <Something<T>>::get() {
				// Return an error if the value has not been set.
				None => Err(Error::<T>::NoneValue)?,
				Some(old) => {
					// Increment the value read from storage; will error in the event of overflow.
					let new = old.checked_add(1).ok_or(Error::<T>::StorageOverflow)?;
					// Update the value in storage with the incremented result.
					<Something<T>>::put(new);
					Ok(().into())
				}
			}
		}

		#[pallet::weight(0)]
		fn test_downward_crowdloan_contribute(origin: OriginFor<T>, #[pallet::compact] para_id: u32, #[pallet::compact] value: BalanceOf) -> DispatchResult {
			let _who = ensure_signed(origin.clone())?;

			let origin_location:MultiLocation = T::SendXcmOrigin::ensure_origin(origin)?;

			// let account_location = origin_location.first().ok_or_else(|| Error::<T>::NoneValue)?;

			match origin_location {
				X1(Junction::AccountId32 { id,.. }) => {


					let contribution = Contribution{ index: para_id.clone(), contributor: AccountId32::from(id.clone()), value };

					let call = CrowdloanPalletCall::CrowdloanContribute(ContributeCall::Contribute(contribution)).encode();

					let msg = Xcm::Transact {
						origin_type: OriginKind::SovereignAccount,
						require_weight_at_most: u64::MAX,
						call: call.into(),
					};

					log::info!(
						target: "template",
						"crowdloan transact as {:?}",
						msg,
					);

					match T::XcmSender::send_xcm(MultiLocation::X1(Junction::Parachain(para_id)), msg) {
						Ok(()) => {
							Self::deposit_event(Event::SendingSuccess());
							log::info!(
								target: "template",
								"crowdloan transact sent success!"
							);
						},
						Err(e) => {
							Self::deposit_event(Event::SendingError());
							log::error!(
								target: "template",
								"crowdloan transact sent failed:{:?}",
								e,
							);
						}
					}
				}
				_ => {}
			}
			Ok(())
		}
	}
}
