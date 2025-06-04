use chrono::{DateTime, Utc};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, FromRow)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, FromRow)]
pub struct Player {
    pub id: Uuid,
    pub user_id: Uuid,
    pub nickname: String,
    pub elo_rating: i32,
    pub credits: i64,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, FromRow)]
pub struct Faction {
    pub id: Uuid,
    pub name: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, FromRow)]
pub struct FactionMember {
    pub faction_id: Uuid,
    pub player_id: Uuid,
    pub role: String,
    pub joined_at: DateTime<Utc>,
}
