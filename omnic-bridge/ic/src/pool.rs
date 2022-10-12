use crate::token::Token;
/**
 * @brief  keep sync with pool from all chains
 */
use candid::{CandidType, Deserialize};
use std::string::String;

// Pool errors
#[derive(derive_more::Display, Debug, Clone, PartialEq)]
pub enum Error {
    #[display(fmt = "Invalid: {}", _0)]
    Invalid(String),
}

#[derive(Deserialize, CandidType, Clone, Debug)]
pub struct Pool {
    src_chain: u32,
    src_pool_id: u32,
    pool_address: String,
    shared_decimals: u8,
    local_decimals: u8,
    convert_rate: u128,
    token: Token,
    liquidity: u128, // liquidity left in that pool
    // pub lps: BTreeMap<String, u128>, // liquidity providers, ignore for now
}

// local_decimals >= shared_decimals
impl Pool {
    pub fn new(
        src_chain: u32,
        src_pool_id: u32,
        pool_address: String,
        shared_decimals: u8,
        local_decimals: u8,
        token: Token,
    ) -> Self {
        Pool {
            src_chain,
            src_pool_id,
            pool_address,
            shared_decimals,
            local_decimals,
            convert_rate: u128::pow(10, local_decimals.into())
                / u128::pow(10, shared_decimals.into()),
            token,
            liquidity: 0u128,
        }
    }

    pub fn src_chain(&self) -> u32 {
        self.src_chain
    }

    pub fn src_pool_id(&self) -> u32 {
        self.src_pool_id
    }

    pub fn pool_address(&self) -> String {
        self.pool_address.clone()
    }

    pub fn shared_decimals(&self) -> u8 {
        self.shared_decimals
    }

    pub fn local_decimals(&self) -> u8 {
        self.local_decimals
    }

    pub fn convert_rate(&self) -> u128 {
        self.convert_rate
    }

    pub fn token(&self) -> Token {
        self.token.clone()
    }

    pub fn liquidity(&self) -> u128 {
        self.liquidity
    }
}

impl Pool {
    pub fn add_liquidity(&mut self, amount_ld: u128) {
        self.liquidity += amount_ld;
    }

    pub fn enough_liquidity(&self, amount_ld: u128) -> bool{
        self.liquidity >= amount_ld
    }

    pub fn remove_liquidity(&mut self, amount_ld: u128) {
        self.liquidity -= amount_ld;
    }

    pub fn amount_ld(&self, amount_sd: u128) -> u128 {
        amount_sd * self.convert_rate
    }

    pub fn amount_sd(&self, amount_ld: u128) -> u128 {
        amount_ld / self.convert_rate
    }
}
