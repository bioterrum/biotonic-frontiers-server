use anyhow::Result;
use sqlx::PgPool;
use uuid::Uuid;

/// Atomically apply an Elo delta and return the new rating.
pub async fn apply_delta(db: &PgPool, player_id: Uuid, delta: i32) -> Result<i32> {
    let new = sqlx::query_scalar!(
        "UPDATE players
             SET elo_rating = GREATEST(0, elo_rating + $2)
           WHERE id = $1
       RETURNING elo_rating",
        player_id,
        delta
    )
    .fetch_one(db)
    .await?;
    Ok(new)
}
