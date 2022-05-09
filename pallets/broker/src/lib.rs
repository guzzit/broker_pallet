#![cfg_attr(not(feature = "std"), no_std)]

/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// <https://docs.substrate.io/v3/runtime/frame>
pub use pallet::*;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
mod tests;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::{dispatch::DispatchResult, log, pallet_prelude::*};
	use frame_system::pallet_prelude::*;
	use frame_support::{
		sp_runtime::traits::Hash,
		traits::{ Randomness, Currency, tokens::ExistenceRequirement },
		transactional
	};
	use pallet_timestamp::{self as timestamp};

	use scale_info::TypeInfo;

	type AccountOf<T> = <T as frame_system::Config>::AccountId;
	type BalanceOf<T> =
	<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;


	// Struct for holding broker information.
	#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo)]
	#[scale_info(skip_type_params(T))]
	pub struct Brokerage<T: Config> {
		pub client: AccountOf<T>,
		pub broker: AccountOf<T>,
		pub fixed_block_limit: BalanceOf<T>,
		pub reductive_block_limit: BalanceOf<T>,
		pub last_transaction_timestamp: Option<T::Moment>,
	}

	// Struct for modifying broker information.
	#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo)]
	#[scale_info(skip_type_params(T))]
	pub struct ModifyBrokerage<T: Config>{
		pub fixed_block_limit: BalanceOf<T>,
		pub reductive_block_limit: BalanceOf<T>,
		pub last_transaction_timestamp: Option<T::Moment>,
	}

	#[pallet::storage]
	#[pallet::getter(fn brokerages)]
	/// Stores a list of brokerages
	pub(super) type Brokerages<T: Config> = StorageMap<_, Twox64Concat, T::Hash, Brokerage<T>>;
	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config + timestamp::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		/// The Currency handler for the Broker pallet.
		type Currency: Currency<Self::AccountId>;
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	// The pallet's runtime storage items.
	// https://docs.substrate.io/v3/runtime/storage
	#[pallet::storage]
	#[pallet::getter(fn something)]
	// Learn more about declaring storage items:
	// https://docs.substrate.io/v3/runtime/storage#declaring-storage-items
	pub type Something<T> = StorageValue<_, u32>;

	// Pallets use events to inform users when important changes are made.
	// https://docs.substrate.io/v3/runtime/events-and-errors
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A new Brokerage was sucessfully granted. \[client, broker, brokerage_id\]
		Created(T::AccountId, T::AccountId, T::Hash),
		/// A Brokerage was sucessfully modified. \[client, brokerage_id\]
		Modified(AccountOf<T>, T::Hash),
		/// A new Brokerage was sucessfully granted. \[client\]
		Revoked(T::AccountId),
		/// Succesful tranfer through a Brokerage. \[amount, to, brokerage_id\]
		Transferred(BalanceOf<T>, AccountOf<T>, T::Hash),
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// Broker Id does not exist
		BrokerNotExist,
		/// Expiry date should not be set before the present UTC date
		ExpiryDateBeforePresentUTCDate,
		/// Brokerage Id does not exist
		BrokerageNotExist,
		/// Unauthorized to process operation
		Unauthorized,
		/// Amount for transaction greater than fixed daily limit
		AmountGreaterThanFixedBlockLimit,
		/// Amount for transaction greater than reductive daily limit
		AmountGreaterThanReductiveBlockLimit,
		/// Reductive daily limit greater than fixed daily limit
		ReductiveDailyLimitMoreThanFixedBlockLimit,
	}

	// Our pallet's genesis configuration.
	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub brokerages: Vec<(T::AccountId, T::AccountId, BalanceOf<T>)>,
	}

	// Required to implement default for GenesisConfig.
	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> GenesisConfig<T> {
			GenesisConfig { brokerages: vec![] }
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			// When building a kitty from genesis config, we require the dna and gender to be supplied.
			for (client_id, broker_id, fixed_block_limit) in &self.brokerages {
				let _ = <Pallet<T>>::create_brokerage(client_id.clone(), broker_id.clone(), fixed_block_limit.clone());
			}
		}
	}

	// Dispatchable functions allows users to interact with the pallet and invoke state changes.
	// These functions materialize as "extrinsics", which are often compared to transactions.
	// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(100)]
		pub fn grant_brokerage(
			origin: OriginFor<T>,
			broker_id: AccountOf<T>,
			fixed_block_limit: BalanceOf<T>,
		) -> DispatchResult {
			let client = ensure_signed(origin)?;

			let brokerage_id = Self::create_brokerage(client.clone(), broker_id.clone(),
													  fixed_block_limit)?;
			log::info!("New brokerage created with ID: {:?}.", &brokerage_id);
			Self::deposit_event(Event::Created(client, broker_id, brokerage_id));
			Ok(())
		}

		#[pallet::weight(100)]
		pub fn revoke_brokerage(
			origin: OriginFor<T>,
			brokerage_id: T::Hash,
		) -> DispatchResult {
			let client = ensure_signed(origin)?;

			let brokerage = Self::brokerages(&brokerage_id).ok_or(<Error<T>>::BrokerageNotExist)?;

			ensure!(brokerage.client == client, <Error<T>>::Unauthorized);

			let remove_brokerage = <Brokerages<T>>::remove(&brokerage_id);

			log::info!("Brokerage with ID: {:?} revoked", &brokerage_id);
			Self::deposit_event(Event::Revoked(client));
			Ok(())
		}

		#[pallet::weight(100)]
		pub fn modify_brokerage(
			origin: OriginFor<T>,
			brokerage_id: T::Hash,
			fixed_block_limit: BalanceOf<T>,
		) -> DispatchResult {
			let client = ensure_signed(origin)?;

			let brokerage = Self::brokerages(&brokerage_id).ok_or(<Error<T>>::BrokerageNotExist)?;

			ensure!(brokerage.client == client, <Error<T>>::Unauthorized);

			let updated_brokerage = ModifyBrokerage::<T>{
				fixed_block_limit,
				last_transaction_timestamp: None,
				reductive_block_limit: brokerage.reductive_block_limit,
			};

			Self::update_brokerage(updated_brokerage, brokerage_id.clone(), None)?;
			log::info!("Brokerage with ID: {:?} modified.", &brokerage_id);
			Self::deposit_event(Event::Modified(client,  brokerage_id));
			Ok(())
		}

		#[transactional]
		#[pallet::weight(100)]
		pub fn transfer(
			origin: OriginFor<T>,
			brokerage_id: T::Hash,
			to: AccountOf<T>,
			amount: BalanceOf<T>
		) -> DispatchResult {
			let broker = ensure_signed(origin)?;

			Self::process_transfer(brokerage_id,broker,to.clone(), amount)?;
			log::info!("Transfer of {:?} successful to account with ID {:?} through brokerage with ID {:?}", amount, to, brokerage_id);
			Self::deposit_event(Event::Transferred(amount, to, brokerage_id));
			Ok(())
		}

	}

	impl<T: Config> Pallet<T> {

		pub fn create_brokerage(
			client_id: AccountOf<T>,
			broker_id: AccountOf<T>,
			fixed_block_limit: BalanceOf<T>,
		) -> Result<T::Hash, Error<T>> {

			let brokerage = Brokerage::<T>{
				client: client_id,
				broker: broker_id,
				fixed_block_limit,
				reductive_block_limit: fixed_block_limit,
				last_transaction_timestamp: None,
			};

			let validate_update = Self::validate_brokerage(brokerage.clone())?;

			let brokerage_id = T::Hashing::hash_of(&brokerage);
			<Brokerages<T>>::insert(brokerage_id, brokerage);
			Ok(brokerage_id)
		}

		pub fn validate_brokerage(
			brokerage: Brokerage<T>,
		) -> Result<bool, Error<T>> {

			let broker_exists = frame_system::Pallet::<T>::account_exists(&brokerage.broker);
			//ensure broker exists
			ensure!(broker_exists, <Error<T>>::BrokerNotExist);

			//ensure reductive daily limit less than fixed daily limit
			ensure!(brokerage.reductive_block_limit <= brokerage.fixed_block_limit, <Error<T>>::ReductiveDailyLimitMoreThanFixedBlockLimit);

			match brokerage.last_transaction_timestamp {
				None => (),
				Some(timestamp) => {
					let _now = <timestamp::Pallet<T>>::get();

					ensure!(_now <= timestamp, <Error<T>>::ExpiryDateBeforePresentUTCDate);
				}
			}

			Ok(true)
		}

		pub fn update_brokerage(
			brokerageParams: ModifyBrokerage<T>,
			brokerage_id: T::Hash,
			amount: Option<BalanceOf<T>>,
		) -> Result<(), Error<T>> {

			let mut brokerage = Self::brokerages(&brokerage_id).ok_or(<Error<T>>::BrokerageNotExist)?;

			//add all the input validation
			match amount {
				None => {
					brokerage.reductive_block_limit = brokerageParams.reductive_block_limit;
				},
				Some(transactionAmount) => {
					brokerage.reductive_block_limit = brokerageParams.reductive_block_limit - transactionAmount;
				}
			}

			brokerage.fixed_block_limit = brokerageParams.fixed_block_limit;
			brokerage.last_transaction_timestamp = brokerageParams.last_transaction_timestamp;

			let validate_update = Self::validate_brokerage(brokerage.clone())?;

			<Brokerages<T>>::insert(brokerage_id, brokerage);

			Ok(())
		}

		#[transactional]
		pub fn process_transfer(
			brokerage_id: T::Hash,
			broker: AccountOf<T>,
			to: AccountOf<T>,
			amount: BalanceOf<T>
		) -> DispatchResult {

			let mut brokerage = Self::brokerages(&brokerage_id).ok_or(<Error<T>>::BrokerageNotExist)?;

			ensure!(broker == brokerage.broker, <Error<T>>::Unauthorized);

			let last_tran_timestamp = brokerage.last_transaction_timestamp;

			let mut reductive_block_limit = brokerage.reductive_block_limit;
			match last_tran_timestamp {
				None => (),
				Some(timestamp) => {
					//if it's a new block, update the reductive block limit and last transaction timestamp
					let _now = <timestamp::Pallet<T>>::get();
					if _now > timestamp {
						//update reductive daily limit
						reductive_block_limit = brokerage.fixed_block_limit;
					}
					else {
						//consider transaction fee in refactor
						ensure!(amount <= brokerage.reductive_block_limit, <Error<T>>::AmountGreaterThanReductiveBlockLimit);
					}

				}
			}

			//consider transaction fee in refactor
			ensure!(amount <= brokerage.fixed_block_limit, <Error<T>>::AmountGreaterThanFixedBlockLimit);

			let from = brokerage.client.clone();
			// Transfer the amount from buyer to seller
			T::Currency::transfer(&from, &to, amount, ExistenceRequirement::KeepAlive)?;

			let _now = <timestamp::Pallet<T>>::get();

			//update reductive_daily_limit and last_tran_date
			let updated_brokerage = ModifyBrokerage::<T>{
				fixed_block_limit: brokerage.fixed_block_limit,
				reductive_block_limit,
				last_transaction_timestamp: Option::from(_now),
			};

			Self::update_brokerage(updated_brokerage, brokerage_id, Option::from(amount))?;

			Ok(())
		}
	}
}

