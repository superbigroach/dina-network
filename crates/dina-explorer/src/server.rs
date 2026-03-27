use std::sync::Arc;

use tokio::sync::RwLock;
use tower_http::cors::{Any, CorsLayer};
use tracing::info;

use dina_core::block::Block;
use dina_storage::DinaDB;

use crate::api::{explorer_routes, AppState};
use crate::indexer::ExplorerIndexer;

/// The block explorer HTTP server.
///
/// Serves REST API endpoints for browsing blocks, transactions, accounts,
/// validators, and devices on the Dina Network. Maintains an in-memory
/// transaction indexer that is updated as new blocks are committed.
pub struct ExplorerServer {
    indexer: Arc<RwLock<ExplorerIndexer>>,
    db: Arc<DinaDB>,
    bind_addr: String,
}

impl ExplorerServer {
    /// Create a new explorer server bound to the given address (e.g. "0.0.0.0:8080").
    pub fn new(bind_addr: String, db: DinaDB) -> Self {
        Self {
            indexer: Arc::new(RwLock::new(ExplorerIndexer::new())),
            db: Arc::new(db),
            bind_addr,
        }
    }

    /// Start the HTTP server. This function runs until the server is shut down.
    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let cors = CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any);

        let state = AppState {
            indexer: Arc::clone(&self.indexer),
            db: Arc::clone(&self.db),
        };

        let app = explorer_routes(state).layer(cors);

        let listener = tokio::net::TcpListener::bind(&self.bind_addr).await?;
        info!("Explorer server listening on {}", self.bind_addr);
        axum::serve(listener, app).await?;

        Ok(())
    }

    /// Index a newly committed block. Call this from the consensus layer
    /// each time a block is finalized.
    pub async fn index_block(&self, block: Block) {
        let mut indexer = self.indexer.write().await;
        indexer.index_block(&block);
    }

    /// Get a reference to the shared indexer (for external queries).
    pub fn indexer(&self) -> Arc<RwLock<ExplorerIndexer>> {
        Arc::clone(&self.indexer)
    }
}
