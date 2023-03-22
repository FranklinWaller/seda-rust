pub fn get_oracle_contract_id() -> String {
    std::env::var("ORACLE_CONTRACT_ID").expect("Env 'ORACLE_CONTRACT_ID' does not exist")
}
