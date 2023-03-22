pub use bn254::{PrivateKey as Bn254PrivateKey, PublicKey as Bn254PublicKey, Signature as Bn254Signature};

use super::raw;

pub fn bn254_verify(message: &[u8], signature: &Bn254Signature, public_key: &Bn254PublicKey) -> bool {
    let message_len = message.len() as i64;
    let signature_bytes = signature.to_compressed().expect("Signature should be valid");
    let signature_length = signature_bytes.len() as i64;
    let public_key_bytes = public_key.to_compressed().expect("Public Key should be valid");
    let public_key_length = public_key_bytes.len() as i64;

    let result = unsafe {
        raw::bn254_verify(
            message.as_ptr(),
            message_len,
            signature_bytes.as_ptr(),
            signature_length,
            public_key_bytes.as_ptr(),
            public_key_length,
        )
    };

    match result {
        0 => false,
        1 => true,
        _ => panic!("Bn254 verify returned invalid bool in u8: {}", result),
    }
}

pub fn bn254_sign(message: &[u8]) -> Bn254Signature {
    let message_len = message.len() as i64;

    // Compressed Signatures in G1 have a length of 33 bytes
    let value_len = 33;
    let mut result_data_ptr = vec![0; value_len as usize];

    unsafe { raw::bn254_sign(message.as_ptr(), message_len, result_data_ptr.as_mut_ptr(), value_len) };

    Bn254Signature::from_compressed(result_data_ptr).expect("Signature should be valid")
}
