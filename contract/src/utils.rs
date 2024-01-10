use near_sdk::Timestamp;
use uint::construct_uint;

pub const DURATION_30DAYS_IN_SEC: u32 = 60 * 60 * 24 * 30;

construct_uint! {
    /// 256-bit unsigned integer.
    pub struct U256(4);
}

pub fn nano_to_sec(nano: Timestamp) -> u32 {
    (nano / 1_000_000_000) as u32
}
