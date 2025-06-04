use crate::db::faction_repo;
use actix_web::{get, post, web, Error, HttpResponse};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Serialize)]
pub struct Structure {
    pub id: i32,
    pub owner_player_id: Option<Uuid>,
    pub owner_faction_id: Option<Uuid>,
    #[serde(rename = "type")]
    pub structure_type: String,
    pub x: i32,
    pub y: i32,
    pub stats: serde_json::Value,
    pub placed_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Deserialize)]
pub struct BuildReq {
    pub player_id: Uuid,
    #[serde(rename = "type")]
    pub structure_type: String,
    pub x: i32,
    pub y: i32,
    #[serde(default)]
    pub stats: serde_json::Value,
}

#[post("/structures/build")]
pub async fn build(
    info: web::Json<BuildReq>,
    db: web::Data<PgPool>,
) -> Result<HttpResponse, Error> {
    // 1) Look up the owning faction ID (nullable column ⇒ Option<Uuid>)
    let owner_faction = sqlx::query_scalar!(
        "SELECT owner_faction_id FROM land_parcels WHERE x = $1 AND y = $2",
        info.x,
        info.y
    )
    .fetch_optional(db.get_ref())
    .await
    .map_err(actix_web::error::ErrorInternalServerError)?
    // fetch_optional ⇒ Result<Option<Option<Uuid>>, _>
    .flatten()
    .ok_or_else(|| actix_web::error::ErrorBadRequest("parcel not claimed"))?;

    // 2) Check membership via our DB helper
    if !faction_repo::is_faction_member(db.get_ref(), owner_faction, info.player_id)
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?
    {
        return Err(actix_web::error::ErrorUnauthorized("not in owning faction"));
    }

    // 3) Insert and return the new structure ID
    let sid: i32 = sqlx::query_scalar!(
        r#"INSERT INTO structures
             (owner_player_id, owner_faction_id, type, x, y, stats)
           VALUES ($1, $2, $3, $4, $5, $6)
           RETURNING id"#,
        info.player_id,
        owner_faction,
        info.structure_type,
        info.x,
        info.y,
        info.stats
    )
    .fetch_one(db.get_ref())
    .await
    .map_err(actix_web::error::ErrorInternalServerError)?;

    Ok(HttpResponse::Ok().json(serde_json::json!({ "structure_id": sid })))
}

#[get("/structures/at/{x}/{y}")]
pub async fn list_at(
    path: web::Path<(i32, i32)>,
    db: web::Data<PgPool>,
) -> Result<HttpResponse, Error> {
    let (x, y) = path.into_inner();
    let rows = sqlx::query_as!(
        Structure,
        r#"SELECT id,
                  owner_player_id,
                  owner_faction_id,
                  type AS "structure_type!",
                  x, y, stats, placed_at
           FROM structures
          WHERE x = $1 AND y = $2"#,
        x,
        y
    )
    .fetch_all(db.get_ref())
    .await
    .map_err(actix_web::error::ErrorInternalServerError)?;

    Ok(HttpResponse::Ok().json(rows))
}

pub fn init_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(build).service(list_at);
}
