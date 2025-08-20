//! WebRTC P2P communication WASM modules
//!
//! This module provides WebAssembly-compatible WebRTC data channel management
//! for peer-to-peer communication in browsers. It includes connection management,
//! signaling, and data channel operations optimized for WASM execution.

use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    RtcPeerConnection, RtcDataChannel, RtcSessionDescription, RtcIceCandidate,
    RtcConfiguration, RtcDataChannelInit, MessageEvent,
};
use js_sys::{Object, Promise, JSON};

use crate::error::{WasmError, WasmResult};
use crate::types::{HostFunction, WasmValue, ExecutionContext};
use crate::{MAX_WEBRTC_MESSAGE_SIZE};

/// WebRTC connection manager for P2P communication
pub struct WebRtcManager {
    /// Active peer connections
    connections: HashMap<String, PeerConnection>,
    /// WebRTC configuration
    config: RtcConfiguration,
    /// Connection statistics
    stats: ConnectionStats,
}

impl WebRtcManager {
    /// Create a new WebRTC manager
    pub fn new() -> WasmResult<Self> {
        let config = RtcConfiguration::new();
        // Add STUN servers for NAT traversal
        let ice_servers = js_sys::Array::new();
        let stun_server = Object::new();
        js_sys::Reflect::set(&stun_server, &"urls".into(), &"stun:stun.l.google.com:19302".into())
            .map_err(|_| WasmError::Network("Failed to configure STUN server".to_string()))?;
        ice_servers.push(&stun_server);
        config.set_ice_servers(&ice_servers);

        Ok(Self {
            connections: HashMap::new(),
            config,
            stats: ConnectionStats::default(),
        })
    }

    /// Create a new peer connection
    pub async fn create_connection(&mut self, peer_id: String) -> WasmResult<String> {
        let rtc_connection = RtcPeerConnection::new_with_configuration(&self.config)
            .map_err(|_| WasmError::Network("Failed to create peer connection".to_string()))?;

        let connection = PeerConnection::new(peer_id.clone(), rtc_connection).await?;
        let connection_id = connection.id.clone();
        
        self.connections.insert(peer_id, connection);
        self.stats.connections_created += 1;

        tracing::info!(peer_id = %peer_id, connection_id = %connection_id, "WebRTC connection created");
        Ok(connection_id)
    }

    /// Create data channel for peer
    pub async fn create_data_channel(
        &mut self,
        peer_id: &str,
        channel_name: String,
        options: DataChannelOptions,
    ) -> WasmResult<String> {
        let connection = self.connections.get_mut(peer_id)
            .ok_or_else(|| WasmError::Network(format!("Peer {} not found", peer_id)))?;

        let channel_id = connection.create_data_channel(channel_name, options).await?;
        self.stats.data_channels_created += 1;

        tracing::debug!(peer_id = %peer_id, channel_id = %channel_id, "Data channel created");
        Ok(channel_id)
    }

    /// Send data through data channel
    pub async fn send_data(
        &mut self,
        peer_id: &str,
        channel_id: &str,
        data: &[u8],
    ) -> WasmResult<()> {
        if data.len() > MAX_WEBRTC_MESSAGE_SIZE {
            return Err(WasmError::Network(format!(
                "Data size {} exceeds maximum {} bytes",
                data.len(),
                MAX_WEBRTC_MESSAGE_SIZE
            )));
        }

        let connection = self.connections.get_mut(peer_id)
            .ok_or_else(|| WasmError::Network(format!("Peer {} not found", peer_id)))?;

        connection.send_data(channel_id, data).await?;
        self.stats.messages_sent += 1;
        self.stats.bytes_sent += data.len() as u64;

        Ok(())
    }

    /// Receive data from data channel (non-blocking)
    pub async fn receive_data(&mut self, peer_id: &str, channel_id: &str) -> WasmResult<Option<Vec<u8>>> {
        let connection = self.connections.get_mut(peer_id)
            .ok_or_else(|| WasmError::Network(format!("Peer {} not found", peer_id)))?;

        if let Some(data) = connection.receive_data(channel_id).await? {
            self.stats.messages_received += 1;
            self.stats.bytes_received += data.len() as u64;
            Ok(Some(data))
        } else {
            Ok(None)
        }
    }

    /// Get connection statistics
    pub fn get_stats(&self) -> &ConnectionStats {
        &self.stats
    }

