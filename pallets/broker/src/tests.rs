#![cfg(test)]

//use std::alloc::System;
use super::*;
use crate as broker;
use sp_core::H256;

use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
};

//use crate::{mock::*, Error};
use crate::Error;
use frame_support::{assert_noop, assert_ok,
					pallet_prelude::GenesisBuild,parameter_types,
					traits::OnInitialize,
					PalletId,};
use sp_runtime::traits::Hash;

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
		Broker: broker::{Pallet, Call, Storage, Config<T>, Event<T>},
		TimeStamp: pallet_timestamp::{Pallet, Call, Storage, Inherent},
	}
);

parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub BlockWeights: frame_system::limits::BlockWeights =
		frame_system::limits::BlockWeights::simple_max(1024);
}
impl frame_system::Config for Test {
	type BaseCallFilter = frame_support::traits::Everything;
	type BlockWeights = ();
	type BlockLength = ();
	type DbWeight = ();
	type Origin = Origin;
	type Index = u64;
	type BlockNumber = u64;
	type Call = Call;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = u128; // u64 is not enough to hold bytes used to generate bounty account
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type Event = Event;
	type BlockHashCount = BlockHashCount;
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = pallet_balances::AccountData<u64>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = ();
	type OnSetCode = ();
}
parameter_types! {
	pub const ExistentialDeposit: u64 = 1;
}
impl pallet_balances::Config for Test {
	type MaxLocks = ();
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
	type Balance = u64;
	type Event = Event;
	type DustRemoval = ();
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type WeightInfo = ();
}

impl Config for Test {
	type Event = Event;
	type Currency = pallet_balances::Pallet<Test>;
}


impl pallet_timestamp::Config for Test {
	/// A timestamp: milliseconds since the unix epoch.
	type Moment = u64;
	type OnTimestampSet = ();
	type MinimumPeriod = ();
	type WeightInfo = ();
}

pub fn new_test_ext() -> sp_io::TestExternalities {
	let mut t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();
	pallet_balances::GenesisConfig::<Test> {
		// Total issuance will be 200 with treasury account initialized at ED.
		balances: vec![(0, 100), (1, 98), (2, 1)],
	}
		.assimilate_storage(&mut t)
		.unwrap();
	//GenesisBuild::<Test>::assimilate_storage(&crate::GenesisConfig, &mut t).unwrap();
	t.into()
}

#[test]
fn grant_brokerage_should_work() {
	new_test_ext().execute_with(|| {
		// Dispatch a signed extrinsic.
		//assert_ok!(broker::do_something(Origin::signed(1), 42));
		assert_ok!(Broker::grant_brokerage(Origin::signed(0), 1, 1_000_000_000_000));

		let brokerage_id = BlakeTwo256::hash_of(&Brokerage::<Test> {
			client: 0,
			broker: 1,
			fixed_block_limit: 1_000_000_000_000,
			reductive_block_limit: 1_000_000_000_000,
			last_transaction_timestamp: None,
		});

		assert_eq!(
			Broker::brokerages(brokerage_id).unwrap(),
			Brokerage {
				client: 0,
				broker: 1,
				fixed_block_limit: 1_000_000_000_000,
				reductive_block_limit: 1_000_000_000_000,
				last_transaction_timestamp: None,
			}
		);

		assert_ok!(Broker::transfer(Origin::signed(1), brokerage_id, 2, 10));
		// Read pallet storage and assert an expected result.
		//assert_eq!(Broker::something(), Some(42));
	});
}

#[test]
fn correct_error_for_none_value() {
	new_test_ext().execute_with(|| {
		// Ensure the expected error is thrown when no value is present.
		//assert_noop!(Broker::cause_error(Origin::signed(1)), Error::<Test>::NoneValue);
	});
}
