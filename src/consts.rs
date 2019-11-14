use ckb_types::{h256, H256};

pub const SECP_TYPE_SCRIPT_HASH: H256 =
    h256!("0x9bd7e06f3ecf4be0f2fcd2188b23f1b9fcc88e5d4b65a8637b17723bbda3cce8");
pub const MULTISIG_TYPE_SCRIPT_HASH: H256 =
    h256!("0x5c5069eb0857efc65e1bca0c07df34c31663b3622fd3876c876320fc9634e2a8");

pub const ONE_CKB: u64 = 100_000_000;
pub const EPOCH_REWARD2: u64 = (200_0000 / 80) * ONE_CKB;
pub const EPOCH_REWARD3: u64 = (300_0000 / 80) * ONE_CKB;
pub const FINAL_ROUND_REWARD: u64 = 18_000_000 * ONE_CKB;

pub const INIT_TOTAL: u64 = 33_600_000_000 * ONE_CKB;
// 0.5%
pub const INIT_TESTNET: u64 = INIT_TOTAL / 200;
// 2%
pub const INIT_FOUNDATION: u64 = INIT_TOTAL / 50;
// 25%
pub const INIT_BURN: u64 = INIT_TOTAL / 4;

pub const TESTNET_FOUNDATION_ADDR: &str = "ckb1qyqy6mtud5sgctjwgg6gydd0ea05mr339lnslczzrc";

pub const FOUNDATION_RESERVE_ADDR: &str = "ckb1qyqyz340d4nhgtx2s75mp5wnavrsu7j5fcwqktprrp";
pub const FOUNDATION_RESERVE_LOCK_TIME: &str = "2020-07-01";
pub const FOUNDATION_RESERVE: u64 = 670_735_037 * ONE_CKB;
// UTC time
pub const SINCE_BEGIN: &str = "2019-11-16T06:00:00+00:00";
pub const DEFAULT_TIME_SUFFIX: &str = "T00:00:00+00:00";

