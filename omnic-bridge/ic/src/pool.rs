use candid::{types::number::Nat, CandidType, Deserialize};
use std::collections::BTreeMap;
use std::string::String;
use crate::token::{Operation, Token};

// Pool errors
#[derive(derive_more::Display, Debug, Clone, PartialEq)]
pub enum Error {
    #[display(fmt = "Invalid: {}", _0)]
    Invalid(String),
}

#[derive(Deserialize, CandidType, Clone, Debug)]
pub struct Pool {
    pub src_chain: u32,
    pub src_pool_id: u32,
    pub pool_address: String,
    pub shared_decimals: u8,
    pub local_decimals: u8,
    pub convert_rate: u128,
    pub token: Token,
    pub liquidity: u128, // liquidity left in that pool
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
        token: Token
    ) -> Self {
        Pool {
            src_chain,
            src_pool_id,
            pool_address,
            shared_decimals,
            local_decimals,
            convert_rate: u128::pow(10, local_decimals.into()) / u128::pow(10, shared_decimals.into()),
            token,
            liquidity: 0u128,
        }
    }

    pub fn add_liquidity(&mut self, amount_ld: u128) {
        self.liquidity += amount_ld;
    }

    pub fn enough_liquidity(&self, amount_ld: u128) -> bool {
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
