use chrono::{NaiveDate, Utc};
use sqlx::prelude::FromRow;

#[derive(Debug, FromRow)]
pub struct ServerUser {
    pub id: i64,
    pub message_count: i64,
    pub mutes_left: i64,
    pub mutes_used: i64,
    pub streak: i64,
    pub last_activity: NaiveDate,
}

impl ServerUser {
    pub async fn get_user_from_id(pool: &sqlx::SqlitePool, user_id: i64) -> Option<ServerUser> {
        sqlx::query_as::<_, ServerUser>(
            r#"
        SELECT * FROM users WHERE id = ?
        "#,
        )
        .bind(user_id)
        .fetch_optional(pool)
        .await
        .unwrap_or(None)
    }

    pub async fn update_user_messages(pool: &sqlx::SqlitePool, user_id: i64, message_count: i64) {
        sqlx::query(
            r#"
        UPDATE users
        SET
            message_count = ?,
        WHERE id = ?
        "#,
        )
        .bind(message_count)
        .bind(user_id)
        .execute(pool)
        .await
        .unwrap();
    }

    pub async fn increment_message_count(
        pool: &sqlx::SqlitePool,
        user_id: i64,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
        INSERT INTO users (id, message_count, mutes_left, mutes_used, streak, last_activity)
        VALUES (?, 1, 0, 0, 1, ?)
        ON CONFLICT(id) DO UPDATE SET message_count = message_count + 1
        "#,
        )
        .bind(user_id)
        .bind(Utc::now().date_naive().to_string())
        .execute(pool)
        .await?;

        Ok(())
    }
}
