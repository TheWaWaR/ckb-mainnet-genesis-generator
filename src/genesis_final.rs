use crate::consts::ONE_CKB;
use crate::basic::Address;
use ckb_types::{H160, H256, core::ScriptHashType, bytes::Bytes, packed, prelude::*};

use std::fs;

pub fn read_all_records(last_epoch: u64) -> Vec<(packed::Script, u64)> {
    let mut results = Vec::new();
    let mut total_capacity = 0;
    for line in crate::data::DATA_GENESIS_FINAL.split('\n') {
        if line.trim().is_empty() {
            continue;
        }
        let parts = line.trim().split(',').collect::<Vec<_>>();
        let address_str = parts[0];
        let capacity_str = parts[1];
        let datetime_str = parts[2];
        let address = Address::from_input(address_str).unwrap().1;
        let capacity = capacity_str.parse::<u64>().unwrap() * ONE_CKB;
        let lock_script = match datetime_str {
            "\"\"" | "" => {
                packed::Script::new_builder()
                    .code_hash(crate::consts::SECP_TYPE_SCRIPT_HASH.pack())
                    .hash_type(ScriptHashType::Type.into())
                    .args(Bytes::from(address.hash().as_bytes()).pack())
                    .build()
            },
            value => {
                let lock_arg: Bytes = crate::build_multisig_lock_arg(address, value, last_epoch).into();
                packed::Script::new_builder()
                    .code_hash(crate::consts::MULTISIG_TYPE_SCRIPT_HASH.pack())
                    .hash_type(ScriptHashType::Type.into())
                    .args(lock_arg.pack())
                    .build()
            },
        };
        total_capacity += capacity;
        results.push((lock_script, capacity));
    }
    println!("genesis_final.total_capacity: {}", total_capacity);
    results
}
