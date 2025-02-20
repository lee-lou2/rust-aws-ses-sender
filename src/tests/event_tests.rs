#[cfg(test)]
mod tests {
    use sqlx::Row;
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

    #[tokio::test]
    async fn test_open_message_handler_success_get_image() {
        // 이메일 오픈 이벤트를 생성하기 위한 1x1 공백 이미지를 반환하는 테스트
        // 1. API 상태가 200인지 확인
        // 2. 반환된 이미지의 Content-Type이 image/png인지 확인
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
        // 이메일 요청을 생성하고, 해당 요청에 대한 이메일 오픈 이벤트를 생성하는 테스트
        // 1. 이메일 요청 생성
        // 2. 생성된 이메일 요청에 대한 이메일 오픈 이벤트 생성
        // 3. 생성된 이메일 오픈 이벤트가 DB에 정상적으로 저장되었는지 확인
        // 4. 생성된 이메일 오픈 이벤트의 상태가 Open인지 확인
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
        // 이메일 오픈 이벤트를 생성하기 위한 1x1 공백 이미지를 반환하는 테스트
        // 1. API 엔드포인트 마지막에 /가 있는 경우 404 상태를 반환하는지 확인
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
}
