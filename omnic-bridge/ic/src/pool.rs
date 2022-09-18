/**
* Module     : main.rs
* Copyright  : 2021 Rocklabs
* License    : Apache 2.0 with LLVM Exception
* Maintainer : Rocklabs <hello@rocklabs.io>
* Stability  : Experimental
*/
use candid::{candid_method, types::number::Nat, CandidType, Deserialize};
use ic_cdk_macros::*;
use std::collections::{BTreeSet, HashMap, VecDeque};
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
pub struct Pool<T: Operation> {
    pool_id: Nat,
    token_info: HashMap<Nat, T>,
    shared_decimals: u8,
    total_liquidity: Nat,
}

impl<T: Operation> Pool<T> {
    pub fn new(pool_id: Nat, shared_decimals: u8) -> Self {
        Pool {
            pool_id,
            token_info: HashMap::default(),
            shared_decimals,
            total_liquidity: Nat::from(0),
        }
    }

    pub fn addToken(&mut self, pool_id:Nat, token: T) -> bool {
        self.token_info.entry(pool_id).or_insert(token);
        true
    }

    pub fn removeToken(&mut self, pool_id: Nat) -> T {
        //
        self.token_info.remove(&pool_id).unwrap()
    }

    pub fn getTokenBySrcChainId(&self, srcChainId: Nat) -> Option<T> {
        //
        self.token_info.get(&srcChainId).cloned()
    }

    pub fn getSubTokenSupplyBySrcChainId(&self, srcChainId: Nat) -> Nat {
        //
        let token = self.getTokenBySrcChainId(srcChainId).unwrap();
        token.totalSupply()
    }

    pub fn totalLiquidity(&self) -> Nat {
        let mut totalLiquidity: Nat = Nat::from(0);
        for token in self.token_info.values() {
            totalLiquidity += token.totalSupply();
        }
        totalLiquidity
    }
}
