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
    pub pool_id: u32,
    pub tokens: BTreeMap<u32, Token>, // chain_id -> token
}

impl Pool {
    pub fn new(pool_id: u32, tokens: BTreeMap<u32, Token>) -> Self {
        Pool {
            pool_id,
            tokens
        }
    }

    pub fn tokens_count(&self) -> u32 {
        self.tokens.len() as u32
    }

    pub fn add_token(&mut self, chain_id: u32, token: Token) {
        self.tokens.entry(chain_id).or_insert(token);
    }

    pub fn remove_token(&mut self, chain_id: u32) {
        self.tokens.remove(&chain_id);
    }

    pub fn token_by_chain_id(&self, chain_id: u32) -> Token {
        match self.tokens.get(&chain_id) {
            Some(t) => t.cloned(),
            None => unreachable!(),
        }
    }

    pub fn has_token(&self, chain_id: u32) -> bool {
        self.tokens.contains_key(&chain_id)
    }

    pub fn token_supply_by_chain_id(&self, chain_id: u32) -> u128 {
        self.token_by_chain_id(chain_id).total_supply()
    }

    pub fn total_liquidity(&self) -> u128 {
        let mut total_liquidity: u128 = 0;
        for token in self.tokens.values() {
            total_liquidity += token.total_supply();
        }
        total_liquidity
    }

    // utils 
    pub fn amount_evm_to_amount_ic(&self, amount_evm: Nat, native_token_decimal: u8, wrapper_token_decimal: u8) -> Nat {
        amount_evm * (u128::pow(10, wrapper_token_decimal as u32)) / (u128::pow(10, native_token_decimal as u32))
    }

    pub fn amount_ic_to_amount_evm(&self, amount_ic: Nat, native_token_decimal: u8, wrapper_token_decimal: u8) -> Nat {
        amount_ic * (u128::pow(10, native_token_decimal as u32)) / (u128::pow(10, wrapper_token_decimal as u32))
    }
}