// #[cfg(test)]
// mod tests {
// 	use super::*;
// 	use crate as pallet_broker;
// 	use crate::Pallet;
//
// 	//use frame_support::{assert_noop, assert_ok, ord_parameter_types, parameter_types};
// 	use frame_support::{
// 		assert_noop, assert_ok,
// 		pallet_prelude::GenesisBuild,
// 		parameter_types,
// 		traits::{ConstU32, ConstU64, OnInitialize},
// 		PalletId,
// 	};
// 	use frame_system::EnsureSignedBy;
// 	use frame_system::pallet_prelude::*;
// 	use sp_core::H256;
// 	use sp_runtime::{
// 		testing::Header,
// 		traits::{BadOrigin, BlakeTwo256, IdentityLookup},
// 	};
//
// 	type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
// 	type Block = frame_system::mocking::MockBlock<Test>;
//
// 	frame_support::construct_runtime!(
// 		pub enum Test where
// 			Block = Block,
// 			NodeBlock = Block,
// 			UncheckedExtrinsic = UncheckedExtrinsic,
// 		{
// 			System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
// 			Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
// 			Broker: pallet_broker::{Pallet, Call, Storage, Event<T>},
// 		}
// 	);
//
// 	parameter_types! {
// 		pub const BlockHashCount: u64 = 250;
// 		pub BlockWeights: frame_system::limits::BlockWeights =
// 			frame_system::limits::BlockWeights::simple_max(1024);
// 	}
// 	impl frame_system::Config for Test {
// 		type BaseCallFilter = frame_support::traits::Everything;
// 		type BlockWeights = ();
// 		type BlockLength = ();
// 		type Origin = Origin;
// 		type Call = Call;
// 		type Index = u64;
// 		type BlockNumber = u64;
// 		type Hash = H256;
// 		type Hashing = BlakeTwo256;
// 		type AccountId = u64;
// 		type Lookup = IdentityLookup<Self::AccountId>;
// 		type Header = Header;
// 		type Event = Event;
// 		type BlockHashCount = BlockHashCount;
// 		type DbWeight = ();
// 		type Version = ();
// 		type PalletInfo = PalletInfo;
// 		type AccountData = pallet_balances::AccountData<u64>;
// 		type OnNewAccount = ();
// 		type OnKilledAccount = ();
// 		type SystemWeightInfo = ();
// 		type SS58Prefix = ();
// 		type OnSetCode = ();
// 	}
//
//
// 	impl pallet_timestamp::Config for Test {
// 		/// A timestamp: milliseconds since the unix epoch.
// 		type Moment = u64;
// 		type OnTimestampSet = ();
// 		type MinimumPeriod = ();
// 		type WeightInfo = ();
// 	}
//
//
// 	parameter_types! {
// 		pub const ExistentialDeposit: u64 = 1;
// 	}
//
// 	type Balance = u64;
//
// 	impl pallet_balances::Config for Test {
// 		type MaxLocks = ();
// 		type MaxReserves = ();
// 		type ReserveIdentifier = [u8; 8];
// 		type Balance = Balance;
// 		type Event = Event;
// 		type DustRemoval = ();
// 		type ExistentialDeposit = ConstU64<1>;
// 		type AccountStore = System;
// 		type WeightInfo = ();
// 	}
// 	// impl pallet_balances::Config for Test {
// 	// 	type MaxLocks = ();
// 	// 	type MaxReserves = ();
// 	// 	type ReserveIdentifier = [u8; 8];
// 	// 	type Balance = u64;
// 	// 	type Event = Event;
// 	// 	type DustRemoval = ();
// 	// 	type ExistentialDeposit = ExistentialDeposit;
// 	// 	type AccountStore = System;
// 	// 	type WeightInfo = ();
// 	// }
// 	parameter_types! {
// 		pub const ReservationFee: u64 = 2;
// 		pub const MinLength: u32 = 3;
// 		pub const MaxLength: u32 = 16;
// 	}
// 	ord_parameter_types! {
// 		pub const One: u64 = 1;
// 	}
// 	impl Config for Test {
// 		type Event = Event;
// 		type Currency = Balances;
// 	}
//
// 	fn new_test_ext() -> sp_io::TestExternalities {
// 		let mut t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();
// 		pallet_balances::GenesisConfig::<Test> { balances: vec![(1, 10), (2, 10)] }
// 			.assimilate_storage(&mut t)
// 			.unwrap();
// 		t.into()
// 	}
//
// 	//grant brokerage should work
// 	//modify brokerage should work
// 	//revoke brokerage should work
// 	//transaction with brokerage should work
// 	//errors
//
// 	#[test]
// 	fn grant_brokerage_should_work() {
// 		new_test_ext().execute_with(|| {
// 			//assert_eq!(Balances::total_balance(&2), 10);
// 			let acct = 0x1Au64;
// 			assert_ok!(Broker::grant_brokerage(Origin::signed(2), acct,
// 				1_000_000_000_000));
// 			//assert_eq!(Balances::total_balance(&2), 8);
// 		});
// 	}
//
// 	// #[test]
// 	// fn kill_name_should_work() {
// 	// 	new_test_ext().execute_with(|| {
// 	// 		assert_ok!(Nicks::set_name(Origin::signed(2), b"Dave".to_vec()));
// 	// 		assert_eq!(Balances::total_balance(&2), 10);
// 	// 		assert_ok!(Nicks::kill_name(Origin::signed(1), 2));
// 	// 		assert_eq!(Balances::total_balance(&2), 8);
// 	// 		assert_eq!(<NameOf<Test>>::get(2), None);
// 	// 	});
// 	// }
// 	//
// 	// #[test]
// 	// fn force_name_should_work() {
// 	// 	new_test_ext().execute_with(|| {
// 	// 		assert_noop!(
// 	// 			Nicks::set_name(Origin::signed(2), b"Dr. David Brubeck, III".to_vec()),
// 	// 			Error::<Test>::TooLong,
// 	// 		);
// 	//
// 	// 		assert_ok!(Nicks::set_name(Origin::signed(2), b"Dave".to_vec()));
// 	// 		assert_eq!(Balances::reserved_balance(2), 2);
// 	// 		assert_ok!(Nicks::force_name(Origin::signed(1), 2, b"Dr. David Brubeck, III".to_vec()));
// 	// 		assert_eq!(Balances::reserved_balance(2), 2);
// 	// 		assert_eq!(<NameOf<Test>>::get(2).unwrap(), (b"Dr. David Brubeck, III".to_vec(), 2));
// 	// 	});
// 	// }
// 	//
// 	// #[test]
// 	// fn normal_operation_should_work() {
// 	// 	new_test_ext().execute_with(|| {
// 	// 		assert_ok!(Nicks::set_name(Origin::signed(1), b"Gav".to_vec()));
// 	// 		assert_eq!(Balances::reserved_balance(1), 2);
// 	// 		assert_eq!(Balances::free_balance(1), 8);
// 	// 		assert_eq!(<NameOf<Test>>::get(1).unwrap().0, b"Gav".to_vec());
// 	//
// 	// 		assert_ok!(Nicks::set_name(Origin::signed(1), b"Gavin".to_vec()));
// 	// 		assert_eq!(Balances::reserved_balance(1), 2);
// 	// 		assert_eq!(Balances::free_balance(1), 8);
// 	// 		assert_eq!(<NameOf<Test>>::get(1).unwrap().0, b"Gavin".to_vec());
// 	//
// 	// 		assert_ok!(Nicks::clear_name(Origin::signed(1)));
// 	// 		assert_eq!(Balances::reserved_balance(1), 0);
// 	// 		assert_eq!(Balances::free_balance(1), 10);
// 	// 	});
// 	// }
// 	//
// 	// #[test]
// 	// fn error_catching_should_work() {
// 	// 	new_test_ext().execute_with(|| {
// 	// 		assert_noop!(Nicks::clear_name(Origin::signed(1)), Error::<Test>::Unnamed);
// 	//
// 	// 		assert_noop!(
// 	// 			Nicks::set_name(Origin::signed(3), b"Dave".to_vec()),
// 	// 			pallet_balances::Error::<Test, _>::InsufficientBalance
// 	// 		);
// 	//
// 	// 		assert_noop!(
// 	// 			Nicks::set_name(Origin::signed(1), b"Ga".to_vec()),
// 	// 			Error::<Test>::TooShort
// 	// 		);
// 	// 		assert_noop!(
// 	// 			Nicks::set_name(Origin::signed(1), b"Gavin James Wood, Esquire".to_vec()),
// 	// 			Error::<Test>::TooLong
// 	// 		);
// 	// 		assert_ok!(Nicks::set_name(Origin::signed(1), b"Dave".to_vec()));
// 	// 		assert_noop!(Nicks::kill_name(Origin::signed(2), 1), BadOrigin);
// 	// 		assert_noop!(Nicks::force_name(Origin::signed(2), 1, b"Whatever".to_vec()), BadOrigin);
// 	// 	});
// 	// }
// }
