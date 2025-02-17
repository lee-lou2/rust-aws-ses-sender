use crate::models::request::{EmailMessageStatus, EmailRequest};
use sqlx::SqlitePool;
use std::time::Duration;
use tokio::sync::mpsc;

/// schedule_pre_send_message
/// Scheduler for sending scheduled messages
/// Queries valid messages in batches of 1000 based on the current time and sends them
pub async fn schedule_pre_send_message(tx: &mpsc::Sender<EmailRequest>, db_pool: SqlitePool) {
    loop {
        match sqlx::query!(
            "SELECT id, topic_id, email, subject, content \
             FROM email_requests \
             WHERE status = 0 AND scheduled_at <= datetime('now') \
             LIMIT 1000"
        )
        .fetch_all(&db_pool)
        .await
        {
            Ok(rows) => {
                if rows.is_empty() {
                    println!("No data to send");
                    tokio::time::sleep(Duration::from_secs(60)).await;
                    continue;
                }

                // 1. Collect ids of the retrieved data and update their status
                let ids: Vec<i32> = rows
                    .iter()
                    .filter_map(|row| row.id)
                    .map(|id| id as i32)
                    .collect();

                if !ids.is_empty() {
                    let ids_str = ids
                        .iter()
                        .map(|id| id.to_string())
                        .collect::<Vec<_>>()
                        .join(",");
                    let update_sql = format!(
                        "UPDATE email_requests SET status = 1 WHERE id IN ({})",
                        ids_str
                    );
                    if let Err(e) = sqlx::query(&update_sql).execute(&db_pool).await {
                        eprintln!("Failed to update status: {:?}", e);
                        continue;
                    }
                }

                for row in rows {
                    let id = row.id.unwrap_or(0) as i32;
                    let topic_id = row.topic_id;
                    let email = row.email;
                    let subject = row.subject;
                    let content = row.content;
                    let request = EmailRequest {
                        id: Some(id),
                        topic_id: Some(topic_id),
                        email,
                        subject,
                        content,
                        // Unused value (initialization only)
                        scheduled_at: None,
                        status: EmailMessageStatus::Created as i32,
                        error: None,
                        message_id: None,
                    };
                    if let Err(e) = tx.send(request).await {
                        eprintln!("Failed to send data to channel: {:?}", e);
                    }
                }
            }
            Err(e) => {
                eprintln!("Error fetching events: {:?}", e);
            }
        }
    }
}
