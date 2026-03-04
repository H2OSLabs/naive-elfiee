//! MCP Transport Layer
//!
//! Provides SSE server for MCP protocol communication.
//!
//! Each connection gets a per-connection ElfieeMcpServer instance.
//! Identity is established via `elfiee_auth` tool (not GUI active_editor).
//!
//! Notification fan-out: subscribes to `state_changed_tx` broadcast channel
//! and pushes `resources/list_changed` notifications to each connected client.

use super::ElfieeMcpServer;
use crate::state::AppState;
use rmcp::transport::sse_server::{SseServer, SseServerConfig};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio_util::sync::CancellationToken;

/// MCP SSE Server default port
pub const MCP_PORT: u16 = 47200;

/// Start the MCP SSE Server.
///
/// Used by `elf serve` to start the headless MCP server.
/// Each connected client gets per-connection identity via `elfiee_auth`.
///
/// After a client connects, a background task subscribes to `state_changed_tx`
/// and sends `resources/list_changed` MCP notifications to the client's Peer.
pub async fn start_mcp_server(app_state: Arc<AppState>, port: u16) -> Result<(), String> {
    let ct = CancellationToken::new();
    let config = SseServerConfig {
        bind: SocketAddr::from(([127, 0, 0, 1], port)),
        sse_path: "/sse".to_string(),
        post_path: "/message".to_string(),
        ct: ct.clone(),
        sse_keep_alive: Some(Duration::from_secs(30)),
    };

    let mut sse_server = SseServer::serve_with_config(config)
        .await
        .map_err(|e| format!("MCP: Failed to bind on port {}: {}", port, e))?;

    println!("MCP Server listening on http://127.0.0.1:{}", port);
    println!("  GET  /sse      - SSE connection");
    println!("  POST /message  - MCP messages");

    tokio::spawn(async move {
        use rmcp::service::ServiceExt;

        while let Some(transport) = sse_server.next_transport().await {
            let app_state = app_state.clone();
            let ct = ct.child_token();

            println!("MCP: Client connected");

            tokio::spawn(async move {
                let result = async {
                    let service = ElfieeMcpServer::new(app_state.clone());
                    let server = service
                        .serve_with_ct(transport, ct)
                        .await
                        .map_err(std::io::Error::other)?;

                    // Fan-out: subscribe to state changes and notify this client
                    let peer = server.peer().clone();
                    let mut state_rx = app_state.state_changed_tx.subscribe();
                    tokio::spawn(async move {
                        while let Ok(_file_id) = state_rx.recv().await {
                            // Notify client that resources have changed
                            // Client can then re-fetch resources to get updated state
                            if peer.notify_resource_list_changed().await.is_err() {
                                break; // Client disconnected
                            }
                        }
                    });

                    server.waiting().await?;
                    tokio::io::Result::Ok(())
                }
                .await;

                if let Err(e) = result {
                    eprintln!("MCP: Connection error: {}", e);
                }

                println!("MCP: Client disconnected");
            });
        }
    });

    Ok(())
}
