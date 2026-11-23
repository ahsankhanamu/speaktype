//! Speech-to-text providers: an explicit provider config, Keychain secrets,
//! and the providers with their retry, timeout and cancellation policy.

mod config;
mod deepgram;
mod error;
mod local;
mod openai;
mod provider;
mod retry;
mod secrets;

pub use config::{CloudService, ProviderConfig};
pub use error::SttError;
pub use provider::{
    provider_for, Decode, RemoteModel, Segment, SttProvider, TranscribeRequest, Transcript,
};
pub use retry::{timeout_for, RetryPolicy};
#[cfg(feature = "keychain")]
pub use secrets::KeychainStore;
pub use secrets::{MemoryStore, SecretError, SecretStore};
