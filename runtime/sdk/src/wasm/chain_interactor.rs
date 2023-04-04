use crate::{Chain, ChainCallAction, ChainViewAction, PromiseStatus};

pub fn chain_view<C: ToString, M: ToString>(
    chain: Chain,
    contract_id: C,
    method_name: M,
    args: Vec<u8>,
) -> PromiseStatus {
    let chain_view_action = ChainViewAction {
        chain,
        contract_id: contract_id.to_string(),
        method_name: method_name.to_string(),
        args,
    };

    let action = serde_json::to_string(&chain_view_action).unwrap();
    let result_length = unsafe { super::raw::chain_view(action.as_ptr(), action.len() as u32) };
    let mut result_data_ptr = vec![0; result_length as usize];

    unsafe {
        super::raw::call_result_write(result_data_ptr.as_mut_ptr(), result_length);
    }

    serde_json::from_slice(&result_data_ptr).expect("Could not deserialize chain_view")
}

pub fn chain_call<C: ToString, M: ToString>(
    chain: Chain,
    contract_id: C,
    method_name: M,
    args: Vec<u8>,
    deposit: u128,
) -> PromiseStatus {
    let chain_call_action = ChainCallAction {
        chain,
        contract_id: contract_id.to_string(),
        method_name: method_name.to_string(),
        args,
        deposit,
    };

    let action = serde_json::to_string(&chain_call_action).unwrap();
    let result_length = unsafe { super::raw::chain_call(action.as_ptr(), action.len() as u32) };
    let mut result_data_ptr = vec![0; result_length as usize];

    unsafe {
        super::raw::call_result_write(result_data_ptr.as_mut_ptr(), result_length);
    }

    serde_json::from_slice(&result_data_ptr).expect("Could not deserialize chain_call")
}
