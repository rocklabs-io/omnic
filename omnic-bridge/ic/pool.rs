
/**
* Module     : main.rs
* Copyright  : 2021 Rocklabs
* License    : Apache 2.0 with LLVM Exception
* Maintainer : Rocklabs <hello@rocklabs.io>
* Stability  : Experimental
*/
use candid::{candid_method, CandidType, Deserialize, types::number::Nat};
use ic_kit::{ic , Principal};
use ic_cdk_macros::*;
use std::collections::{HashMap, BTreeSet, VecDeque};
use std::iter::FromIterator;
use std::string::String;
use crate::token::{Token, Operations};
use crate::error::{Error, ErrorKind};

pub type Result<T> = std::result::Result<T, Error>;


#[derive(Deserialize, CandidType, Clone, Debug)]
pub struct Pool<T: Operation> {
    poolId: Nat,
    tokenInfo: HashMap<Nat, Token<T>>,
    sharedDecimals: u8,
    totalLiquidity: Nat,
}

impl<T: Operation> Pool<T> {
    pub fn new(poolId: Nat, sharedDecimals: u8) -> Self {
        Pool {
            poolId,
            tokenInfo: HashMap::default(),
            sharedDecimals,
            totalLiquidity: 0,
        }
    }

    pub fn addToken(&mut self, token: Token<T>) {
        self.tokenInfo.insert(token)
    }
    pub fn removeToken(&mut self, token: Token<T>) -> Token<T> {
        //
        self.tokenInfo.remove(&token).unwrap()
    }
    pub fn getTokenBySrcChainId(&self, srcChainId: Nat) -> Option<Token<T>> {
        //
        self.tokenInfo.get(&srcChainId)
    }

    pub fn getSubTokenSupplyBySrcChainId(&self, srcChainId: Nat) -> Nat {
        //
        self.tokenInfo.get(&srcChainId).unwrap_or(Nat::from(0))
    }

    pub fn totalLiquidity(&self) -> Nat {
        let totalLiquidity: Nat = 0.to();
        for token in self.tokenInfo.values() {
            totalLiquidity += self.token.total_supply;
        }
        totalLiquidity
    }
}
