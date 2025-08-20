//! # Synapsed WASM P2P Platform
//! 
//! A comprehensive WebAssembly runtime and module system for P2P secure communication.
//! This crate provides WASM-based execution environments for decentralized communication,
//! real-time collaboration, cryptographic operations, and browser PWA integration.
//!
//! ## Features
//!
//! - **WASM Runtime**: High-performance WebAssembly execution environment for browsers
//! - **WebRTC Integration**: WASM modules for P2P data channel management
//! - **CRDT Operations**: Real-time collaborative editing with Yjs integration
//! - **Sync Algorithms**: Rsync-like chunking for efficient P2P synchronization
//! - **Browser Crypto**: Optimized cryptographic operations for web environments
//! - **Zero-Knowledge Proofs**: Privacy-preserving authentication and credentials
//! - **DID Operations**: Decentralized identity and key management
//! - **PWA Integration**: Service worker and IndexedDB support for offline operation
//!
//! ## Quick Start
//!
//! ```no_run
//! use synapsed_wasm::runtime::WasmRuntime;
//! use synapsed_wasm::p2p::WebRtcManager;
//! use synapsed_wasm::crdt::CrdtSyncEngine;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Initialize P2P WASM runtime
//! let runtime = WasmRuntime::new_p2p().await?;
//!
//! // Initialize WebRTC data channels
//! let webrtc = WebRtcManager::new(&runtime).await?;
//! 
//! // Setup CRDT synchronization
//! let crdt = CrdtSyncEngine::new(&runtime).await?;
//!
//! // Load a P2P communication module
//! let p2p_module = include_bytes!("../examples/wasm/p2p_comm.wasm");
//! let result = runtime.execute_p2p_module(p2p_module, &webrtc, &crdt).await?;
//!
//! println!("P2P communication result: {:?}", result);
//! # Ok(())
//! # }
//! ```
//!
//! ## Architecture
//! 
//! The P2P platform is organized into several key modules:
//! 
//! - [`runtime`]: Core WASM runtime optimized for browser execution
//! - [`modules`]: P2P-focused WASM modules and module management
//! - [`p2p`]: WebRTC data channel management and peer-to-peer networking
//! - [`crdt`]: Conflict-free replicated data types for real-time collaboration
//! - [`sync`]: Rsync-like algorithms for efficient data synchronization
//! - [`crypto`]: Browser-optimized cryptographic operations
//! - [`zkp`]: Zero-knowledge proof circuits and verification
//! - [`did`]: Decentralized identity and key management
//! - [`pwa`]: Progressive Web App integration (service workers, IndexedDB)
//! - [`storage`]: Browser storage operations and data persistence

#![deny(missing_docs)]
#![deny(unsafe_code)]
#![warn(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_errors_doc)]

// Re-export major components
pub use crate::error::{WasmError, WasmResult};
pub use crate::types::{WasmValue, ExecutionContext, ModuleInstance};

// Core modules
pub mod error;
pub mod types;
pub mod runtime;
pub mod modules;

// P2P Platform modules
#[cfg(feature = "webrtc-modules")]
pub mod p2p;

#[cfg(feature = "crdt-modules")]
pub mod crdt;

#[cfg(feature = "sync-modules")]
pub mod sync;

#[cfg(feature = "crypto-modules")]
pub mod crypto;

#[cfg(feature = "zkp-modules")]
pub mod zkp;

#[cfg(feature = "did-modules")]
pub mod did;

#[cfg(feature = "service-worker")]
pub mod pwa;

#[cfg(feature = "storage-modules")]
pub mod storage;

// Prelude for common P2P platform imports
pub mod prelude {
    //! Common imports for working with synapsed-wasm P2P platform

    pub use crate::error::{WasmError, WasmResult};
    pub use crate::types::{WasmValue, ExecutionContext, ModuleInstance};
    pub use crate::runtime::{WasmRuntime, RuntimeConfig};
    pub use crate::modules::{ModuleRegistry, WasmModule};

    // P2P specific exports
    #[cfg(feature = "webrtc-modules")]
    pub use crate::p2p::{WebRtcManager, PeerConnection};
    
    #[cfg(feature = "crdt-modules")]
    pub use crate::crdt::{CrdtSyncEngine, Document};
    
    #[cfg(feature = "sync-modules")]
    pub use crate::sync::{SyncEngine, ChunkManager};
    
    #[cfg(feature = "zkp-modules")]
    pub use crate::zkp::{ZkProofSystem, Circuit};
    
    #[cfg(feature = "did-modules")]
    pub use crate::did::{DidManager, KeyDerivation};
    
    #[cfg(feature = "service-worker")]
    pub use crate::pwa::{ServiceWorkerRuntime, IndexedDbManager};

    // Re-export async-trait for convenience
    pub use async_trait::async_trait;
    
    // Re-export wasm-bindgen for browser integration
    pub use wasm_bindgen::prelude::*;
    pub use wasm_bindgen_futures::*;
    pub use web_sys;
    pub use js_sys;
}

/// Current version of the synapsed-wasm crate
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Default WASM memory page size (64KB) - optimized for browsers
pub const WASM_PAGE_SIZE: u32 = 64 * 1024;

/// Maximum WASM memory pages for browsers (2GB to avoid OOM)
pub const MAX_WASM_PAGES: u32 = 32768;

/// Default execution timeout for P2P operations (reduced for responsiveness)
pub const DEFAULT_EXECUTION_TIMEOUT: u64 = 10;

/// Maximum WebRTC data channel message size
pub const MAX_WEBRTC_MESSAGE_SIZE: usize = 64 * 1024;

/// Default CRDT document sync interval in milliseconds
pub const DEFAULT_CRDT_SYNC_INTERVAL: u64 = 100;

/// Maximum chunk size for rsync-like operations
pub const MAX_SYNC_CHUNK_SIZE: usize = 1024 * 1024;

/// Default IndexedDB quota request in bytes (500MB)
pub const DEFAULT_INDEXEDDB_QUOTA: u64 = 500 * 1024 * 1024;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!VERSION.is_empty());
    }

    #[test]
    fn test_constants() {
        assert_eq!(WASM_PAGE_SIZE, 64 * 1024);
        assert_eq!(MAX_WASM_PAGES, 32768); // Reduced for browsers
        assert_eq!(DEFAULT_EXECUTION_TIMEOUT, 10); // Reduced for responsiveness
        assert_eq!(MAX_WEBRTC_MESSAGE_SIZE, 64 * 1024);
        assert_eq!(DEFAULT_CRDT_SYNC_INTERVAL, 100);
        assert_eq!(MAX_SYNC_CHUNK_SIZE, 1024 * 1024);
        assert_eq!(DEFAULT_INDEXEDDB_QUOTA, 500 * 1024 * 1024);
    }
}