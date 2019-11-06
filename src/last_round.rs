use chrono::prelude::*;
use std::collections::HashMap;
use std::fmt;
use std::thread;
use std::time::Duration;

use ckb_jsonrpc_types::{BlockNumber, EpochNumber};
use ckb_sdk::HttpRpcClient;
use ckb_types::{
    core::{EpochNumberWithFraction, ScriptHashType},
    packed,
    prelude::*,
    utilities::{compact_to_difficulty, difficulty_to_compact},
    H160, H256, U256,
};
use serde::{Deserialize, Serialize};

use crate::consts::{ONE_CKB, SECP_TYPE_SCRIPT_HASH};

#[derive(Debug, Deserialize, Serialize)]
pub struct CurrentTestnetResult {
    pub rewards: Vec<(H160, u64)>,
    pub total_base_reward: u64,
    pub last_block_hash: H256,
    pub last_block_number: u64,
    // Last block's timestamp
    pub last_timestamp: u64,
    // compact target
    pub mainnet_difficulty: u32,
}

impl CurrentTestnetResult {
    pub fn new(
        rewards: HashMap<H160, u64>,
        total_base_reward: u64,
        last_block_hash: H256,
        last_block_number: u64,
        last_timestamp: u64,
        mainnet_difficulty: u32,
    ) -> Self {
        let rewards: Vec<(H160, u64)> = rewards.into_iter().collect();
        CurrentTestnetResult {
            rewards,
            total_base_reward,
            last_block_hash,
            last_block_number,
            last_timestamp,
            mainnet_difficulty,
        }
    }

    pub fn real_rewards(&self) -> Vec<(H160, u64)> {
        self.rewards
            .iter()
            .map(|(lock_arg, reward)| {
                let real_reward = (u128::from(*reward)
                    * u128::from(crate::consts::FINAL_ROUND_REWARD)
                    / u128::from(self.total_base_reward)) as u64;
                (lock_arg.clone(), real_reward / ONE_CKB * ONE_CKB)
            })
            .collect()
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
        writeln!(f, "  last_timestamp: {}", self.last_timestamp)?;
        writeln!(f, "  mainnet_difficulty: {:#x}", self.mainnet_difficulty)?;
        writeln!(f, "  rewards.len(): {}", self.rewards.len())?;
        let mut total_real_reward = 0;
        for (lock_arg, reward) in &self.rewards {
            let real_reward = (u128::from(*reward) * u128::from(crate::consts::FINAL_ROUND_REWARD)
                / u128::from(self.total_base_reward)) as u64;
            total_real_reward += real_reward;
            writeln!(
                f,
                "  > lock_arg: {:#}, reward: {}, real-reward: {}",
                lock_arg, reward, real_reward
            )?;
        }
        writeln!(f, "  total_real_reward: {}", total_real_reward)?;
        writeln!(f, "}}")
    }
}

pub fn read_last_round(url: &str, confirmations: u16) -> CurrentTestnetResult {
    let mut client = HttpRpcClient::from_uri(url);
    let mut rewards = HashMap::default();
    let mut last_block_hash = H256::default();
    let mut last_block_number = 0;
    let mut last_timestamp = 0;
    let mut total_base_reward = 0;
    let mut tip_number = get_tip_block_number(&mut client);
    let current_epoch_number = client.get_current_epoch().call().unwrap().number.value();
    println!(
        "[{}] tip: {}, epoch-number: {}, epoch-count: {}",
        Local::now(),
        tip_number,
        current_epoch_number,
        crate::consts::EPOCH_COUNT,
    );

    let mut last_epoch_number = 0;
    for number in (last_block_number + 1)..std::u64::MAX {
        tip_number = wait_until(&mut client, number + 11, Some(tip_number), 100);

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
            println!(
                "[{}] New epoch: {}, block number: {}",
                Local::now(),
                epoch_number,
                number
            );
            last_epoch_number = epoch_number;
        }
        if epoch_number >= crate::consts::EPOCH_COUNT {
            break;
        }
        last_block_hash = block_hash.clone();
        last_block_number = number;
        last_timestamp = block.header.inner.timestamp.value();

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
            let cursor_hash = client
                .get_block_hash(BlockNumber::from(number + 11))
                .call()
                .unwrap()
                .0
                .unwrap();
            let base_reward: u64 = client
                .get_cellbase_output_capacity_details(cursor_hash)
                .call()
                .unwrap()
                .0
                .unwrap()
                .primary
                .value();
            total_base_reward += base_reward;
            let lock_arg = H160::from_slice(&lock_script.args().raw_data()).unwrap();
            log::debug!(
                "lock_arg: {:#}, block-number: {:05}, base-reward: {}",
                lock_arg,
                number,
                base_reward
            );
            *rewards.entry(lock_arg).or_default() += base_reward;
        } else {
            log::error!(
                "Invalid lock script: {}, block number: {}",
                lock_script,
                number
            );
        }
    }

    println!(
        "[{}] Finished, last block number: {}",
        Local::now(),
        last_block_number
    );
    for n in 1..=u64::from(confirmations) {
        println!("[{}] Waiting for {} confirmation", Local::now(), n);
        let number = last_block_number + n;
        tip_number = wait_until(&mut client, number, Some(tip_number), 100);
    }

    rewards.retain(|lock_arg, capacity| {
        if *capacity <= 1000 * ONE_CKB {
            println!("WARN: reward not greater than 1000CKB {:#} => {}", lock_arg, capacity);
        }
        *capacity > 1000 * ONE_CKB
    });
    let mainnet_difficulty = {
        let mut total_difficulty = U256::zero();
        for offset in 0..4 {
            let epoch_number = crate::consts::EPOCH_COUNT - 1 - offset;
            let compact_target = client
                .get_epoch_by_number(EpochNumber::from(epoch_number))
                .call()
                .unwrap()
                .0
                .unwrap()
                .compact_target
                .value();
            println!(
                "[{}] Epoch {}, compact_target: {:#x} / {}",
                Local::now(),
                epoch_number,
                compact_target,
                compact_to_difficulty(compact_target),
            );
            total_difficulty += compact_to_difficulty(compact_target);
        }
        total_difficulty = total_difficulty / U256::from(4u32);
        // total_difficulty * 1.5
        total_difficulty = total_difficulty * U256::from(3u32) / U256::from(2u32);
        total_difficulty = total_difficulty * U256::from(total_base_reward) / U256::from(crate::consts::FINAL_ROUND_REWARD);
        println!("mainet difficulty: {}", total_difficulty);
        difficulty_to_compact(total_difficulty)
    };

    CurrentTestnetResult::new(
        rewards,
        total_base_reward,
        last_block_hash,
        last_block_number,
        last_timestamp,
        mainnet_difficulty,
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
