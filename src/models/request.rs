use chrono::{DateTime, Local, NaiveDateTime, TimeZone, Utc};
use serde::Deserialize;
use sqlx::SqlitePool;

/// EmailMessageStatus
/// Email message status
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum EmailMessageStatus {
    Created = 0,   // Created
    Processed = 1, // Processing
    Sent = 2,      // Sent
    Failed = 3,    // Failed
    Stopped = 4,   // Stopped
}

/// Request
/// Email request
#[derive(Deserialize, Clone)]
pub struct EmailRequest {
    pub id: Option<i32>,
    pub topic_id: Option<String>,
    pub email: String,
    pub subject: String,
    pub content: String,
    pub scheduled_at: Option<String>,
    pub status: i32,
    pub error: Option<String>,
    pub message_id: Option<String>,
}

impl EmailRequest {
    /// save
    /// Save the email request
    pub async fn save(self, db_pool: &SqlitePool) -> Self {
        let now = Utc::now();
        let scheduled_at = match self.scheduled_at.clone() {
            Some(scheduled_at) => {
                if scheduled_at.is_empty() {
                    now.format("%Y-%m-%d %H:%M:%S").to_string()
                } else {
                    // Convert to UTC
                    let naive_dt =
                        NaiveDateTime::parse_from_str(&scheduled_at, "%Y-%m-%d %H:%M:%S")
                            .expect("Failed to parse date");
                    let local_dt: DateTime<Local> = Local
                        .from_local_datetime(&naive_dt)
                        .single()
                        .expect("Ambiguous time or conversion failure");
                    let utc_dt: DateTime<Utc> = local_dt.with_timezone(&Utc);
                    utc_dt.format("%Y-%m-%d %H:%M:%S").to_string()
                }
            }
            None => now.format("%Y-%m-%d %H:%M:%S").to_string(),
        };
        let instance = sqlx::query!(
            r#"
            INSERT INTO email_requests (
                topic_id,
                email,
                subject,
                content,
                scheduled_at,
                status,
                created_at,
                updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, datetime('now'), datetime('now'))
            RETURNING id
            "#,
            self.topic_id,
            self.email,
            self.subject,
            self.content,
            scheduled_at,
            self.status,
        )
        .fetch_one(db_pool)
        .await
        .expect("Failed to insert message");

        EmailRequest {
            id: Some(instance.id as i32),
            ..self
        }
    }

    /// update
    /// Update the email request status
    pub async fn update(self, db_pool: &SqlitePool) {
        sqlx::query!(
            r#"
            UPDATE email_requests
            SET status = ?,
                message_id = ?,
                error = ?,
                updated_at = datetime('now')
            WHERE id = ?
            "#,
            self.status,
            self.message_id,
            self.error,
            self.id,
        )
        .execute(db_pool)
        .await
        .expect("Failed to update message status");
    }

    /// sent_count
    /// Retrieve the count of requests sent in the last n hours
    pub async fn sent_count(db_pool: &SqlitePool, hours: i32) -> Result<i32, sqlx::Error> {
        let hours_str = format!("-{} hours", hours);
        let count = sqlx::query!(
            r#"
            SELECT COUNT(*) as count
            FROM email_requests
            WHERE status = ?
            AND created_at >= datetime('now', ?)
            "#,
            EmailMessageStatus::Sent as i32,
            hours_str,
        )
        .fetch_one(db_pool)
        .await?;

        Ok(count.count as i32)
    }

