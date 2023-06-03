use sp_core::{crypto::AccountId32, Pair};
use subxt::{dynamic::Value, ext::scale_value::At, OnlineClient, PolkadotConfig};

use super::Result;

/// Verify a message and its corresponding against a public key;
///
/// * signature: UIntArray with 64 element
/// * message: Arbitrary length UIntArray
/// * pubkey: UIntArray with 32 element
pub fn verify(signature: &[u8], message: &[u8], pubkey: &[u8]) -> bool {
    sp_core::sr25519::Pair::verify_weak(signature, message, pubkey)
}

pub async fn balance(
    api: &OnlineClient<PolkadotConfig>,
    account: AccountId32,
) -> Result<Option<u128>> {
    // Build a storage query to access account information.
    let storage_query =
        subxt::dynamic::storage("System", "Account", vec![Value::from_bytes(account)]);

    let result = api
        .storage()
        .at_latest()
        .await?
        .fetch(&storage_query)
        .await?;

    result
        .ok_or_else(|| "Failed to decode balance".into())
        .and_then(|v| {
            v.to_value()
                .map_err(|err| err.into())
                .map(|v| v.at("data").at("free").and_then(|v| v.as_u128()))
        })
}
