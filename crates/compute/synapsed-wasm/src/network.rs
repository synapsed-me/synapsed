//! Network WASM operations

#[cfg(feature = "network-modules")]
use synapsed_net::*;

use crate::error::{WasmError, WasmResult};
use crate::types::{HostFunction, WasmValue};
use std::collections::HashMap;
use std::sync::Arc;

/// Create network host functions
pub fn create_network_host_functions() -> HashMap<String, HostFunction> {
    let mut functions = HashMap::new();

    // HTTP request
    functions.insert(
        "http_get".to_string(),
        Arc::new(|args| {
            if let Some(WasmValue::String(url)) = args.get(0) {
                tracing::info!("HTTP GET request to: {}", url);
                // In a real implementation, this would make actual HTTP requests
                // For security, this should be restricted to allowed domains
                Ok(vec![WasmValue::String("HTTP response".to_string())])
            } else {
                Err(WasmError::Network("URL required".to_string()))
            }
        }) as HostFunction,
    );

    // Network info
    functions.insert(
        "get_peer_count".to_string(),
        Arc::new(|_args| {
            // Return mock peer count
            Ok(vec![WasmValue::I32(5)])
        }) as HostFunction,
    );

    functions
}

/// WASM-compatible network operations
pub struct WasmNetwork {
    /// Connected peers
    peers: Vec<String>,
    /// Network status
    connected: bool,
}

impl WasmNetwork {
    /// Create new network interface
    pub fn new() -> Self {
        Self {
            peers: Vec::new(),
            connected: false,
        }
    }

    /// Connect to network
    pub fn connect(&mut self) -> WasmResult<()> {
        self.connected = true;
        tracing::info!("Connected to network");
        Ok(())
    }

    /// Disconnect from network
    pub fn disconnect(&mut self) -> WasmResult<()> {
        self.connected = false;
        self.peers.clear();
        tracing::info!("Disconnected from network");
        Ok(())
    }

    /// Add peer
    pub fn add_peer(&mut self, peer: String) -> WasmResult<()> {
        if !self.connected {
            return Err(WasmError::Network("Not connected to network".to_string()));
        }
        
        if !self.peers.contains(&peer) {
            self.peers.push(peer.clone());
            tracing::info!("Added peer: {}", peer);
        }
        Ok(())
    }

    /// Remove peer
    pub fn remove_peer(&mut self, peer: &str) -> WasmResult<bool> {
        if let Some(pos) = self.peers.iter().position(|p| p == peer) {
            self.peers.remove(pos);
            tracing::info!("Removed peer: {}", peer);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Get peer count
    pub fn peer_count(&self) -> usize {
        self.peers.len()
    }

    /// Get all peers
    pub fn peers(&self) -> Vec<String> {
        self.peers.clone()
    }

    /// Check if connected
    pub fn is_connected(&self) -> bool {
        self.connected
    }

    /// Send message to peer (mock implementation)
    pub fn send_message(&self, peer: &str, message: &str) -> WasmResult<()> {
        if !self.connected {
            return Err(WasmError::Network("Not connected to network".to_string()));
        }
        
        if !self.peers.contains(&peer.to_string()) {
            return Err(WasmError::Network("Peer not found".to_string()));
        }

        tracing::info!("Sending message to {}: {}", peer, message);
        Ok(())
    }
}

impl Default for WasmNetwork {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_network_operations() {
        let mut network = WasmNetwork::new();
        
        assert!(!network.is_connected());
        assert_eq!(network.peer_count(), 0);

        // Connect to network
        network.connect().unwrap();
        assert!(network.is_connected());

        // Add peers
        network.add_peer("peer1".to_string()).unwrap();
        network.add_peer("peer2".to_string()).unwrap();
        assert_eq!(network.peer_count(), 2);

        // Send message
        network.send_message("peer1", "hello").unwrap();

        // Remove peer
        assert!(network.remove_peer("peer1").unwrap());
        assert!(!network.remove_peer("peer1").unwrap()); // Already removed
        assert_eq!(network.peer_count(), 1);

        // Disconnect
        network.disconnect().unwrap();
        assert!(!network.is_connected());
        assert_eq!(network.peer_count(), 0);
    }

    #[test]
    fn test_network_errors() {
        let mut network = WasmNetwork::new();
        
        // Try to add peer without connecting
        let result = network.add_peer("peer1".to_string());
        assert!(result.is_err());

        // Try to send message without connecting
        let result = network.send_message("peer1", "hello");
        assert!(result.is_err());
    }

    #[test]
    fn test_network_host_functions() {
        let functions = create_network_host_functions();
        assert!(functions.contains_key("http_get"));
        assert!(functions.contains_key("get_peer_count"));
    }
}