#[cfg(test)]
mod tests {
    use jsonwebtoken::{encode, EncodingKey, Header};
    use serde::{Deserialize, Serialize};
    use sqlx::Row;
    use std::env;
    use tower::util::ServiceExt;

    async fn db_pool() -> sqlx::sqlite::SqlitePool {
        let db_pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(10)
            .connect("sqlite::memory:")
            .await
            .expect("Failed to create pool");
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS email_requests (
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

            CREATE INDEX idx_requests_status ON email_requests(status);
            CREATE INDEX idx_requests_scheduled_at ON email_requests(scheduled_at DESC);
            CREATE INDEX idx_requests_topic_id ON email_requests(topic_id);

            CREATE TABLE IF NOT EXISTS email_results (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                request_id INTEGER NOT NULL,
                status VARCHAR(50) NOT NULL,
                raw TEXT,
                created_at DATETIME NOT NULL DEFAULT (datetime('now')),
                FOREIGN KEY (request_id) REFERENCES email_requests(id)
            );

            CREATE INDEX idx_results_status ON email_results(status);
            "#,
        )
        .execute(&db_pool)
        .await
        .expect("Failed to create tables");
        db_pool
    }

    async fn authorize() -> String {
        #[derive(Debug, Serialize, Deserialize)]
        struct Claims {
            sub: String,
            exp: usize,
        }

        let jwt_secret = "secret";
        env::set_var("JWT_SECRET", jwt_secret);
        let claims = Claims {
            sub: "".to_string(),
            exp: 10000000000,
        };
        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(jwt_secret.as_ref()),
        )
        .expect("Failed to generate JWT token");
        token
    }

    #[tokio::test]
    async fn test_open_message_handler_success_get_image() {
        // Test to return a 1x1 blank image to create an email open event
        // 1. Check if the API status is 200
        // 2. Check if the Content-Type of the returned image is image/png
        let db_pool = db_pool().await;
        let (tx_send, _) = tokio::sync::mpsc::channel(1);
        let cloned_tx_send = tx_send.clone();
        let app = crate::app::app(crate::state::AppState::new(db_pool, cloned_tx_send))
            .await
            .unwrap();
        let response = axum::http::Request::builder()
            .uri("/v1/events/open")
            .method("GET")
            .body(axum::body::Body::empty())
            .unwrap();
        let response = app.oneshot(response).await.unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::OK);
        assert_eq!(response.headers().get("Content-Type").unwrap(), "image/png");
    }

    #[tokio::test]
    async fn test_open_message_handler_success_insert_open_event() {
        // Test to create an email request and create an email open event for that request
        // 1. Create an email request
        // 2. Create an email open event for the created email request
        // 3. Check if the created email open event is successfully saved in the DB
        // 4. Check if the status of the created email open event is Open
        let db_pool = db_pool().await;
        sqlx::query(
            r#"
            INSERT INTO email_requests (id, topic_id, email, subject, content, scheduled_at)
            VALUES (1, 'topic_id', 'test', 'test', 'test', datetime('now'));
            "#,
        )
        .execute(&db_pool)
        .await
        .expect("Failed to insert email request");

        let (tx_send, _) = tokio::sync::mpsc::channel(1);
        let cloned_tx_send = tx_send.clone();
        let app = crate::app::app(crate::state::AppState::new(db_pool.clone(), cloned_tx_send))
            .await
            .unwrap();
        let response = axum::http::Request::builder()
            .uri("/v1/events/open?request_id=1")
            .method("GET")
            .body(axum::body::Body::empty())
            .unwrap();
        let response = app.oneshot(response).await.unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::OK);

        let result = sqlx::query("SELECT * FROM email_results WHERE request_id = 1")
            .fetch_one(&db_pool)
            .await
            .expect("Failed to fetch email result");
        assert_eq!(result.get::<i64, _>("request_id"), 1);
        assert_eq!(result.get::<String, _>("status"), "Open");
    }

    #[tokio::test]
    async fn test_open_message_handler_fail_path_in_slash() {
        // Test to return a 1x1 blank image to create an email open event
        // 1. Check if a 404 status is returned when there is a / at the end of the API endpoint
        let db_pool = db_pool().await;
        let (tx_send, _) = tokio::sync::mpsc::channel(1);
        let cloned_tx_send = tx_send.clone();
        let app = crate::app::app(crate::state::AppState::new(db_pool, cloned_tx_send))
            .await
            .unwrap();
        let response = axum::http::Request::builder()
            .uri("/v1/events/open/")
            .method("GET")
            .body(axum::body::Body::empty())
            .unwrap();
        let response = app.oneshot(response).await.unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_open_message_handler_fail_unauthorized() {
        // Test to return a 401 status when the request is not authorized
        // 1. Check if a 401 status is returned when the request is not authorized
        // 2. Check if the Content-Type of the returned image is image/png
        let db_pool = db_pool().await;
        let (tx_send, _) = tokio::sync::mpsc::channel(1);
        let cloned_tx_send = tx_send.clone();
        let app = crate::app::app(crate::state::AppState::new(db_pool.clone(), cloned_tx_send))
            .await
            .unwrap();
        let response = axum::http::Request::builder()
            .uri("/v1/events/counts/sent")
            .method("GET")
            .body(axum::body::Body::empty())
            .unwrap();
        let response = app.oneshot(response).await.unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_open_message_handler_success_get_sent_count() {
        // Test to get the sent count of the email requests
        // 1. Create an email request
        // 2. Create an email sent event for the created email request
        // 3. Check if the created email sent event is successfully saved in the DB
        // 4. Check if the status of the created email sent event is Sent
        let db_pool = db_pool().await;
        sqlx::query(
            r#"
            INSERT INTO email_requests (id, topic_id, email, subject, content, status, scheduled_at)
            VALUES (1, 'topic_id', 'test', 'test', 'test', 2, datetime('now'));
            "#,
        )
        .execute(&db_pool)
        .await
        .expect("Failed to insert email request");

        let token = authorize().await;
        let (tx_send, _) = tokio::sync::mpsc::channel(1);
        let cloned_tx_send = tx_send.clone();
        let app = crate::app::app(crate::state::AppState::new(db_pool.clone(), cloned_tx_send))
            .await
            .unwrap();
        let response = axum::http::Request::builder()
            .uri("/v1/events/counts/sent")
            .method("GET")
            .header(
                axum::http::header::AUTHORIZATION,
                format!("Bearer {}", token),
            )
            .body(axum::body::Body::empty())
            .unwrap();
        let response = app.oneshot(response).await.unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: crate::handlers::event_handlers::GetSentCountResponse =
            serde_json::from_slice(&body).unwrap();
        assert_eq!(body.count, 1);
    }

    #[tokio::test]
    async fn test_get_sent_count_handler_success_get_has_not_sent_count() {
        // Get sent email count - no sent history
        // 1. Create data that has been requested to be sent but has not been sent
        // 2. Get sent email count
        // 3. Test successful if sent email count is 0
        let db_pool = db_pool().await;
        sqlx::query(
            r#"
            INSERT INTO email_requests (id, topic_id, email, subject, content, status, scheduled_at)
            VALUES (1, 'topic_id', 'test', 'test', 'test', 1, datetime('now'));
            "#,
        )
        .execute(&db_pool)
        .await
        .expect("Failed to insert email request");

        let token = authorize().await;
        let (tx_send, _) = tokio::sync::mpsc::channel(1);
        let cloned_tx_send = tx_send.clone();
        let app = crate::app::app(crate::state::AppState::new(db_pool.clone(), cloned_tx_send))
            .await
            .unwrap();
        let response = axum::http::Request::builder()
            .uri("/v1/events/counts/sent")
            .method("GET")
            .header(
                axum::http::header::AUTHORIZATION,
                format!("Bearer {}", token),
            )
            .body(axum::body::Body::empty())
            .unwrap();
        let response = app.oneshot(response).await.unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: crate::handlers::event_handlers::GetSentCountResponse =
            serde_json::from_slice(&body).unwrap();
        assert_eq!(body.count, 0);
    }
}
