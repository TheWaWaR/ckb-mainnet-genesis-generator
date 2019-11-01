use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::path::Path;
use std::thread;
use std::time::Duration;
use chrono::prelude::*;

use ckb_jsonrpc_types::BlockNumber;
use ckb_sdk::HttpRpcClient;
use ckb_types::{
    core::{EpochNumberWithFraction, ScriptHashType},
    packed,
    prelude::*,
    H160, H256,
};
use serde::{Deserialize, Serialize};

use crate::consts::SECP_TYPE_SCRIPT_HASH;

#[derive(Debug, Deserialize, Serialize)]
pub struct CurrentTestnetResult {
    pub rewards: Vec<(H160, u64)>,
    pub total_base_reward: u64,
    pub last_block_hash: H256,
    pub last_block_number: u64,
}

impl CurrentTestnetResult {
    pub fn new(
        rewards: HashMap<H160, u64>,
        total_base_reward: u64,
        last_block_hash: H256,
        last_block_number: u64,
    ) -> Self {
        let rewards: Vec<(H160, u64)> = rewards.into_iter().collect();
        CurrentTestnetResult {
            rewards,
            total_base_reward,
            last_block_hash,
            last_block_number,
        }
    }

    pub fn map(&self) -> HashMap<H160, u64> {
        self.rewards.iter().cloned().collect()
    }
}

impl fmt::Display for CurrentTestnetResult {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{{")?;
        writeln!(f, "  total_base_reward: {}", self.total_base_reward)?;
        writeln!(f, "  last_block_hash: {:#}", self.last_block_hash)?;
        writeln!(f, "  last_block_number: {}", self.last_block_number)?;
        for (lock_arg, reward) in &self.rewards {
            writeln!(f, "  > lock_arg: {:#}, reward: {}", lock_arg, reward)?;
        }
        writeln!(f, "}}")
    }
}

pub fn read_last_round(
    url: &str,
    confirmations: u16,
    cache_path: Option<&str>,
) -> CurrentTestnetResult {
    let mut client = HttpRpcClient::from_uri(url);
    let (mut rewards, mut last_block_hash, mut last_block_number, mut total_base_reward) =
        match cache_path.as_ref() {
            Some(path) if Path::new(path).exists() => {
                let file = fs::File::open(path).unwrap();
                let result: CurrentTestnetResult = serde_json::from_reader(file).unwrap();
                (
                    result.map(),
                    result.last_block_hash.clone(),
                    result.last_block_number,
                    result.total_base_reward,
                )
            }
            _ => (HashMap::default(), H256::default(), 0, 0),
        };

    let mut tip_number = get_tip_block_number(&mut client);
    let current_epoch_number = client.get_current_epoch().call().unwrap().number.value();
    println!(
        "[{}] tip: {}, epoch-number: {}, target-epoch-number: {}",
        Local::now(),
        tip_number,
        current_epoch_number,
        crate::consts::LAST_EPOCH,
    );
    println!(
        "[{}] last_block_number: {}, last_block_hash: {:#}, total_base_reward: {}",
        Local::now(),
        last_block_number,
        last_block_hash,
        total_base_reward,
    );

    let mut last_epoch_number = 0;
    for number in (last_block_number + 1)..std::u64::MAX {
        tip_number = wait_until(&mut client, number, Some(tip_number), 100);

        let block = client
            .get_block_by_number(BlockNumber::from(number))
            .call()
            .unwrap()
            .0
            .unwrap();
        let block_hash = &block.header.hash;
        let epoch = EpochNumberWithFraction::from_full_value(block.header.inner.epoch.value());
        if number % 1000 == 0 {
            log::info!("block: {}", number);
        } else {
            log::debug!("block: {}", number);
        }
        let epoch_number = epoch.number();
        if epoch_number != last_epoch_number {
            println!("[{}] New epoch: {}, block number: {}", Local::now(), epoch_number, number);
            last_epoch_number = epoch_number;
        }
        if epoch_number > crate::consts::LAST_EPOCH {
            break;
        }
        last_block_hash = block_hash.clone();
        last_block_number = number;

        let cellbase: packed::Transaction = block.transactions[0].clone().inner.into();
        let lock_script = cellbase
            .into_view()
            .witnesses()
            .get(0)
            .map(|data| packed::CellbaseWitness::from_slice(&data.raw_data()).unwrap())
            .unwrap()
            .lock();
        if lock_script.code_hash() == SECP_TYPE_SCRIPT_HASH.pack()
            && lock_script.hash_type() == ScriptHashType::Type.into()
            && lock_script.args().raw_data().len() == 20
        {
            let lock_arg = H160::from_slice(&lock_script.args().raw_data()).unwrap();
            let base_reward: u64 = client
                .get_cellbase_output_capacity_details(block_hash.clone())
                .call()
                .unwrap()
                .0
                .unwrap()
                .primary
                .value();
            total_base_reward += base_reward;
            *rewards.entry(lock_arg).or_default() += base_reward;
        } else {
            log::warn!(
                "Invalid lock script: {}, block number: {}",
                lock_script,
                number
            );
        }

        if number % 2000 == 0 && tip_number - number > 300 {
            if let Some(path) = cache_path.as_ref() {
                let file = fs::File::create(path).unwrap();
                let result = CurrentTestnetResult::new(
                    rewards.clone(),
                    total_base_reward,
                    block_hash.clone(),
                    number,
                );
                serde_json::to_writer(file, &result).unwrap();
                log::info!(
                    "Current block number: {}, save cache to: {:?}",
                    number,
                    path
                );
            }
        }
    }

    println!("Finished, last block number: {}", last_block_number);
    for n in 1..=u64::from(confirmations) {
        println!("Waiting for {} confirmation", n);
        let number = last_block_number + n;
        tip_number = wait_until(&mut client, number, Some(tip_number), 100);
    }
    CurrentTestnetResult::new(
        rewards,
        total_base_reward,
        last_block_hash,
        last_block_number,
    )
}

fn get_tip_block_number(client: &mut HttpRpcClient) -> u64 {
    client.get_tip_block_number().call().unwrap().value()
}

fn wait_until(
    client: &mut HttpRpcClient,
    number: u64,
    tip_number: Option<u64>,
    interval: u64,
) -> u64 {
    let mut tip_number = tip_number.unwrap_or_else(|| get_tip_block_number(client));
    let mut check_round = 0;
    loop {
        if number > tip_number {
            if check_round % (2000 / interval) == 0 {
                log::info!("Wait for next block: {}", number);
            }
            thread::sleep(Duration::from_millis(interval));
            tip_number = get_tip_block_number(client);
            check_round += 1;
        } else {
            return tip_number;
        }
    }
}