    /// Close connection with peer
    pub async fn close_connection(&mut self, peer_id: &str) -> WasmResult<()> {
        if let Some(mut connection) = self.connections.remove(peer_id) {
            connection.close().await?;
            self.stats.connections_closed += 1;
            tracing::info!(peer_id = %peer_id, "WebRTC connection closed");
        }
        Ok(())
    }

    /// List active connections
    pub fn list_connections(&self) -> Vec<String> {
        self.connections.keys().cloned().collect()
    }
}

/// Individual peer connection wrapper
pub struct PeerConnection {
    /// Unique connection ID
    pub id: String,
    /// Peer ID
    pub peer_id: String,
    /// WebRTC peer connection
    rtc_connection: RtcPeerConnection,
    /// Data channels
    data_channels: HashMap<String, DataChannelWrapper>,
    /// Connection state
    state: ConnectionState,
}

impl PeerConnection {
    /// Create a new peer connection
    pub async fn new(peer_id: String, rtc_connection: RtcPeerConnection) -> WasmResult<Self> {
        let id = format!("conn_{}", uuid::Uuid::new_v4());
        
        Ok(Self {
            id,
            peer_id,
            rtc_connection,
            data_channels: HashMap::new(),
            state: ConnectionState::New,
        })
    }

    /// Create a data channel
    pub async fn create_data_channel(
        &mut self,
        name: String,
        options: DataChannelOptions,
    ) -> WasmResult<String> {
        let mut init = RtcDataChannelInit::new();
        init.ordered(options.ordered);
        if let Some(max_retransmits) = options.max_retransmits {
            init.max_retransmits(max_retransmits);
        }

        let rtc_channel = self.rtc_connection.create_data_channel_with_data_channel_dict(&name, &init);
        
        let channel_id = format!("ch_{}_{}", self.peer_id, uuid::Uuid::new_v4());
        let wrapper = DataChannelWrapper::new(channel_id.clone(), rtc_channel)?;
        
        self.data_channels.insert(channel_id.clone(), wrapper);
        Ok(channel_id)
    }

    /// Send data through data channel
    pub async fn send_data(&mut self, channel_id: &str, data: &[u8]) -> WasmResult<()> {
        let channel = self.data_channels.get_mut(channel_id)
            .ok_or_else(|| WasmError::Network(format!("Data channel {} not found", channel_id)))?;

        channel.send(data).await
    }

    /// Receive data from data channel
    pub async fn receive_data(&mut self, channel_id: &str) -> WasmResult<Option<Vec<u8>>> {
        let channel = self.data_channels.get_mut(channel_id)
            .ok_or_else(|| WasmError::Network(format!("Data channel {} not found", channel_id)))?;

        channel.receive().await
    }

    /// Close the connection
    pub async fn close(&mut self) -> WasmResult<()> {
        // Close all data channels first
        for (_, channel) in self.data_channels.drain() {
            channel.close();
        }
        
        // Close the peer connection
        self.rtc_connection.close();
        self.state = ConnectionState::Closed;
        
        Ok(())
    }

    /// Get connection state
    pub fn state(&self) -> &ConnectionState {
        &self.state
    }
}

/// Data channel wrapper for WASM integration
pub struct DataChannelWrapper {
    /// Channel ID
    pub id: String,
    /// WebRTC data channel
    channel: RtcDataChannel,
    /// Received message queue
    message_queue: Vec<Vec<u8>>,
}

impl DataChannelWrapper {
    /// Create a new data channel wrapper
    pub fn new(id: String, channel: RtcDataChannel) -> WasmResult<Self> {
        // Set up message handler
        let message_queue = Vec::new();
        
        Ok(Self {
            id,
            channel,
            message_queue,
        })
    }

    /// Send data through the channel
    pub async fn send(&self, data: &[u8]) -> WasmResult<()> {
        let array = js_sys::Uint8Array::new_with_length(data.len() as u32);
        array.copy_from(data);
        
        self.channel.send_with_u8_array(&array)
            .map_err(|_| WasmError::Network("Failed to send data".to_string()))?;
        
        Ok(())
    }

    /// Receive data from the channel (non-blocking)
    pub async fn receive(&mut self) -> WasmResult<Option<Vec<u8>>> {
        if let Some(data) = self.message_queue.pop() {
            Ok(Some(data))
        } else {
            Ok(None)
        }
    }

