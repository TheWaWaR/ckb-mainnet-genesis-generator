use crate::consts::ONE_CKB;
use crate::AddressParser;
use ckb_types::{H160, H256};

pub fn read_sighash_all_records() -> Vec<(H160, u64)> {
    let mut rdr = csv::Reader::from_reader(crate::data::DATA_SIGHASH_ALL.as_bytes());
    let mut results = Vec::new();
    for record in rdr.records() {
        let record = record.unwrap();
        let address_str = record.get(0).unwrap();
        if address_str.is_empty() {
            log::warn!("empty address lock_hash={}", record.get(1).unwrap());
            continue;
        }
        match AddressParser.parse(address_str) {
            Ok(hash) => {
                let capacity: u64 = record.get(1).unwrap().parse::<u64>().unwrap() * ONE_CKB;
                log::debug!("{:#} => {}", hash, capacity);
                results.push((hash, capacity));
            }
            Err(err) => {
                log::warn!("invalid address {}, error: {}", address_str, err);
            }
        }
    }
    results
}

pub fn read_multisig_all_records() -> Vec<(H160, u128, u64)> {
    Default::default()
}
