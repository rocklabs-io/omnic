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
    // pub lps: BTreeMap<String, u128>, // liquidity providers
}

impl Pool {
    pub fn new(
        src_chain: u32,
        src_pool_id: u32,
        pool_address: String,
        shared_decimals: u8,
        local_decimals: u8,
        convert_rate: u128,
        token: Token
    ) -> Self {
        Pool {
            src_chain,
            src_pool_id,
            pool_address,
            shared_decimals,
            local_decimals,
            convert_rate,
            token,
            liquidity: 0u128,
        }
    }

    // utils 
    // pub fn amount_evm_to_amount_ic(&self, amount_evm: Nat, native_token_decimal: u8, wrapper_token_decimal: u8) -> Nat {
    //     amount_evm * (u128::pow(10, wrapper_token_decimal as u32)) / (u128::pow(10, native_token_decimal as u32))
    // }

    // pub fn amount_ic_to_amount_evm(&self, amount_ic: Nat, native_token_decimal: u8, wrapper_token_decimal: u8) -> Nat {
    //     amount_ic * (u128::pow(10, native_token_decimal as u32)) / (u128::pow(10, wrapper_token_decimal as u32))
    // }
}
