//! Inventory endpoints (+ dev-only starter-loot helper).

use actix_web::{error, get, post, web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::http::auth::JwtAuth;

#[derive(Serialize)]
pub struct InventoryEntry {
    item_id: i32,
    name: String,
    quantity: i32,
}

#[derive(Deserialize)]
pub struct UseReq {
    pub item_id: i32,
    #[serde(default)]
    pub quantity: i32,
}

/// GET /api/inventory/{player_id}
#[get("/inventory/{player_id}")]
pub async fn get_inventory(path: web::Path<Uuid>, db: web::Data<PgPool>) -> impl Responder {
    let pid = path.into_inner();

    let rows = sqlx::query!(
        r#"
        SELECT
            i.id   AS item_id,
            i.name AS name,
            COALESCE(pi.quantity, 0) AS quantity
        FROM items i
        LEFT JOIN player_items pi
            ON pi.item_id   = i.id
           AND pi.player_id = $1
        ORDER BY i.id
        "#,
        pid
    )
    .fetch_all(&**db)
    .await
    .unwrap_or_default();
    
    let out: Vec<InventoryEntry> = rows
    .into_iter()
    .map(|r| InventoryEntry {
        item_id:  r.item_id,
        name:     r.name,
        quantity: r.quantity.unwrap_or(0),
    })
    .collect();

    HttpResponse::Ok().json(out)
}

/// POST /api/inventory/use
#[post("/inventory/use")]
pub async fn use_item(
    auth: JwtAuth,
    info: web::Json<UseReq>,
    db: web::Data<PgPool>,
) -> Result<HttpResponse, actix_web::Error> {
    let pid = auth.player_id;
    let iid = info.item_id;
    let qty = info.quantity.max(1);

    // try to decrement; fail if not enough
    let res = sqlx::query!(
        r#"UPDATE player_items
              SET quantity = quantity - $3
            WHERE player_id = $1 AND item_id = $2 AND quantity >= $3
        RETURNING quantity"#,
        pid,
        iid,
        qty
    )
    .fetch_optional(&**db)
    .await
    .map_err(error::ErrorInternalServerError)?;

    let resp = match res {
        Some(r) => HttpResponse::Ok().json(json!({ "remaining": r.quantity })),
        None => HttpResponse::BadRequest().body("not enough items"),
    };

    Ok(resp)
}

/// POST /api/inventory/grant_starter   (debug builds only)
#[cfg(debug_assertions)]
#[post("/inventory/grant_starter")]
pub async fn grant_starter(
    info: web::Json<Uuid>, // player_id
    db: web::Data<PgPool>,
) -> impl Responder {
    use rand::rng;
    use rand::seq::SliceRandom;

    let pid = *info;

    // Grab every item ID from the catalogue
    let mut item_ids: Vec<i32> = sqlx::query_scalar!("SELECT id FROM items")
        .fetch_all(&**db)
        .await
        .unwrap_or_default();

    if item_ids.is_empty() {
        return HttpResponse::BadRequest().body("no items in catalogue");
    }

    // Shuffle, then take the first 3 distinct IDs
    let mut rng = rng();
    item_ids.shuffle(&mut rng);
    for iid in item_ids.into_iter().take(3) {
        let _ = sqlx::query!(
            r#"INSERT INTO player_items (player_id, item_id, quantity)
               VALUES ($1, $2, 1)
               ON CONFLICT (player_id, item_id)
               DO UPDATE SET quantity = player_items.quantity + 1"#,
            pid,
            iid
        )
        .execute(&**db)
        .await;
    }

    HttpResponse::Ok().body("starter loot granted")
}

pub fn init_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(get_inventory);
    #[cfg(debug_assertions)]
    cfg.service(grant_starter);
    cfg.service(use_item);
}