    /// stop_topic
    /// Stop sending requests for the topic
    pub async fn stop_topic(db_pool: &SqlitePool, topic_id: &str) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
            UPDATE email_requests
            SET status = ?,
                updated_at = datetime('now')
            WHERE status = ? AND topic_id = ?
            "#,
            EmailMessageStatus::Stopped as i32,
            EmailMessageStatus::Created as i32,
            topic_id,
        )
        .execute(db_pool)
        .await?;
        Ok(())
    }

    /// get_request_counts_by_topic_id
    /// Retrieve request counts by topic
    pub async fn get_request_counts_by_topic_id(
        db_pool: &SqlitePool,
        topic_id: &str,
    ) -> Result<std::collections::HashMap<String, i32>, sqlx::Error> {
        let requests = sqlx::query!(
            r#"
            SELECT status, COUNT(*) as "count: i64"
            FROM email_requests
            WHERE topic_id = ?
            GROUP BY status
            "#,
            topic_id,
        )
        .fetch_all(db_pool)
        .await?;

        let mut request_counts = std::collections::HashMap::new();
        for r in requests {
            let status = match r.status {
                0 => "Created".to_string(),
                1 => "Processed".to_string(),
                2 => "Sent".to_string(),
                3 => "Failed".to_string(),
                4 => "Stopped".to_string(),
                _ => "Unknown".to_string(),
            };
            request_counts.insert(status, r.count.unwrap_or(0) as i32);
        }
        Ok(request_counts)
    }

    /// get_request_id_by_message_id
    /// Retrieve the request ID by message ID
    pub async fn get_request_id_by_message_id(
        db_pool: &SqlitePool,
        message_id: &str,
    ) -> Result<i32, sqlx::Error> {
        let request = sqlx::query!(
            r#"
            SELECT id
            FROM email_requests
            WHERE message_id = ?
            "#,
            message_id
        )
        .fetch_one(db_pool)
        .await?;

        Ok(request.id as i32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Local, NaiveDateTime, Utc};
    use regex::Regex;

    // production 코드에서 사용한 로직을 테스트하기 위해 동일한 변환 함수를 작성합니다.
    // 이 함수는 Option<&str>으로 scheduled_at 값을 받아서, UTC 기준 "YYYY-MM-DD HH:MM:SS" 형식의 문자열을 반환합니다.
    fn convert_scheduled_at(scheduled: Option<&str>) -> String {
        let now = Utc::now();
        match scheduled {
            Some(s) => {
                if s.is_empty() {
                    now.format("%Y-%m-%d %H:%M:%S").to_string()
                } else {
                    let naive_dt = NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S")
                        .expect("Failed to parse date");
                    let local_dt = Local
                        .from_local_datetime(&naive_dt)
                        .single()
                        .expect("Ambiguous time or conversion failure");
                    let utc_dt = local_dt.with_timezone(&Utc);
                    utc_dt.format("%Y-%m-%d %H:%M:%S").to_string()
                }
            }
            None => now.format("%Y-%m-%d %H:%M:%S").to_string(),
        }
    }

    // 테스트 1: scheduled_at 값이 None인 경우
    #[test]
    fn test_none_scheduled_at() {
        let result = convert_scheduled_at(None);
        // 결과 문자열이 "YYYY-MM-DD HH:MM:SS" 형식인지 확인
        let re = Regex::new(r"^\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}$").unwrap();
        assert!(
            re.is_match(&result),
            "결과 '{}'가 기대하는 형식과 일치하지 않습니다",
            result
        );
    }

    // 테스트 1 변형: scheduled_at 값이 빈 문자열("")인 경우 -> None과 동일하게 처리됨
    #[test]
    fn test_empty_scheduled_at() {
        let result = convert_scheduled_at(Some(""));
        let re = Regex::new(r"^\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}$").unwrap();
        assert!(
            re.is_match(&result),
            "빈 문자열 입력 결과 '{}'가 올바른 형식이 아닙니다",
            result
        );
    }

    // 테스트 2: scheduled_at 값이 있으나 형식이 전혀 맞지 않는 경우
    #[test]
    #[should_panic(expected = "Failed to parse date")]
    fn test_invalid_format_completely_different() {
        let _ = convert_scheduled_at(Some("invalid_date_string"));
    }

    // 테스트 2 변형: 초(second) 정보가 빠진 경우 (형식 불일치)
    #[test]
    #[should_panic(expected = "Failed to parse date")]
    fn test_invalid_format_missing_seconds() {
        let _ = convert_scheduled_at(Some("2023-10-12 15:30"));
    }

    // 테스트 3: 입력 값이 정상적으로 들어왔을 때
    #[test]
    fn test_valid_scheduled_at() {
        let input = "2023-10-12 15:30:45";
        let result = convert_scheduled_at(Some(input));
        // 예상 결과를 직접 계산: 입력값을 로컬 시간으로 해석한 후 UTC로 변환합니다.
        let naive = NaiveDateTime::parse_from_str(input, "%Y-%m-%d %H:%M:%S").unwrap();
        let local_dt = Local.from_local_datetime(&naive).single().unwrap();
        let utc_dt = local_dt.with_timezone(&Utc);
        let expected = utc_dt.format("%Y-%m-%d %H:%M:%S").to_string();
        assert_eq!(
            result, expected,
            "정상 입력 값의 변환이 예상 결과와 다릅니다"
        );
    }

    // 테스트 4: 타임존이 제대로 변환되는지 확인
    #[test]
    fn test_timezone_conversion() {
        let input = "2023-10-12 15:30:45";
        let result = convert_scheduled_at(Some(input));
        // 로컬 날짜시간과 UTC 변환 값을 계산합니다.
        let naive = NaiveDateTime::parse_from_str(input, "%Y-%m-%d %H:%M:%S").unwrap();
        let local_dt = Local.from_local_datetime(&naive).single().unwrap();
        let offset_secs = local_dt.offset().local_minus_utc();
        // 로컬 타임존이 UTC가 아닌 경우, 변환된 결과는 입력과 달라야 합니다.
        if offset_secs != 0 {
            assert_ne!(
                result, input,
                "타임존 오프셋이 non-zero임에도 불구하고 변환된 시간이 입력과 동일합니다"
            );
        } else {
            // 로컬 타임존이 UTC일 경우에는 결과가 입력과 동일할 수 있습니다.
            assert_eq!(
                result, input,
                "로컬 타임존이 UTC임에도 결과가 입력과 다릅니다"
            );
        }
    }

    // 추가 테스트: 미래 날짜의 경우에도 올바른 형식의 결과가 나오는지 확인
    #[test]
    fn test_future_date_format() {
        let input = "2099-12-31 23:59:59";
        let result = convert_scheduled_at(Some(input));
        let re = Regex::new(r"^\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}$").unwrap();
        assert!(
            re.is_match(&result),
            "미래 날짜 입력 결과 '{}'가 올바른 형식이 아닙니다",
            result
        );
    }
}
