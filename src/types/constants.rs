// Tx type IDs and protocol constants.

/// Transaction type constants
pub const TX_TYPE_L2_CHANGE_PUB_KEY: u8 = 8;
pub const TX_TYPE_L2_CREATE_SUB_ACCOUNT: u8 = 9;
pub const TX_TYPE_L2_CREATE_PUBLIC_POOL: u8 = 10;
pub const TX_TYPE_L2_UPDATE_PUBLIC_POOL: u8 = 11;
pub const TX_TYPE_L2_TRANSFER: u8 = 12;
pub const TX_TYPE_L2_WITHDRAW: u8 = 13;
pub const TX_TYPE_L2_CREATE_ORDER: u8 = 14;
pub const TX_TYPE_L2_CANCEL_ORDER: u8 = 15;
pub const TX_TYPE_L2_CANCEL_ALL_ORDERS: u8 = 16;
pub const TX_TYPE_L2_MODIFY_ORDER: u8 = 17;
pub const TX_TYPE_L2_MINT_SHARES: u8 = 18;
pub const TX_TYPE_L2_BURN_SHARES: u8 = 19;
pub const TX_TYPE_L2_UPDATE_LEVERAGE: u8 = 20;
pub const TX_TYPE_L2_UPDATE_MARGIN: u8 = 29;
pub const TX_TYPE_L2_CREATE_GROUPED_ORDERS: u8 = 28;
pub const TX_TYPE_L2_STAKE_ASSETS: u8 = 35;
pub const TX_TYPE_L2_UNSTAKE_ASSETS: u8 = 36;
pub const TX_TYPE_L2_APPROVE_INTEGRATOR: u8 = 45;

// Order types
pub const ORDER_TYPE_LIMIT: u8 = 0;
pub const ORDER_TYPE_MARKET: u8 = 1;
pub const ORDER_TYPE_STOP_LOSS: u8 = 2;
pub const ORDER_TYPE_STOP_LOSS_LIMIT: u8 = 3;
pub const ORDER_TYPE_TAKE_PROFIT: u8 = 4;
pub const ORDER_TYPE_TAKE_PROFIT_LIMIT: u8 = 5;
pub const ORDER_TYPE_TWAP: u8 = 6;
pub const API_MAX_ORDER_TYPE: u8 = ORDER_TYPE_TWAP;

// Time-in-force
pub const TIF_IMMEDIATE_OR_CANCEL: u8 = 0;
pub const TIF_GOOD_TILL_TIME: u8 = 1;
pub const TIF_POST_ONLY: u8 = 2;

// Grouping types
pub const GROUPING_ONE_TRIGGERS_OTHER: u8 = 1;
pub const GROUPING_ONE_CANCELS_OTHER: u8 = 2;
pub const GROUPING_OTOCO: u8 = 3;

// Margin modes
pub const CROSS_MARGIN: u8 = 0;
pub const ISOLATED_MARGIN: u8 = 1;

// Margin update direction
pub const REMOVE_FROM_ISOLATED_MARGIN: u8 = 0;
pub const ADD_TO_ISOLATED_MARGIN: u8 = 1;

// Asset route types
pub const ASSET_ROUTE_PERPS: u8 = 0;
pub const ASSET_ROUTE_SPOT: u8 = 1;

// Value limits
pub const ONE_USDC: i64 = 1_000_000;
pub const FEE_TICK: i64 = 1_000_000;
pub const MARGIN_FRACTION_TICK: i64 = 10_000;
pub const SHARE_TICK: u16 = 10_000;

pub const MIN_ACCOUNT_INDEX: i64 = -1;
pub const MAX_ACCOUNT_INDEX: i64 = 281474976710654;
pub const MIN_SUB_ACCOUNT_INDEX: i64 = 140737488355328;
pub const MIN_API_KEY_INDEX: u8 = 0;
pub const MAX_API_KEY_INDEX: u8 = 254;

pub const MIN_MARKET_INDEX: i16 = 0;
pub const MIN_PERPS_MARKET_INDEX: i16 = 0;
pub const MAX_PERPS_MARKET_INDEX: i16 = 254;
pub const MIN_SPOT_MARKET_INDEX: i16 = 2048;
pub const MAX_SPOT_MARKET_INDEX: i16 = 4094;

pub const MIN_ASSET_INDEX: i16 = 1;
pub const MAX_ASSET_INDEX: i16 = 62;
pub const NIL_ASSET_INDEX: i16 = 0;

pub const MIN_NONCE: i64 = 0;
pub const MAX_TIMESTAMP: i64 = (1i64 << 48) - 1;

pub const MIN_CLIENT_ORDER_INDEX: i64 = 1;
pub const MAX_CLIENT_ORDER_INDEX: i64 = (1i64 << 48) - 1;
pub const NIL_CLIENT_ORDER_INDEX: i64 = 0;
pub const MIN_ORDER_INDEX: i64 = MAX_CLIENT_ORDER_INDEX + 1;
pub const MAX_ORDER_INDEX: i64 = (1i64 << 60) - 1;

pub const MIN_ORDER_BASE_AMOUNT: i64 = 1;
pub const MAX_ORDER_BASE_AMOUNT: i64 = (1i64 << 48) - 1;
pub const NIL_ORDER_BASE_AMOUNT: i64 = 0;

pub const NIL_ORDER_PRICE: u32 = 0;
pub const MIN_ORDER_PRICE: u32 = 1;
pub const MAX_ORDER_PRICE: u32 = u32::MAX;

pub const NIL_ORDER_TRIGGER_PRICE: u32 = 0;
pub const MIN_ORDER_TRIGGER_PRICE: u32 = 1;
pub const MAX_ORDER_TRIGGER_PRICE: u32 = u32::MAX;

pub const NIL_ORDER_EXPIRY: i64 = 0;
pub const MIN_ORDER_EXPIRY: i64 = 1;
pub const MAX_ORDER_EXPIRY: i64 = i64::MAX;

pub const MAX_GROUPED_ORDER_COUNT: i64 = 3;
pub const NB_ATTRIBUTES_PER_TX: usize = 4;

pub const MAX_EXCHANGE_USDC: i64 = (1i64 << 60) - 1;
pub const MIN_TRANSFER_AMOUNT: i64 = 1;
pub const MAX_TRANSFER_AMOUNT: i64 = MAX_EXCHANGE_USDC;
pub const MIN_WITHDRAWAL_AMOUNT: u64 = 1;
pub const MAX_WITHDRAWAL_AMOUNT: u64 = MAX_EXCHANGE_USDC as u64;

pub const MIN_POOL_SHARES_TO_MINT_OR_BURN: i64 = 1;
pub const MAX_POOL_SHARES_TO_MINT_OR_BURN: i64 = (1i64 << 60) - 1;
pub const MIN_INITIAL_TOTAL_SHARES: i64 = 1_000 * (ONE_USDC / 1_000);
pub const MAX_INITIAL_TOTAL_SHARES: i64 = 1_000_000_000 * (ONE_USDC / 1_000);
pub const MIN_INITIAL_TOTAL_STAKING_SHARES: i64 = 100_000 * 100_000;
pub const MAX_INITIAL_TOTAL_STAKING_SHARES: i64 = 1_000_000_000 * 100_000;
pub const MIN_STAKING_SHARES_TO_MINT_OR_BURN: i64 = 1;
pub const MAX_STAKING_SHARES_TO_MINT_OR_BURN: i64 = (1i64 << 60) - 1;

pub const NIL_INTEGRATOR_INDEX: i64 = 0;

pub const SIG_LENGTH: usize = 80;
pub const PUB_KEY_LENGTH: usize = 40;
