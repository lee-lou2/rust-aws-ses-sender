use crate::models::request::{EmailMessageStatus, EmailRequest};
use crate::state::AppState;
use axum::extract::State;
use axum::response::IntoResponse;
use axum::Json;
use futures::stream::{self, StreamExt};
use reqwest::StatusCode;
use serde::Deserialize;
use std::sync::Arc;

/// Message
/// Message used in a creation request
#[derive(Deserialize)]
pub struct Message {
    pub topic_id: Option<String>,
    pub emails: Vec<String>,
    pub subject: String,
    pub content: String,
}

/// CreateMessageRequest
/// Message creation request
#[derive(Deserialize)]
pub struct CreateMessageRequest {
    pub messages: Vec<Message>,
    pub scheduled_at: Option<String>,
}

/// create_message_handler
/// Message creation handler
/// Creates messages and processes them concurrently using a thread pool.
/// Immediately sends if no scheduled send time is provided; otherwise, schedules the send.
pub async fn create_message_handler(
    State(state): State<AppState>,
    Json(payload): Json<CreateMessageRequest>,
) -> impl IntoResponse {
    let start = std::time::Instant::now();
    let scheduled_at = payload.scheduled_at;
    // Immediately send if no scheduled send time is provided
    let mut status = EmailMessageStatus::Created as i32;
    if let Some(scheduled_at) = scheduled_at.clone() {
        if scheduled_at.is_empty() {
            status = EmailMessageStatus::Processed as i32;
        }
    } else {
        status = EmailMessageStatus::Processed as i32;
    }

    // Process concurrently using a pool of 100 threads
    let tasks = stream::iter(payload.messages.into_iter().flat_map(|message| {
        let scheduled_at = scheduled_at.clone();
        let request = EmailRequest {
            id: None,
            topic_id: Some(message.topic_id.unwrap_or_default()),
            error: None,
            email: String::from(""),
            subject: message.subject,
            content: message.content,
            scheduled_at: scheduled_at.clone(),
            status,
            message_id: None,
        };
        let db_pool = Arc::new(state.db_pool.clone());
        let tx = Arc::new(state.tx.clone());
        message.emails.into_iter().map(move |email| {
            let mut request = request.clone();
            request.email = email.clone();
            let db_pool = Arc::clone(&db_pool);
            let tx = Arc::clone(&tx);
            async move {
                let request = request.save(&db_pool).await;
                if status == EmailMessageStatus::Processed as i32 {
                    if let Err(e) = tx.send(request).await {
                        eprintln!("Error sending data to channel: {:?}", e);
                    }
                }
            }
        })
    }));
    tasks.buffer_unordered(100).for_each(|_| async {}).await;
    let duration = start.elapsed();
    (StatusCode::OK, format!("Processed in {:?}", duration)).into_response()
}
