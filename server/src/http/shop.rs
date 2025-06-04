use actix_web::{error, get, post, web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Serialize)]
pub struct ShopEntry {
    pub item_id: i32,
    pub name: String,
    pub description: Option<String>,
    pub price: i32,
}

#[derive(Deserialize)]
pub struct BuyReq {
    pub player_id: Uuid,
    pub item_id: i32,
    pub quantity: i32,
}

#[derive(Deserialize)]
pub struct SellReq {
    pub player_id: Uuid,
    pub item_id: i32,
    pub quantity: i32,
}

/// price = base_price + current global stock
async fn dynamic_price(db: &PgPool, item_id: i32, base: i32) -> i32 {
    // Try to fetch SUM(quantity); treat any error or NULL as 0
    let stock_res = sqlx::query_scalar!(
        "SELECT SUM(quantity)::BIGINT FROM player_items WHERE item_id = $1",
        item_id
    )
    .fetch_optional(db)
    .await;

    // stock_res: Result<Option<Option<i64>>, _>
    let stock: i64 = match stock_res {
        // Ok(Some(inner)) → inner: Option<i64>; unwrap or default to 0
        Ok(Some(inner)) => inner.unwrap_or(0),
        // Any error or no row → 0
        _ => 0,
    };

    base + (stock as i32)
}

/// GET /api/shop/items
#[get("/shop/items")]
pub async fn list_items(db: web::Data<PgPool>) -> impl Responder {
    let rows = sqlx::query!("SELECT id, name, description, base_price FROM items ORDER BY id")
        .fetch_all(&**db)
        .await
        .unwrap_or_default();

    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let price = dynamic_price(&db, row.id, row.base_price).await;
        out.push(ShopEntry {
            item_id: row.id,
            name: row.name,
            description: row.description,
            price,
        });
    }

    HttpResponse::Ok().json(out)
}

/// POST /api/shop/buy
#[post("/shop/buy")]
pub async fn buy(
    info: web::Json<BuyReq>,
    db: web::Data<PgPool>,
) -> Result<HttpResponse, actix_web::Error> {
    if info.quantity <= 0 {
        return Ok(HttpResponse::BadRequest().body("quantity must be > 0"));
    }

    let mut tx = db.begin().await.map_err(error::ErrorInternalServerError)?;

    // 1) Fetch base price
    let base_price_opt =
        sqlx::query_scalar!("SELECT base_price FROM items WHERE id = $1", info.item_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(error::ErrorInternalServerError)?;

    let base_price = if let Some(bp) = base_price_opt {
        bp
    } else {
        tx.rollback().await.ok();
        return Err(error::ErrorBadRequest("invalid item_id"));
    };

    // 2) Compute total cost
    let unit_price = dynamic_price(&db, info.item_id, base_price).await;
    let total_cost = unit_price.checked_mul(info.quantity).unwrap_or_default();
    let total_cost_i64 = total_cost as i64;

    // 3) Debit credits
    let debited = sqlx::query!(
        "UPDATE players SET credits = credits - $1 WHERE id = $2 AND credits >= $1",
        total_cost_i64,
        info.player_id
    )
    .execute(&mut *tx)
    .await
    .map_err(error::ErrorInternalServerError)?
    .rows_affected();

    if debited == 0 {
        tx.rollback().await.ok();
        return Ok(HttpResponse::BadRequest().body("insufficient credits"));
    }

    // 4) Upsert inventory
    sqlx::query!(
        r#"
        INSERT INTO player_items (player_id, item_id, quantity)
        VALUES ($1, $2, $3)
        ON CONFLICT (player_id, item_id)
        DO UPDATE SET quantity = player_items.quantity + EXCLUDED.quantity
        "#,
        info.player_id,
        info.item_id,
        info.quantity
    )
    .execute(&mut *tx)
    .await
    .map_err(error::ErrorInternalServerError)?;

    tx.commit().await.map_err(error::ErrorInternalServerError)?;
    Ok(HttpResponse::Ok().body("purchased"))
}

/// POST /api/shop/sell
#[post("/shop/sell")]
pub async fn sell(
    info: web::Json<SellReq>,
    db: web::Data<PgPool>,
) -> Result<HttpResponse, actix_web::Error> {
    if info.quantity <= 0 {
        return Ok(HttpResponse::BadRequest().body("quantity must be > 0"));
    }

    let mut tx = db.begin().await.map_err(error::ErrorInternalServerError)?;

    // 1) Check ownership — unwrap the Option<i64> to i64
    let owned: i64 = sqlx::query_scalar!(
        r#"
        SELECT COALESCE(SUM(quantity)::BIGINT, 0)
          FROM player_items
         WHERE player_id = $1 AND item_id = $2
        "#,
        info.player_id,
        info.item_id
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(error::ErrorInternalServerError)?
    .unwrap_or(0);

    if owned < (info.quantity as i64) {
        tx.rollback().await.ok();
        return Ok(HttpResponse::BadRequest().body("not enough items"));
    }

    // 2) Fetch base price
    let base_price_opt =
        sqlx::query_scalar!("SELECT base_price FROM items WHERE id = $1", info.item_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(error::ErrorInternalServerError)?;

    let base_price = if let Some(bp) = base_price_opt {
        bp
    } else {
        tx.rollback().await.ok();
        return Err(error::ErrorBadRequest("invalid item_id"));
    };

    // 3) Compute gain
    let unit_price = dynamic_price(&db, info.item_id, base_price).await;
    let total_gain = (unit_price.saturating_sub(1) as i64).saturating_mul(info.quantity as i64);

    // 4) Remove items
    sqlx::query!(
        "UPDATE player_items SET quantity = quantity - $3 WHERE player_id = $1 AND item_id = $2",
        info.player_id,
        info.item_id,
        info.quantity
    )
    .execute(&mut *tx)
    .await
    .map_err(error::ErrorInternalServerError)?;

    // 5) Credit player
    sqlx::query!(
        "UPDATE players SET credits = credits + $2 WHERE id = $1",
        info.player_id,
        total_gain
    )
    .execute(&mut *tx)
    .await
    .map_err(error::ErrorInternalServerError)?;

    tx.commit().await.map_err(error::ErrorInternalServerError)?;
    Ok(HttpResponse::Ok().json(json!({ "gained": total_gain })))
}

pub fn init_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(list_items).service(buy).service(sell);
}