    /// Close the data channel
    pub fn close(self) {
        self.channel.close();
    }
}

/// Data channel configuration options
#[derive(Debug, Clone)]
pub struct DataChannelOptions {
    /// Ordered delivery
    pub ordered: bool,
    /// Maximum retransmits (None for unlimited)
    pub max_retransmits: Option<u16>,
    /// Protocol
    pub protocol: Option<String>,
}

impl Default for DataChannelOptions {
    fn default() -> Self {
        Self {
            ordered: true,
            max_retransmits: None,
            protocol: None,
        }
    }
}

/// Connection state enumeration
#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionState {
    New,
    Connecting,
    Connected,
    Disconnected,
    Failed,
    Closed,
}

/// WebRTC connection statistics
#[derive(Debug, Clone, Default)]
pub struct ConnectionStats {
    /// Number of connections created
    pub connections_created: u64,
    /// Number of connections closed
    pub connections_closed: u64,
    /// Number of data channels created
    pub data_channels_created: u64,
    /// Number of messages sent
    pub messages_sent: u64,
    /// Number of messages received
    pub messages_received: u64,
    /// Total bytes sent
    pub bytes_sent: u64,
    /// Total bytes received
    pub bytes_received: u64,
}

/// Create WebRTC host functions for WASM modules
pub fn create_webrtc_host_functions() -> HashMap<String, HostFunction> {
    let mut functions = HashMap::new();

    // Create peer connection
    functions.insert(
        "webrtc_create_connection".to_string(),
        Arc::new(|args| {
            if let Some(WasmValue::String(peer_id)) = args.get(0) {
                // In a real implementation, this would interact with the WebRTC manager
                tracing::info!("Creating WebRTC connection for peer: {}", peer_id);
                Ok(vec![WasmValue::String(format!("conn_{}", peer_id))])
            } else {
                Err(WasmError::Network("Peer ID required".to_string()))
            }
        }) as HostFunction,
    );

    // Send data through WebRTC
    functions.insert(
        "webrtc_send_data".to_string(),
        Arc::new(|args| {
            match (args.get(0), args.get(1), args.get(2)) {
                (Some(WasmValue::String(peer_id)), 
                 Some(WasmValue::String(channel_id)), 
                 Some(WasmValue::Bytes(data))) => {
                    tracing::debug!(
                        peer_id = %peer_id,
                        channel_id = %channel_id,
                        data_len = data.len(),
                        "Sending WebRTC data"
                    );
                    Ok(vec![WasmValue::I32(1)]) // Success
                }
                _ => Err(WasmError::Network("Invalid arguments for WebRTC send".to_string()))
            }
        }) as HostFunction,
    );

    // Receive data from WebRTC
    functions.insert(
        "webrtc_receive_data".to_string(),
        Arc::new(|args| {
            match (args.get(0), args.get(1)) {
                (Some(WasmValue::String(peer_id)), Some(WasmValue::String(channel_id))) => {
                    tracing::debug!(
                        peer_id = %peer_id,
                        channel_id = %channel_id,
                        "Receiving WebRTC data"
                    );
                    // Return mock data for now
                    Ok(vec![WasmValue::Bytes(b"mock_received_data".to_vec())])
                }
                _ => Err(WasmError::Network("Invalid arguments for WebRTC receive".to_string()))
            }
        }) as HostFunction,
    );

    functions
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_channel_options() {
        let options = DataChannelOptions::default();
        assert!(options.ordered);
        assert!(options.max_retransmits.is_none());
        assert!(options.protocol.is_none());
    }

    #[test]
    fn test_connection_stats() {
        let mut stats = ConnectionStats::default();
        assert_eq!(stats.connections_created, 0);
        assert_eq!(stats.messages_sent, 0);
        
        stats.connections_created = 1;
        stats.messages_sent = 5;
        stats.bytes_sent = 1024;
        
        assert_eq!(stats.connections_created, 1);
        assert_eq!(stats.messages_sent, 5);
        assert_eq!(stats.bytes_sent, 1024);
    }

    #[test]
    fn test_connection_state() {
        let state = ConnectionState::New;
        assert_eq!(state, ConnectionState::New);
        assert_ne!(state, ConnectionState::Connected);
    }
}