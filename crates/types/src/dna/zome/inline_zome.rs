//! A variant of Zome which is defined entirely by native, inline Rust code
//!
//! This type of Zome is only meant to be used for testing. It's designed to
//! make it easy to write a zome on-the-fly or programmatically, rather than
//! having to go through the heavy machinery of wasm compilation

use holochain_serialized_bytes::prelude::*;
use holochain_zome_types::zome::FunctionName;
use serde::de::DeserializeOwned;
use std::collections::HashMap;

use self::{api::InlineHostApi, error::InlineZomeResult};

pub mod api;
pub mod error;

/// An InlineZome, which consists
pub struct InlineZome {
    /// Since closures cannot be serialized, we include a UUID which
    /// is the only part of an InlineZome that gets serialized.
    /// This uuid becomes part of the determination of the DnaHash
    /// that it is a part of.
    /// Think of it as a stand-in for the WasmHash of a WasmZome.
    pub(super) uuid: String,

    /// The collection of closures which define this zome.
    /// These callbacks are directly called by the Ribosome.
    pub(super) callbacks: HashMap<FunctionName, InlineZomeFn>,
}

impl InlineZome {
    /// Create a new zome with the given UUID
    pub fn new<S: Into<String>>(uuid: S) -> Self {
        Self {
            uuid: uuid.into(),
            callbacks: HashMap::new(),
        }
    }

    /// Define a new zome function or callback with the given name
    pub fn callback<F, I, O>(mut self, name: &str, f: F) -> Self
    where
        F: Fn(InlineHostApi, I) -> InlineZomeResult<O> + 'static + Send + Sync,
        I: DeserializeOwned,
        O: Serialize,
    {
        let z = move |api: InlineHostApi,
                      input: SerializedBytes|
              -> InlineZomeResult<SerializedBytes> {
            let output = f(
                api,
                holochain_serialized_bytes::decode(input.bytes()).expect("TODO"),
            )?;
            Ok(SerializedBytes::from(UnsafeBytes::from(
                holochain_serialized_bytes::encode(&output).expect("TODO"),
            )))
        };
        self.callbacks.insert(name.into(), Box::new(z));
        self
    }
}

/// An inline zome function takes a Host API and an input, and produces an output.
pub type InlineZomeFn = Box<
    dyn Fn(InlineHostApi, SerializedBytes) -> InlineZomeResult<SerializedBytes>
        + 'static
        + Send
        + Sync,
>;

impl std::fmt::Debug for InlineZome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("<InlineZome {}>", self.uuid))
    }
}

impl PartialEq for InlineZome {
    fn eq(&self, other: &InlineZome) -> bool {
        self.uuid == other.uuid
    }
}

impl PartialOrd for InlineZome {
    fn partial_cmp(&self, other: &InlineZome) -> Option<std::cmp::Ordering> {
        Some(self.uuid.cmp(&other.uuid))
    }
}

impl Eq for InlineZome {}

impl Ord for InlineZome {
    fn cmp(&self, other: &InlineZome) -> std::cmp::Ordering {
        self.uuid.cmp(&other.uuid)
    }
}

impl std::hash::Hash for InlineZome {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.uuid.hash(state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use holo_hash::AnyDhtHash;
    use holochain_zome_types::prelude::GetOptions;
    use maplit::hashmap;

    #[test]
    fn can_create_inline_dna() {
        let zome = InlineZome::new("").callback("zome_fn_1", |api, a: ()| {
            let hash: AnyDhtHash = todo!();
            api.get(hash, GetOptions::default())
        });
        // let dna = InlineDna::new(hashmap! {
        //     "zome".into() => zome
        // });
    }
}