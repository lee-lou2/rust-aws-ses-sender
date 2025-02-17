use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

/// Result
/// Email delivery result
#[derive(Serialize, Deserialize)]
pub struct EmailResult {
    pub id: Option<i32>,
    pub request_id: i32,
    pub status: String,
    pub raw: Option<String>,
}

impl EmailResult {
    /// save
    /// Save email delivery result
    pub async fn save(self, db_pool: &SqlitePool) -> Result<Self, sqlx::Error> {
        let instance = sqlx::query!(
            r#"
            INSERT INTO email_results (
                request_id,
                status,
                raw,
                created_at
            ) VALUES (?, ?, ?, datetime('now'))
            RETURNING id
            "#,
            self.request_id,
            self.status,
            self.raw,
        )
        .fetch_one(db_pool)
        .await?;

        Ok(Self {
            id: instance.id.map(|id| id as i32),
            ..self
        })
    }

    /// get_result_counts_by_topic_id
    /// Retrieve result counts by topic
    pub async fn get_result_counts_by_topic_id(
        db_pool: &SqlitePool,
        topic_id: &str,
    ) -> Result<std::collections::HashMap<String, i32>, sqlx::Error> {
        let results = sqlx::query!(
            r#"
            SELECT status, COUNT(DISTINCT request_id) as count
            FROM email_results
            WHERE request_id IN (
                SELECT id
                FROM email_requests
                WHERE topic_id = ?
            )
            GROUP BY status
            "#,
            topic_id,
        )
        .fetch_all(db_pool)
        .await?;

        let mut result_counts = std::collections::HashMap::new();
        for result in results {
            result_counts.insert(result.status, result.count as i32);
        }
        Ok(result_counts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;
    use std::collections::HashMap;

    async fn setup_db() -> SqlitePool {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("Failed to create in-memory database");

        sqlx::query(
            r#"
        CREATE TABLE email_requests (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            topic_id VARCHAR(255) NOT NULL,
            message_id VARCHAR(255) DEFAULT NULL,
            email VARCHAR(255) NOT NULL,
            subject VARCHAR(255) NOT NULL,
            content TEXT NOT NULL,
            scheduled_at DATETIME NOT NULL,
            status TINYINT NOT NULL DEFAULT 0,
            error VARCHAR(255) DEFAULT NULL,
            created_at DATETIME NOT NULL DEFAULT (datetime('now')),
            updated_at DATETIME NOT NULL DEFAULT (datetime('now')),
            deleted_at DATETIME
        );
        "#,
        )
        .execute(&pool)
        .await
        .expect("Failed to create email_requests table");

        sqlx::query(
            r#"
        CREATE TABLE email_results (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            request_id INTEGER NOT NULL,
            status VARCHAR(50) NOT NULL,
            raw TEXT,
            created_at DATETIME NOT NULL DEFAULT (datetime('now')),
            FOREIGN KEY (request_id) REFERENCES email_requests(id)
        );
        "#,
        )
        .execute(&pool)
        .await
        .expect("Failed to create email_results table");

        pool
    }

    #[tokio::test]
    async fn test_get_result_counts_by_topic_id() {
        let db_pool = setup_db().await;
        let topic_id = "topic1";

        // email_requests 테이블에 테스트 데이터 삽입
        sqlx::query!(
            r#"
            INSERT INTO email_requests (topic_id, email, subject, content, scheduled_at)
            VALUES (?, ?, ?, ?, datetime('now'))
            "#,
            topic_id,
            "test@example.com",
            "subject",
            "content"
        )
        .execute(&db_pool)
        .await
        .expect("Failed to insert into email_requests");

        // email_results 테이블에 테스트 데이터 삽입
        let _result1 = EmailResult {
            id: None,
            request_id: 1,
            status: "success".to_string(),
            raw: Some("raw data 1".to_string()),
        }
        .save(&db_pool)
        .await
        .expect("Failed to save result 1");

        let _result2 = EmailResult {
            id: None,
            request_id: 1,
            status: "failed".to_string(),
            raw: Some("raw data 2".to_string()),
        }
        .save(&db_pool)
        .await
        .expect("Failed to save result 2");

        let result_counts = EmailResult::get_result_counts_by_topic_id(&db_pool, topic_id)
            .await
            .expect("Failed to get result counts");

        let mut expected_counts = HashMap::new();
        expected_counts.insert("success".to_string(), 1);
        expected_counts.insert("failed".to_string(), 1);

        assert_eq!(result_counts, expected_counts);
    }
    #[tokio::test]
    async fn test_get_result_counts_by_topic_id_no_results() {
        let db_pool = setup_db().await;
        let topic_id = "topic_no_results";

        // 결과가 없는 경우 빈 HashMap을 반환하는지 확인
        let result_counts = EmailResult::get_result_counts_by_topic_id(&db_pool, topic_id)
            .await
            .expect("Failed to get result counts");

        assert!(result_counts.is_empty());
    }
}
