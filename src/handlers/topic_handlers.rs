use crate::models::request::EmailRequest;
use crate::models::result::EmailResult;
use crate::state::AppState;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;

/// retrieve_topic_handler
/// Topic retrieval handler
/// Retrieve the request and result counts for each topic
pub async fn retrieve_topic_handler(
    State(state): State<AppState>,
    Path(topic_id): Path<String>,
) -> impl IntoResponse {
    if topic_id.is_empty() {
        return (StatusCode::BAD_REQUEST, "topicId is required").into_response();
    }
    // Query request counts
    let request_counts =
        EmailRequest::get_request_counts_by_topic_id(&state.db_pool, &topic_id).await;
    // Query result counts
    let result_counts = EmailResult::get_result_counts_by_topic_id(&state.db_pool, &topic_id).await;
    let response = serde_json::json!({
        "request_counts": request_counts.expect("Failed to retrieve request counts"),
        "result_counts": result_counts.expect("Failed to retrieve result counts"),
    });
    (StatusCode::OK, Json(response)).into_response()
}

/// stop_topic_handler
/// Topic stop sending handler
/// Stop sending requests for the specified topic
pub async fn stop_topic_handler(
    State(state): State<AppState>,
    Path(topic_id): Path<String>,
) -> impl IntoResponse {
    // Process stop sending requests
    match EmailRequest::stop_topic(&state.db_pool, &topic_id).await {
        Ok(_) => (StatusCode::OK, "OK").into_response(),
        Err(e) => {
            eprintln!("Failed to stop topic: {:?}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to stop topic").into_response()
        }
    }
}
