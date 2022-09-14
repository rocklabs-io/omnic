
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
    tokenInfo: HashSet<Token<T>>,
    totalLiquidity: Nat,
}

impl<T: Operation> Pool<T> {
    pub fn new(poolId: Nat) -> Self {
        Pool {
            poolId,
            tokenInfo: Default::default(),
            totalLiquidity: 0,
        }
    }

    pub fn addToken(&mut self, token: Token<T>) {
        self.tokenInfo.insert(token)
    }
    pub fn removeToken(&mut self, token: Token<T>) -> bool {
        //
        true
    }
    pub fn getTokenByName(&self, name: &str) -> Option<Token<T>> {
        //
        None
    }
}
