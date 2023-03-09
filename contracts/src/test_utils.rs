use bn254::{PrivateKey, PublicKey, Signature, ECDSA};
use near_contract_standards::fungible_token::metadata::{FungibleTokenMetadata, FT_METADATA_SPEC};
use near_sdk::{json_types::U128, test_utils::VMContextBuilder, Balance, VMContext, AccountId};
use rand::distributions::{Alphanumeric, DistString};

use crate::{
    consts::{DATA_IMAGE_SVG_ICON, INITIAL_SUPPLY},
    MainchainContract,
};

const TEST_DEPOSIT_AMOUNT: Balance = 9_000_000_000_000_000_000_000_000; // enough deposit to cover storage for all functions that require it

pub fn new_contract() -> MainchainContract {
    MainchainContract::new(
        "dao_near".to_string().try_into().unwrap(),
        U128(INITIAL_SUPPLY),
        FungibleTokenMetadata {
            spec:           FT_METADATA_SPEC.to_string(),
            name:           "Example NEAR fungible token".to_string(),
            symbol:         "EXAMPLE".to_string(),
            icon:           Some(DATA_IMAGE_SVG_ICON.to_string()),
            reference:      None,
            reference_hash: None,
            decimals:       24,
        },
        2,
    )
}

pub struct TestAccount {
    pub account_id: AccountId,
    pub ed25519_public_key: near_sdk::PublicKey,
    pub bn254_private_key: PrivateKey,
    pub bn254_public_key: PublicKey,
}

pub fn bob() -> TestAccount {
    // let random_hex_string = hex::encode(Alphanumeric.sample_string(&mut rand::thread_rng(), 22));
    // let ed25519_public_key_string = "ed25519:".to_string() + &random_hex_string;
    // println!("ed25519_public_key_string: {}", ed25519_public_key_string);
    // let ed25519_public_key: near_sdk::PublicKey = ed25519_public_key_string.parse().unwrap();
    let ed25519_public_key: near_sdk::PublicKey = "ed25519:6E8sCci9badyRkXb3JoRpBj5p8C6Tw41ELDZoiihKEtp".parse()
    .unwrap();

    let random_hex_string_2 = hex::encode(Alphanumeric.sample_string(&mut rand::thread_rng(), 32));
    let bn254_private_key_bytes = hex::decode(random_hex_string_2).unwrap();
    let bn254_private_key = PrivateKey::try_from(bn254_private_key_bytes.as_ref()).unwrap();
    let bn254_public_key = PublicKey::from_private_key(&bn254_private_key);
    
    return TestAccount {
        account_id: "bob_near".to_string().try_into().unwrap(),
        ed25519_public_key: ed25519_public_key,
        bn254_private_key: bn254_private_key,
        bn254_public_key: bn254_public_key,
    }
}

pub fn get_context_view() -> VMContext {
    VMContextBuilder::new().is_view(true).build()
}
pub fn get_context(signer_account_id: String) -> VMContext {
    VMContextBuilder::new()
        .signer_account_id(signer_account_id.parse().unwrap())
        .predecessor_account_id(signer_account_id.parse().unwrap())
        .is_view(false)
        .build()
}
pub fn get_context_for_post_signed_batch(signer_account_id: String) -> VMContext {
    VMContextBuilder::new()
        .signer_account_id(signer_account_id.parse().unwrap())
        .is_view(false)
        .attached_deposit(TEST_DEPOSIT_AMOUNT)
        .block_index(100000000)
        .build()
}
pub fn get_context_with_deposit(test_account: TestAccount) -> VMContext {
    VMContextBuilder::new()
        .signer_account_id(test_account.account_id)
        .signer_account_pk(test_account.ed25519_public_key)
        .is_view(false)
        .attached_deposit(TEST_DEPOSIT_AMOUNT) // required for post_data_request()
        .build()
}
pub fn get_context_for_ft_transfer(signer_account_id: String) -> VMContext {
    VMContextBuilder::new()
        .signer_account_id(signer_account_id.parse().unwrap())
        .predecessor_account_id(signer_account_id.parse().unwrap())
        .is_view(false)
        .attached_deposit(1)
        .build()
}
pub fn get_context_at_block(block_index: u64) -> VMContext {
    VMContextBuilder::new().block_index(block_index).is_view(true).build()
}
pub fn get_context_with_deposit_at_block(signer_account_id: String, block_index: u64) -> VMContext {
    VMContextBuilder::new()
        .signer_account_id(signer_account_id.parse().unwrap())
        .is_view(false)
        .attached_deposit(TEST_DEPOSIT_AMOUNT) // required for post_data_request()
        .block_index(block_index)
        .build()
}

pub fn generate_bn254_key() -> (PublicKey, PrivateKey) {
    let random_hex_string = hex::encode(Alphanumeric.sample_string(&mut rand::thread_rng(), 32));

    let private_key = PrivateKey::try_from(random_hex_string).unwrap();
    let public_key = PublicKey::from_private_key(&private_key);

    (public_key, private_key)
}

pub fn bn254_sign(private_key: &PrivateKey, message: &[u8]) -> Signature {
    ECDSA::sign(message, private_key).unwrap()
}
