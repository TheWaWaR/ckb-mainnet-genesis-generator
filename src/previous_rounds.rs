use crate::consts::ONE_CKB;
use crate::AddressParser;
use ckb_hash::new_blake2b;
use ckb_types::{H160, H256};
use std::collections::HashMap;

pub fn read_round1_rewards() -> Vec<(H160, u64)> {
    let mut rdr = csv::Reader::from_reader(crate::data::DATA_ROUND1.as_bytes());
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

pub fn read_round2_rewards() -> Vec<(H160, u64)> {
    read_epoch_lucky_rewords(
        crate::data::DATA_ROUND2_MINER,
        crate::data::DATA_ROUND2_EPOCH,
        crate::consts::EPOCH_REWARD2,
    )
}

pub fn read_round3_rewards() -> Vec<(H160, u64)> {
    read_epoch_lucky_rewords(
        crate::data::DATA_ROUND3_MINER,
        crate::data::DATA_ROUND3_EPOCH,
        crate::consts::EPOCH_REWARD3,
    )
}

pub fn read_epoch_lucky_rewords(
    miner_data: &str,
    epoch_data: &str,
    epoch_reward: u64,
) -> Vec<(H160, u64)> {
    let mut results = Vec::new();
    let mut rdr_miner = csv::Reader::from_reader(miner_data.as_bytes());
    for record in rdr_miner.records() {
        let record = record.unwrap();
        let address_str = record.get(0).unwrap();
        if address_str.is_empty() {
            log::warn!(
                "empty address info: {}, {}",
                record.get(1).unwrap(),
                record.get(2).unwrap()
            );
            continue;
        }
        match AddressParser.parse(address_str) {
            Ok(hash) => {
                let capacity: u64 = record.get(3).unwrap().parse::<u64>().unwrap() * ONE_CKB;
                log::debug!("miner {:#} => {}", hash, capacity);
                results.push((hash, capacity));
            }
            Err(err) => {
                log::warn!("invalid address {}, error: {}", address_str, err);
            }
        }
    }

    let mut rdr_epoch = csv::Reader::from_reader(epoch_data.as_bytes());
    for record in rdr_epoch.records() {
        let record = record.unwrap();
        let address_str = record.get(1).unwrap();
        if address_str.is_empty() {
            log::warn!("empty address lock-hash: {}", record.get(2).unwrap());
            continue;
        }
        match AddressParser.parse(address_str) {
            Ok(hash) => {
                log::debug!("epoch {:#} => {}", hash, epoch_reward);
                results.push((hash, epoch_reward));
            }
            Err(err) => {
                log::warn!("invalid address {}, error: {}", address_str, err);
            }
        }
    }
    results
}

pub fn read_round4_rewards() -> Vec<(H160, u64)> {
    read_normal_rewards(crate::data::DATA_ROUND4)
}

pub fn read_round5_stage1_rewards() -> Vec<(H160, u64)> {
    read_normal_rewards(crate::data::DATA_ROUND5_STAGE1)
}

pub fn read_round5_stage2_rewards() -> Vec<(H160, u64)> {
    read_normal_rewards(crate::data::DATA_ROUND5_STAGE2)
}

pub fn read_normal_rewards(data: &str) -> Vec<(H160, u64)> {
    let mut results = Vec::new();
    let mut rdr = csv::Reader::from_reader(data.as_bytes());
    for record in rdr.records() {
        let record = record.unwrap();
        let address_str = record.get(0).unwrap();
        if address_str.is_empty() {
            log::warn!(
                "empty address info: {}, {}",
                record.get(1).unwrap(),
                record.get(2).unwrap()
            );
            continue;
        }
        match AddressParser.parse(address_str) {
            Ok(hash) => {
                let capacity: u64 = record.get(3).unwrap().parse::<u64>().unwrap() * ONE_CKB;
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

pub fn all_rewards() -> Vec<(H160, u64)> {
    let mut result: HashMap<H160, u64> = HashMap::default();

    for (round_name, round_rewards) in vec![
        ("1", read_round1_rewards()),
        ("2", read_round2_rewards()),
        ("3", read_round3_rewards()),
        ("4", read_round4_rewards()),
        ("5-stage1", read_round5_stage1_rewards()),
        ("5-stage2", read_round5_stage2_rewards()),
    ]
    .into_iter()
    {
        let mut total_capacity = 0;
        let count = round_rewards.len();
        for (lock_hash, capacity) in round_rewards {
            log::info!("round{}: {:#} => {}", round_name, lock_hash, capacity);
            total_capacity += capacity;
            *result.entry(lock_hash).or_default() += capacity;
        }
        println!(
            "==== Round {}, count: {}, total_capacity: {}\n",
            round_name, count, total_capacity
        );
    }

    let mut rewards: Vec<(H160, u64)> = result.into_iter().collect();
    rewards.sort_by(|a, b| a.0.cmp(&b.0));

    let mut blake2b = new_blake2b();
    let mut total_capacity = 0;
    for (lock_hash, capacity) in &rewards {
        println!("previous: {:#} => {}", lock_hash, capacity);
        blake2b.update(lock_hash.as_bytes());
        total_capacity += capacity;
    }
    let mut digest = [0u8; 32];
    blake2b.finalize(&mut digest);
    let digest = H256::from_slice(&digest[..]).unwrap();
    println!(
        "lock-hash-digest: {:#}, count: {}, total-capacity: {}\n",
        digest,
        rewards.len(),
        total_capacity,
    );
    rewards
}
