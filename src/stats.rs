use chrono::NaiveDate;
use sqlx::SqlitePool;

pub struct Stats;

impl Stats {
    pub async fn increment_message_count(
        pool: &SqlitePool,
        d: NaiveDate,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
        INSERT INTO stats (date, message_count)
        VALUES (?, 1)
        ON CONFLICT(date) DO UPDATE SET message_count = message_count + 1
        "#,
        )
        .bind(d.to_string())
        .execute(pool)
        .await?;

        Ok(())
    }
}
