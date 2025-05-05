mod app;
mod config;
mod handlers;
mod middlewares;
mod models;
mod services;
mod state;
mod tests;

use services::receiver::{receive_post_send_message, receive_send_message};
use services::scheduler::schedule_pre_send_message;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), sqlx::Error> {
    let envs = config::get_environments();

    // Sentry Initialization
    let sentry_dsn = &envs.sentry_dsn;
    let _guard = sentry::init((
        sentry_dsn.as_str(),
        sentry::ClientOptions {
            release: sentry::release_name!(),
            ..Default::default()
        },
    ));

    // Initialize DB
    let db_pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(10)
        .connect("sqlite://sqlite3.db")
        .await
        .expect("Failed to create pool");

    // Initialize channels
    let (tx_send, rx_send) = tokio::sync::mpsc::channel(10000);
    let (tx_post_send, rx_post_send) = tokio::sync::mpsc::channel(1000);
    let cloned_tx_send = tx_send.clone();

    // Preprocess email sending
    tokio::spawn({
        let db_pool = db_pool.clone();
        async move {
            schedule_pre_send_message(&tx_send, db_pool).await;
        }
    });

    // Email sending
    let arc_rx_send = Arc::new(Mutex::new(rx_send));
    tokio::spawn({
        let cloned_arc_rx_send = Arc::clone(&arc_rx_send);
        async move {
            receive_send_message(&cloned_arc_rx_send, &tx_post_send).await;
        }
    });

    // Postprocess email sending
    let arc_rx_post_send = Arc::new(Mutex::new(rx_post_send));
    tokio::spawn({
        let cloned_arc_rx_post_send = Arc::clone(&arc_rx_post_send);
        let db_pool = db_pool.clone();
        async move {
            receive_post_send_message(&cloned_arc_rx_post_send, db_pool).await;
        }
    });

    let state = state::AppState::new(db_pool, cloned_tx_send);

    // Initialize logger
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .with_target(true)
                .with_level(true),
        )
        .with(tracing_subscriber::filter::LevelFilter::DEBUG)
        .init();

    let app = app::app(state).await?;

    // Start the server
    let port = &envs.server_port;
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    println!("Server running on http://0.0.0.0:{}", port);
    axum::serve(listener, app).await?;
    Ok(())
}
