use std::collections::{BTreeMap, HashSet, VecDeque};
use num_traits::cast::ToPrimitive;
use ic_cdk::export::candid::{CandidType, Deserialize, Nat, Int};
use ic_cdk::export::Principal;
use crate::pool::Pool;
use create::error::{Result, Error};

pub trait Methods {
    fn process_message(&self, msg: &mut Message) -> Result<bool>;
    fn send_message(&self, msg: &mut Message) -> Result<bool>;

}

#[derive(Deserialize, CandidType, Clone, Debug)]
pub struct Bridge {
    pools: BTreeMap<Nat, Pool>,
}

impl Bridge {
    fn new() -> Bridge {
        Bridge { 
            pools: Default::default()
        }
    }

    fn getPool(&self, poolId: Nat) -> Option<Pool> {
        //
        self.pools.get(&poolId)
    }
}

impl Message for Bridge {
    fn process_message(&mut self, msg: &mut Message) -> Result<bool> {
        //
    }

    fn send_message(&mut self, msg: &mut Message) -> Result<bool> {
        //
    }
}

