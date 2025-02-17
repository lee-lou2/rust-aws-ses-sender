use crate::config;
use crate::models::request::{EmailMessageStatus, EmailRequest};
use sqlx::SqlitePool;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::sync::Mutex;
use tokio::time::interval;

/// receive_send_message
/// Message reception and sending
pub async fn receive_send_message(
    rx: &Arc<Mutex<mpsc::Receiver<EmailRequest>>>,
    tx: &mpsc::Sender<EmailRequest>,
) {
    let envs = config::get_environments();
    let max_send_per_second = envs.max_send_per_second;
    // Consume 24 messages per second
    let mut interval = interval(Duration::from_millis(1000 / max_send_per_second as u64));
    let mut rx_guard = rx.lock().await;
    loop {
        interval.tick().await;
        if let Some(mut request) = rx_guard.recv().await {
            let server_url = &envs.server_url;
            request.content = format!(
                "{}<img src=\"{}/v1/events/open?request_id={}\">",
                request.content,
                server_url,
                request.id.unwrap_or_default()
            );
            let cloned_tx = tx.clone();
            tokio::spawn(async move {
                let send_result = crate::services::sender::send_email(
                    &envs.aws_ses_from_email,
                    &request.email,
                    &request.subject,
                    &request.content,
                )
                .await;

                match send_result {
                    Ok(message_id) => {
                        request.status = EmailMessageStatus::Sent as i32;
                        request.message_id = Some(message_id);
                    }
                    Err(e) => {
                        request.status = EmailMessageStatus::Failed as i32;
                        request.error = Some(format!("Failed to send email: {}", e));
                    }
                }
                if let Err(e) = cloned_tx.send(request).await {
                    eprintln!("Error sending data to channel: {:?}", e);
                } else {
                    println!("Data sent to channel");
                }
            });
        } else {
            break;
        }
    }
}

/// receive_post_send_message
/// Update the database with received message results
pub async fn receive_post_send_message(
    rx: &Arc<Mutex<mpsc::Receiver<EmailRequest>>>,
    db_pool: SqlitePool,
) {
    let mut rx_guard = rx.lock().await;
    while let Some(request) = rx_guard.recv().await {
        request.update(&db_pool).await;
    }
}
