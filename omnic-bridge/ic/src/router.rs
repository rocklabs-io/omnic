use crate::error::{Error, Result};
use crate::pool::{Error as PoolError, Pool};
use crate::token::{Error as TokenError, Operation};
use ic_cdk::export::candid::{CandidType, Deserialize, Nat};
use std::collections::BTreeMap;

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
    type AccountItem;
    fn add_liquidity(
        &mut self,
        src_chain_id: u32,
        src_pool_id: Nat,
        to: Self::AccountItem,
        amount: Nat,
    ) -> Result<bool>;
    fn remove_liquidity(
        &mut self,
        src_chain_id: u32,
        src_pool_id: Nat,
        from: Self::AccountItem,
        amount: Nat,
    ) -> Result<bool>;
    fn swap(
        &mut self,
        src_chain_id: u32,
        src_pool_id: Nat,
        dst_chain_id: u32,
        dst_pool_id: Nat,
        amount: Nat,
    ) -> Result<bool>;
}

#[derive(Deserialize, CandidType, Clone, Debug)]
pub struct Router<T>
where
    T: std::fmt::Debug + Clone,
    T: CandidType + std::cmp::Ord,
{
    bridge_addr: BTreeMap<u32, T>,
    pool_ids: BTreeMap<u32, BTreeMap<Nat, Nat>>, // src_chain_id -> src_pool_id -> pool_id
    pools: BTreeMap<Nat, Pool<T>>,               // pool_id -> Pool
    pool_symbols: BTreeMap<String, Nat>,         // pool symbol -> pool_id
    // TODO
    weights: BTreeMap<Nat, f32>, // allocate weights for different chain
}

impl<T> Router<T>
where
    T: std::fmt::Debug + Clone,
    T: CandidType + std::cmp::Ord,
{
    pub fn new() -> Self {
        Router {
            bridge_addr: BTreeMap::new(),
            pool_ids: BTreeMap::new(),
            pools: BTreeMap::new(),
            pool_symbols: BTreeMap::new(),
            weights: BTreeMap::new()
        }
    }

    pub fn get_bridge_addr(&self, chain_id: u32) -> Result<T> {
        self.bridge_addr
            .get(&chain_id)
            .ok_or(Error::Pool(PoolError::Invalid(format!(
                "chain id is not found: {}",
                chain_id
            ))))
            .cloned()
    }

    pub fn is_bridge_exist(&self, chain_id: u32) -> bool {
        self.bridge_addr.contains_key(&chain_id)
    }

    pub fn add_bridge_addr(&mut self, chain_id: u32, bridge_addr: T) {
        self.bridge_addr.entry(chain_id).or_insert(bridge_addr);
    }

    pub fn remove_bridge_addr(&mut self, chain_id: u32) -> Result<T> {
        self.bridge_addr
            .remove(&chain_id)
            .ok_or(Error::Pool(PoolError::Invalid(format!(
                "chain id is not found: {}",
                chain_id
            ))))
    }

    pub fn get_pools_length(&self) -> Nat {
        Nat::from(self.pools.len())
    }

    pub fn get_pool_id(&self, src_chain_id: u32, src_pool_id: Nat) -> Result<Nat> {
        //
        match self.pool_ids.get(&src_chain_id) {
            Some(pools) => match pools.get(&src_pool_id).cloned() {
                Some(pid) => Ok(pid.clone()),
                None => Err(Error::Pool(PoolError::Invalid(format!(
                    "source chain id is not found: {}",
                    src_pool_id
                )))),
            },
            None => Err(Error::Pool(PoolError::Invalid(format!(
                "source chain id is not found: {}",
                src_chain_id
            )))),
        }
    }

    pub fn contain_pool(&self, src_chain_id: u32, src_pool_id: Nat) -> Result<bool> {
        let pool_id: Nat = self.get_pool_id(src_chain_id, src_pool_id)?;
        Ok(self.get_pool(pool_id).is_ok())
    }

    pub fn get_pool_id_by_symbol(&self, symbol: &str) -> Result<Nat> {
        //
        self.pool_symbols
            .get(symbol)
            .ok_or(Error::Pool(PoolError::Invalid(format!(
                "{} pool has not created yet. Please check the input!",
                symbol.to_owned()
            )))).cloned()
    }

    pub fn contain_pool_by_symbol(&self, symbol: &str) -> Result<bool> {
        Ok(self.pool_symbols.contains_key(symbol))
    }

    pub fn get_pool(&self, pool_id: Nat) -> Result<Pool<T>> {
        //
        match self.pools.get(&pool_id).cloned() {
            Some(pool) => Ok(pool),
            None => Err(Error::Pool(PoolError::Invalid(format!(
                "pool is not found: {}",
                pool_id
            )))),
        }
    }

    pub fn add_pool(&mut self, pool: Pool<T>) -> Result<bool> {
        let pool_id: Nat = self.get_pools_length();
        self.pools.entry(pool_id.clone()).or_insert(pool);
        Ok(true)
    }

    pub fn add_pool_id(&mut self, src_chain: u32, src_pool_id: Nat) -> Result<bool> {
        let pool_id: Nat = self.get_pools_length() - 1;
        self.pool_ids
            .entry(src_chain)
            .or_default()
            .entry(src_pool_id)
            .or_insert(pool_id);
        Ok(true)
    }

    pub fn add_pool_symbol(&mut self, symbol: String) -> Result<bool> {
        let pool_id: Nat = self.get_pools_length() - 1;
        self.pool_symbols.entry(symbol.clone()).or_insert(pool_id);
        Ok(true)
    }

    pub fn remove_pool(&mut self, pool_id: &Nat) -> Result<Pool<T>> {
        self.pools
            .remove(pool_id)
            .ok_or(Error::Pool(PoolError::Invalid(format!(
                "remove pool failed!"
            ))))
    }

    pub fn remove_pool_id(&mut self, src_chain: &u32, src_pool_id: &Nat) -> Result<Nat> {
        self.pool_ids.remove(src_chain).map_or(
            Err(Error::Pool(PoolError::Invalid(format!(
                "remove pool failed!"
            )))),
            |mut p| {
                p.remove(src_pool_id)
                    .ok_or(Error::Pool(PoolError::Invalid(format!("no pool!"))))
            },
        )
    }
}

