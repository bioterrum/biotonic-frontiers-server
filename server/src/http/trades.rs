//! Player-to-player trade: moves items and credits transactionally.

use actix_web::{post, web, HttpResponse, Responder};
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct TradeReq {
    from_player: Uuid, // seller
    to_player: Uuid,   // buyer
    item_id: i32,
    qty: i32,
    price: i64, // total credits exchanged
}

/// POST /api/trades
#[post("/trades")]
pub async fn trade(info: web::Json<TradeReq>, db: web::Data<PgPool>) -> impl Responder {
    if info.qty <= 0 || info.price < 0 {
        return HttpResponse::BadRequest().body("bad qty / price");
    }

    let mut tx = match db.begin().await {
        Ok(t) => t,
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };

    // 1) seller stock
    let (stock,) = sqlx::query_as::<_, (i64,)>(
        "SELECT COALESCE(SUM(quantity),0)::BIGINT \
         FROM player_items \
         WHERE player_id = $1 AND item_id = $2",
    )
    .bind(info.from_player)
    .bind(info.item_id)
    .fetch_one(&mut *tx)
    .await
    .unwrap_or((0,));

    if stock < info.qty as i64 {
        tx.rollback().await.ok();
        return HttpResponse::BadRequest().body("seller lacks items");
    }

    // 2) buyer credits
    let (credits,) = sqlx::query_as::<_, (i64,)>("SELECT credits FROM players WHERE id = $1")
        .bind(info.to_player)
        .fetch_one(&mut *tx)
        .await
        .unwrap_or((0,));

    if credits < info.price {
        tx.rollback().await.ok();
        return HttpResponse::BadRequest().body("buyer lacks credits");
    }

    // 3) move items
    let _ = sqlx::query!(
        "UPDATE player_items
         SET quantity = quantity - $1
         WHERE player_id = $2 AND item_id = $3",
        info.qty,
        info.from_player,
        info.item_id
    )
    .execute(&mut *tx)
    .await;

    let _ = sqlx::query!(
        r#"INSERT INTO player_items (player_id, item_id, quantity)
           VALUES ($1,$2,$3)
           ON CONFLICT (player_id,item_id)
           DO UPDATE SET quantity = player_items.quantity + EXCLUDED.quantity"#,
        info.to_player,
        info.item_id,
        info.qty
    )
    .execute(&mut *tx)
    .await;

    // 4) move credits
    let _ = sqlx::query!(
        "UPDATE players SET credits = credits - $1 WHERE id = $2",
        info.price,
        info.to_player,
    )
    .execute(&mut *tx)
    .await;

    let _ = sqlx::query!(
        "UPDATE players SET credits = credits + $1 WHERE id = $2",
        info.price,
        info.from_player,
    )
    .execute(&mut *tx)
    .await;

    // 5) record trade
    let _ = sqlx::query!(
        r#"INSERT INTO trades (from_player, to_player, item_id, qty, price)
           VALUES ($1,$2,$3,$4,$5)"#,
        info.from_player,
        info.to_player,
        info.item_id,
        info.qty,
        info.price as i32 // trades.price is INT
    )
    .execute(&mut *tx)
    .await;

    tx.commit().await.ok();
    HttpResponse::Ok().body("trade executed")
}

pub fn init_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(trade);
}
