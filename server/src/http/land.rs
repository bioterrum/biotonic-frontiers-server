//! Land‐parcel endpoints: claim, inspect, and list owned parcels.

use actix_web::{get, post, web, Error, HttpResponse};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Serialize)]
pub struct LandParcel {
    pub id: i32,
    pub biome_type: String,
    pub owner_faction_id: Option<Uuid>,
    pub x: i32,
    pub y: i32,
    pub created_at: DateTime<Utc>,
}

#[derive(Serialize)]
struct OwnedParcel {
    pub x: i32,
    pub y: i32,
    pub biome: String,
    pub owner_faction_id: Option<Uuid>,
}

#[derive(Deserialize)]
pub struct ClaimReq {
    pub faction_id: Uuid,
    pub x: i32,
    pub y: i32,
    pub biome_type: String,
}

#[post("/land/claim")]
pub async fn claim(
    info: web::Json<ClaimReq>,
    db: web::Data<PgPool>,
) -> Result<HttpResponse, Error> {
    // 1) Check for existing parcel at (x,y)
    let existing = sqlx::query_as::<_, (i32, Option<Uuid>)>(
        "SELECT id, owner_faction_id FROM land_parcels WHERE x = $1 AND y = $2",
    )
    .bind(info.x)
    .bind(info.y)
    .fetch_optional(&**db)
    .await
    .map_err(actix_web::error::ErrorInternalServerError)?;

    if let Some((id, owner)) = existing {
        if owner == Some(info.faction_id) {
            // Already owned by this faction
            return Ok(HttpResponse::Ok().json(serde_json::json!({ "parcel_id": id })));
        } else {
            return Ok(HttpResponse::Conflict().body("parcel already owned"));
        }
    }

    // 2) Insert new parcel
    let id: i32 = sqlx::query_scalar!(
        r#"INSERT INTO land_parcels (biome_type, owner_faction_id, x, y)
           VALUES ($1, $2, $3, $4)
           RETURNING id"#,
        info.biome_type,
        info.faction_id,
        info.x,
        info.y
    )
    .fetch_one(&**db)
    .await
    .map_err(actix_web::error::ErrorInternalServerError)?;

    Ok(HttpResponse::Ok().json(serde_json::json!({ "parcel_id": id })))
}

#[get("/land/at/{x}/{y}")]
pub async fn parcel_at(
    path: web::Path<(i32, i32)>,
    db: web::Data<PgPool>,
) -> Result<HttpResponse, Error> {
    let (x, y) = path.into_inner();
    let row = sqlx::query_as!(
        LandParcel,
        r#"SELECT id, biome_type, owner_faction_id, x, y, created_at
           FROM land_parcels
           WHERE x = $1 AND y = $2"#,
        x,
        y
    )
    .fetch_optional(&**db)
    .await
    .map_err(actix_web::error::ErrorInternalServerError)?;

    match row {
        Some(parcel) => Ok(HttpResponse::Ok().json(parcel)),
        None => Ok(HttpResponse::NotFound().finish()),
    }
}

#[get("/land/owned/{player_id}")]
pub async fn owned(path: web::Path<Uuid>, db: web::Data<PgPool>) -> Result<HttpResponse, Error> {
    let pid = path.into_inner();

    let rows = sqlx::query_as!(
        OwnedParcel,
        r#"
        SELECT x,
               y,
               biome_type AS biome,
               owner_faction_id
          FROM land_parcels
         WHERE owner_player_id = $1
            OR owner_faction_id IN (
                   SELECT faction_id
                     FROM faction_members
                    WHERE player_id = $1
               )
        "#,
        pid
    )
    .fetch_all(&**db)
    .await
    .map_err(actix_web::error::ErrorInternalServerError)?;

    Ok(HttpResponse::Ok().json(rows))
}

pub fn init_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(claim).service(parcel_at).service(owned);
}
