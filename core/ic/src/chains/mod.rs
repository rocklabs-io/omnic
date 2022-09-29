pub mod evm;

pub use evm::*;

pub enum ChainType {
    EVM,
    Cosmos,
    Solana,
}