use ckb_sdk::Address;
use ckb_types::{H160, H256};

pub fn read_sighash_all_records() -> Vec<(H160, u64)> {
    Default::default()
}

pub fn read_multisig_all_records() -> Vec<(H160, u128, u64)> {
    Default::default()
}
