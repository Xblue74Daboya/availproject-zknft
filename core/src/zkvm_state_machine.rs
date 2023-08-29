use crate::{
    errors::Error,
    traits::{Leaf, StateTransition, TxHasher},
    types::{BatchHeader, ShaHasher, StateUpdate, TransactionReceipt, AggregatedBatch},
};
use sparse_merkle_tree::{traits::Value, H256};
use std::marker::PhantomData;

pub struct ZKStateMachine<V, T, S: StateTransition<V, T>> {
    stf: S,
    phantom_v: PhantomData<V>,
    phantom_t: PhantomData<T>,
}

impl<
        V: Leaf<H256> + Value + Clone + std::fmt::Debug,
        T: TxHasher + Clone,
        S: StateTransition<V, T>,
    > ZKStateMachine<V, T, S>
{
    pub fn new(stf: S) -> Self {
        ZKStateMachine {
            stf,
            phantom_v: PhantomData,
            phantom_t: PhantomData,
        }
    }

    pub fn execute_tx(
        &self,
        params: T,
        state_update: StateUpdate<V>,
        batch_number: u64,
        aggregated_proof: AggregatedBatch
    ) -> Result<BatchHeader, Error> {
        match state_update.pre_state_with_proof.1.verify::<ShaHasher>(
            &state_update.pre_state_root,
            state_update
                .pre_state_with_proof
                .0
                .iter()
                .map(|v| (v.get_key(), v.to_h256()))
                .collect(),
        ) {
            Ok(true) => (),
            //TODO - Change to invalid proof error
            Ok(false) => {
                println!("Merkle verification failed.");
                return Err(Error::Unknown)
            },
            Err(e) => {
                println!("{:?}", e);
                return Err(Error::Unknown)
            },
        };
        println!("pre state checked.");

        let call_result: Result<(Vec<V>, TransactionReceipt), Error> = self
            .stf
            .execute_tx(state_update.pre_state_with_proof.0.clone(), params.clone(), aggregated_proof);

        let (updated_set, receipt): (Vec<V>, TransactionReceipt) = match call_result {
            Ok(v) => v,
            Err(e) => return Err(e),
        };
        println!("executed.");

        match state_update.post_state_with_proof.clone().1.verify::<ShaHasher>(
            &state_update.post_state_root,
            updated_set
                .iter()
                .map(|x| (x.get_key(), x.to_h256()))
                .collect(),
        ) {
            Ok(true) => (),
            //TODO - Change to invalid proof error
            Ok(false) => {
                println!("Merkle verification failed., {:?}, ", &state_update.post_state_with_proof);
                return Err(Error::Unknown)
            },
            Err(e) => {
                println!("{:?}", e);
                return Err(Error::Unknown)
            },
        };

        Ok(BatchHeader {
            pre_state_root: state_update.pre_state_root,
            state_root: state_update.post_state_root,
            transactions_root: params.to_h256(),
            receipts_root: receipt.to_h256(),
            //Note: Batch can be removed from public parameters.
            batch_number,
        })
    }
}