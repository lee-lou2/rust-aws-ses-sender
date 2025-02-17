use crate::models::request::EmailRequest;
use sqlx::SqlitePool;
use tokio::sync::mpsc;

/// AppState
/// Application state
#[derive(Clone)]
pub struct AppState {
    pub db_pool: SqlitePool,
    pub tx: mpsc::Sender<EmailRequest>,
}

impl AppState {
    /// new
    /// Creates an application state
    pub fn new(db_pool: SqlitePool, tx: mpsc::Sender<EmailRequest>) -> Self {
        Self {
            db_pool,
            tx: tx.clone(),
        }
    }
}
