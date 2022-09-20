/**
* Module     : main.rs
* Copyright  : 2021 Rocklabs
* License    : Apache 2.0 with LLVM Exception
* Maintainer : Rocklabs <hello@rocklabs.io>
* Stability  : Experimental
*/
use candid::{candid_method, types::number::Nat, CandidType, Deserialize};
use ic_cdk_macros::*;
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::iter::FromIterator;
use std::string::String;
use crate::token::{Operation, Token};

// Pool errors
#[derive(derive_more::Display, Debug, Clone, PartialEq)]
pub enum Error {
    #[display(fmt = "Invalid query: {}", _0)]
    InvalidQuery(String),
}

#[derive(Deserialize, CandidType, Clone, Debug)]
pub struct Pool {
    pub pool_id: Nat,
    pub token_info: BTreeMap<Nat, Token>,
    pub shared_decimals: u8,
    pub total_liquidity: Nat,
}

impl Pool {
    pub fn new(pool_id: Nat, shared_decimals: u8) -> Self {
        Pool {
            pool_id,
            token_info: BTreeMap::default(),
            shared_decimals,
            total_liquidity: Nat::from(0),
        }
    }
    pub fn get_pool_id(&self) -> Nat {
        self.pool_id.clone()
    }

    pub fn get_token_count(&self) -> Nat {
        Nat::from(self.token_info.len())
    }

    pub fn add_token(&mut self, pool_id:Nat, token: Token) -> bool {
        self.token_info.entry(pool_id).or_insert(token);
        true
    }

    pub fn remove_token(&mut self, pool_id: Nat) -> Token {
        //
        self.token_info.remove(&pool_id).unwrap()
    }

    pub fn get_token_by_src_chain_id(&self, src_chain_id: Nat) -> Option<Token> {
        //
        self.token_info.get(&src_chain_id).cloned()
    }

    pub fn get_sub_token_supply_by_src_chain_id(&self, src_chain_id: Nat) -> Nat {
        //
        let token = self.get_token_by_src_chain_id(src_chain_id).unwrap();
        token.total_supply()
    }

    pub fn total_liquidity(&self) -> Nat {
        let mut total_liquidity: Nat = Nat::from(0);
        for token in self.token_info.values() {
            total_liquidity += token.total_supply();
        }
        total_liquidity
    }
}
