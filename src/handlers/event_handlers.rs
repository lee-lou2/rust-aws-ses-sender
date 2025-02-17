use crate::models::request::EmailRequest;
use crate::models::result::EmailResult;
use crate::state::AppState;
use axum::extract::Request;
use axum::{
    extract::{Json, Query, State},
    http::header::HeaderValue,
    http::HeaderMap,
    http::StatusCode,
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::{error, info};

/// MAX_BODY_SIZE
/// Maximum body size for incoming requests
const MAX_BODY_SIZE: usize = 1024 * 1024;

/// OpenMessageQueryParams
/// Query parameters for handling open events
#[derive(Deserialize)]
pub struct OpenMessageQueryParams {
    pub request_id: Option<String>,
}

/// GetSentCountQueryParams
/// Query parameters for retrieving recently sent email count
#[derive(Deserialize)]
pub struct GetSentCountQueryParams {
    pub hours: Option<i32>,
}

/// GetSentCountResponse
/// Response for retrieving recently sent email count
#[derive(Deserialize, Serialize)]
pub struct GetSentCountResponse {
    pub count: i32,
}

/// CreateEventRequest
/// Request for creating events
#[derive(Deserialize, Serialize, Debug)]
#[serde(untagged)]
enum CreateEventRequest {
    SubscriptionConfirmation {
        #[serde(rename = "SubscribeURL")]
        subscribe_url: String,
    },
    Notification {
        #[serde(rename = "Message")]
        message: String,
        #[serde(rename = "MessageId")]
        message_id: String,
    },
    Other(Value),
}

/// CreateEventNotification
/// Notification for creating events
#[derive(Deserialize, Debug)]
struct CreateEventNotification {
    #[serde(rename = "notificationType")]
    event_type: String,

    #[serde(flatten)]
    other_fields: Value,
}

/// open_message_handler
/// Handler for processing open events
/// Checks if the email has been opened and saves the result
/// Returns a 1x1 transparent image
pub async fn open_message_handler(
    State(state): State<AppState>,
    Query(query): Query<OpenMessageQueryParams>,
) -> impl IntoResponse {
    if let Some(request_id) = query.request_id {
        // Save open result
        let request_id = request_id.parse();
        match request_id {
            Ok(id) => {
                // Save data if request_id is valid
                let result = EmailResult {
                    id: None,
                    status: "Open".to_string(),
                    request_id: id,
                    raw: None,
                };
                match result.save(&state.db_pool).await {
                    Err(e) => {
                        eprintln!("Failed to save open event: {:?}", e);
                    }
                    _ => { /* Do nothing */ }
                }
            }
            Err(e) => {
                eprintln!("Failed to parse request_id: {:?}", e);
            }
        };
    }
    // Return a 1x1 transparent image
    let png_bytes: &[u8] = &[
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1F,
        0x15, 0xC4, 0x89, 0x00, 0x00, 0x00, 0x0A, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0x00,
        0x00, 0x00, 0x02, 0x00, 0x01, 0xE2, 0x26, 0x05, 0x9B, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45,
        0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
    ];
    let mut headers = HeaderMap::new();
    headers.insert("Content-Type", HeaderValue::from_static("image/png"));
    (StatusCode::OK, headers, png_bytes).into_response()
}

/// get_sent_count_handler
/// Handler for retrieving recently sent email count
/// Queries and returns the count of recently sent emails
pub async fn get_sent_count_handler(
    State(state): State<AppState>,
    Query(query): Query<GetSentCountQueryParams>,
) -> impl IntoResponse {
    let hours = query.hours.unwrap_or(24);
    match EmailRequest::sent_count(&state.db_pool, hours).await {
        Ok(count) => (StatusCode::OK, Json(GetSentCountResponse { count })).into_response(),
        Err(e) => {
            eprintln!("Failed to retrieve sent count: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to retrieve sent count",
            )
                .into_response()
        }
    }
}

/// create_event_handler
/// Event creation handler
/// Processes events received from AWS SNS and saves the result
pub async fn create_event_handler(
    State(state): State<AppState>,
    request: Request,
) -> impl IntoResponse {
    // --- 1. Header Check ---
    if !request
        .headers()
        .get("x-amz-sns-message-type")
        .and_then(|v| v.to_str().ok())
        .map_or(false, |msg_type| {
            msg_type == "Notification" || msg_type == "SubscriptionConfirmation"
        })
    {
        error!("Invalid x-amz-sns-message-type header");
        return (StatusCode::BAD_REQUEST, "Invalid SNS Message Type").into_response();
    }

    // --- 2. Body Extraction (with size limit) ---
    let body_bytes = match axum::body::to_bytes(request.into_body(), MAX_BODY_SIZE).await {
        Ok(bytes) => bytes,
        Err(e) => {
            error!(
                "Failed to read request body (size limit exceeded or other error): {:?}",
                e
            );
            return (StatusCode::BAD_REQUEST, "Failed to read body").into_response();
        }
    };

    // --- 3. Parse SNS Message ---
    let sns_message: CreateEventRequest = match serde_json::from_slice(&body_bytes) {
        Ok(msg) => msg,
        Err(e) => {
            error!(
                "Failed to parse SNS message: {:?}, Raw body: {}",
                e,
                String::from_utf8_lossy(&body_bytes)
            );
            return (StatusCode::BAD_REQUEST, "Failed to parse SNS message").into_response();
        }
    };

    // --- 4. Handle Message Types ---
    match sns_message {
        CreateEventRequest::SubscriptionConfirmation { subscribe_url } => {
            info!(
                "Subscription confirmation required. Visiting: {}",
                subscribe_url
            );
            (StatusCode::OK, "Subscription confirmation required").into_response()
        }
        CreateEventRequest::Notification {
            message,
            message_id,
        } => {
            // --- 4a. Parse SES Notification directly ---
            match serde_json::from_str::<CreateEventNotification>(&message) {
                Ok(ses_notification) => {
                    // --- 4b. Extract SES message_id from other_fields ---
                    let ses_message_id = ses_notification
                        .other_fields
                        .get("mail")
                        .and_then(|mail| mail.get("messageId"))
                        .and_then(|id| id.as_str()) // Convert to &str
                        .map(String::from); // Convert to String

                    // --- 4c. Handle Event Types and Database Operations ---
                    match ses_message_id {
                        Some(ses_msg_id) => {
                            match EmailRequest::get_request_id_by_message_id(
                                &state.db_pool,
                                &ses_msg_id,
                            )
                            .await
                            {
                                Ok(request_id) => {
                                    let result = EmailResult {
                                        id: None,
                                        request_id,
                                        status: ses_notification.event_type.clone(),
                                        raw: Some(message),
                                    };

                                    match result.save(&state.db_pool).await {
                                        Ok(_) => (StatusCode::OK, "OK").into_response(),
                                        Err(e) => {
                                            error!("Failed to save event to database: {:?}", e);
                                            (
                                                StatusCode::INTERNAL_SERVER_ERROR,
                                                "Failed to save event",
                                            )
                                                .into_response()
                                        }
                                    }
                                }
                                Err(e) => {
                                    // Log *both* SNS and SES message IDs for debugging
                                    error!("Failed to retrieve request_id. SNS MessageId: {}, SES MessageId: {}, Error: {:?}", message_id, ses_msg_id, e);
                                    (
                                        StatusCode::INTERNAL_SERVER_ERROR,
                                        "Failed to retrieve request_id",
                                    )
                                        .into_response()
                                }
                            }
                        }
                        None => {
                            // --- 4d. Handle missing SES message_id ---
                            error!("SES message_id not found in notification. SNS MessageId: {}.  Message: {}", message_id, message);
                            (StatusCode::BAD_REQUEST, "SES message_id not found").into_response()
                        }
                    }
                }
                Err(e) => {
                    // --- 4e. Handle Non-JSON or Incorrect SES Messages ---
                    error!(
                        "Failed to parse SES notification: {:?}, message: {}",
                        e, message
                    ); // Log error *and* message
                    (StatusCode::OK, "Non-SES notification received").into_response()
                }
            }
        }
        CreateEventRequest::Other(_) => {
            info!("Received other message type");
            (StatusCode::OK, "Other message type received").into_response()
        }
    }
}