impl<T> RouterInterfaces for Router<T>
where
    T: std::fmt::Debug + Clone,
    T: CandidType + std::cmp::Ord,
{
    type AccountItem = T;
    fn add_liquidity(
        &mut self,
        src_chain_id: u32,
        src_pool_id: Nat,
        to: Self::AccountItem,
        amount: Nat,
    ) -> Result<bool> {
        let pool_id: Nat = self.get_pool_id(src_chain_id.clone(), src_pool_id.clone())?;
        let pool = self.get_pool(pool_id)?;
        let mut token = match pool.get_token_by_chain_id(src_chain_id) {
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
        src_chain_id: u32,
        src_pool_id: Nat,
        from: Self::AccountItem,
        amount: Nat,
    ) -> Result<bool> {
        //
        let pool_id: Nat = self.get_pool_id(src_chain_id.clone(), src_pool_id.clone())?;
        let pool = self.get_pool(pool_id)?;
        let mut token = match pool.get_token_by_chain_id(src_chain_id) {
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
        src_chain_id: u32,
        src_pool_id: Nat,
        dst_chain_id: u32,
        dst_pool_id: Nat,
        amount: Nat,
    ) -> Result<bool> {
        //
        let pool_id1: Nat = self.get_pool_id(src_chain_id.clone(), src_pool_id.clone())?;
        let pool1 = self.get_pool(pool_id1.clone())?;

        let pool_id2: Nat = self.get_pool_id(dst_chain_id.clone(), dst_pool_id.clone())?;
        let pool2 = self.get_pool(pool_id2.clone())?;

        // No AMM now, so pool1 == pool2
        assert_eq!(pool_id1, pool_id2);

        let mut token1 = pool1.get_token_by_chain_id(src_chain_id).unwrap();
        let mut token2 = pool2.get_token_by_chain_id(dst_chain_id).unwrap();

        //1. check if dst_chain has enough token to transfer
        if amount >= token2.get_total_supply() {
            return Err(Error::Token(TokenError::Invalid(format!(
                "dst chain does not have enough tokens to transfer"
            ))));
        }
        // 2. check if src & dst chain bridge address exists

        let src_bridge_addr = self.get_bridge_addr(src_chain_id).unwrap();
        let dst_bridge_addr = self.get_bridge_addr(dst_chain_id).unwrap();

        //3. src_chain_id pool mint token to src bridge address
        if !token1.mint(src_bridge_addr, amount.clone()) {
            return Err(Error::Pool(PoolError::Invalid(format!(
                "mint src token failed!"
            ))));
        }

        //4. dst_chain_id pool burn token
        if !token2.burn(dst_bridge_addr, amount.clone()) {
            return Err(Error::Pool(PoolError::Invalid(format!(
                "burn dst token failed!"
            ))));
        }

        Ok(true)
    }
}
