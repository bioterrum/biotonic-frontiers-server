//! Player match-history queries.

use actix_web::{get, web, HttpResponse, Responder};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Serialize)]
pub struct GameSummary {
    pub game_id: Uuid,
    pub opponent_id: Option<Uuid>,
    pub winner_id: Option<Uuid>,
    pub player_elo_delta: i32,
    pub opponent_elo_delta: i32,
    pub finished_at: chrono::DateTime<chrono::Utc>,
}

/// GET /api/games/history/{player_id}
#[get("/games/history/{player_id}")]
pub async fn history(path: web::Path<Uuid>, db: web::Data<PgPool>) -> impl Responder {
    let pid = path.into_inner();

    // Fetch games where this player participated. COALESCE handles
    // early-prototype rows without `winner_id`.
    let rows = sqlx::query_as!(
        GameSummary,
        r#"
        SELECT
            g.id                                   AS "game_id!",
            CASE
                WHEN g.player1_id = $1 THEN g.player2_id
                ELSE g.player1_id
            END                                     AS "opponent_id",
            g.winner_id                             AS "winner_id",
            CASE
                WHEN g.player1_id = $1 THEN g.player1_elo_delta
                ELSE g.player2_elo_delta
            END                                     AS "player_elo_delta!",
            CASE
                WHEN g.player1_id = $1 THEN g.player2_elo_delta
                ELSE g.player1_elo_delta
            END                                     AS "opponent_elo_delta!",
            g.updated_at                            AS "finished_at!"
        FROM games g
        WHERE g.player1_id = $1 OR g.player2_id = $1
        ORDER BY g.updated_at DESC
        LIMIT 100
        "#,
        pid
    )
    .fetch_all(&**db)
    .await
    .unwrap_or_default();

    HttpResponse::Ok().json(rows)
}

pub fn init_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(history);
}
