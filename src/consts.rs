use ckb_types::{h256, H256};

pub const SECP_TYPE_SCRIPT_HASH: H256 =
    h256!("0x9bd7e06f3ecf4be0f2fcd2188b23f1b9fcc88e5d4b65a8637b17723bbda3cce8");
pub const LAST_EPOCH: u64 = 90;
pub const ONE_CKB: u64 = 100_000_000;
pub const EPOCH_REWARD2: u64 = (200_0000 / 80) * ONE_CKB;
pub const EPOCH_REWARD3: u64 = (300_0000 / 80) * ONE_CKB;
