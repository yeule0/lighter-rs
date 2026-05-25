pub mod schnorr;

pub use schnorr::{sign_hashed_message, verify, public_key_from_secret, Signature};
