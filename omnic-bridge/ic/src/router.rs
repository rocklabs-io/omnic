use crate::error::{Error, Result};
use crate::pool::{Error as PoolError, Pool};
use crate::token::{Error as TokenError, Operation, Token};
use ic_cdk::export::candid::{CandidType, Deserialize, Int, Nat};
use ic_cdk::export::Principal;
use num_traits::cast::ToPrimitive;
use std::collections::{BTreeMap, HashSet, VecDeque};

#[derive(Default, Debug, Clone, PartialEq)]
pub struct Options {
    /// wait_optimistic
    wait_optimistic: bool,
}

impl Options {
    /// Create new default `Options` object with some modifications.
    pub fn with<F>(func: F) -> Options
    where
        F: FnOnce(&mut Options),
    {
        let mut options = Options::default();
        func(&mut options);
        options
    }
}

pub trait RouterInterfaces {
    fn add_liquidity(
        &mut self,
        src_chain: Nat,
        src_pool_id: Nat,
        to: Vec<u8>,
        amount: Nat,
    ) -> Result<bool>;
    fn remove_liquidity(
        &mut self,
        src_chain: Nat,
        src_pool_id: Nat,
        from: Vec<u8>,
        amount: Nat,
    ) -> Result<bool>;
    fn swap(
        &mut self,
        src_chain: Nat,
        src_pool_id: Nat,
        dst_chain: Nat,
        dst_pool_id: Nat,
        from: Vec<u8>,
        to: Vec<u8>,
        amount: Nat,
    ) -> Result<bool>;
}

#[derive(Deserialize, CandidType, Clone, Debug)]
pub struct Router {
    pool_ids: BTreeMap<Nat, BTreeMap<Nat, Nat>>, // src_chain -> src_pool_id -> pool_id
    pools: BTreeMap<Nat, Pool>,               // pool_id -> Pool
}

impl Router {
    pub fn new() -> Self {
        Router {
            pool_ids: BTreeMap::new(),
            pools: BTreeMap::new(),
        }
    }

    fn getPoolId(&self, src_chain: Nat, src_pool_id: Nat) -> Result<Nat> {
        //
        match self.pool_ids.get(&src_chain) {
            Some(pools) => match pools.get(&src_pool_id).cloned() {
                Some(pid) => Ok(pid),
                None => Err(Error::Pool(PoolError::InvalidQuery(format!(
                    "source chain id is not found: {}",
                    src_chain
                )))),
            },
            None => Err(Error::Pool(PoolError::InvalidQuery(format!(
                "source pool id is not found: {}",
                src_pool_id
            )))),
        }
    }

    fn getPool(&self, pool_id: Nat) -> Result<Pool> {
        //
        match self.pools.get(&pool_id).cloned() {
            Some(pool) => Ok(pool),
            None => Err(Error::Pool(PoolError::InvalidQuery(format!(
                "pool is not found: {}",
                pool_id
            )))),
        }
    }
}

impl RouterInterfaces for Router {
    fn add_liquidity(
        &mut self,
        src_chain: Nat,
        src_pool_id: Nat,
        to: Vec<u8>,
        amount: Nat,
    ) -> Result<bool> {
        let pool_id: Nat = self.getPoolId(src_chain.clone(), src_pool_id.clone())?;
        let pool: Pool = self.getPool(pool_id)?;
        let mut token = match pool.getTokenBySrcChainId(src_chain) {
            Some(token) => token,
            None => {
                return Err(Error::Token(TokenError::Invalid(format!(
                    "Errors getting pool token: {}",
                    src_pool_id
                ))))
            }
        };
        Ok(token.mint(to, amount))
    }

    fn remove_liquidity(
        &mut self,
        src_chain: Nat,
        src_pool_id: Nat,
        from: Vec<u8>,
        amount: Nat,
    ) -> Result<bool> {
        //
        let pool_id: Nat = self.getPoolId(src_chain.clone(), src_pool_id.clone())?;
        let pool: Pool = self.getPool(pool_id)?;
        let mut token = match pool.getTokenBySrcChainId(src_chain) {
            Some(token) => token,
            None => {
                return Err(Error::Token(TokenError::Invalid(format!(
                    "Errors getting pool token: {}",
                    src_pool_id
                ))))
            }
        };
        Ok(token.burn(from, amount))
    }
    fn swap(
        &mut self,
        src_chain: Nat,
        src_pool_id: Nat,
        dst_chain: Nat,
        dst_pool_id: Nat,
        from: Vec<u8>,
        to: Vec<u8>,
        amount: Nat,
    ) -> Result<bool> {
        // TODO
        Ok(false)
    }
}
