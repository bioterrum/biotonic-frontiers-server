//! Static item catalogue.

use crate::cache::{ItemDef, ITEMS};
use actix_web::{get, web, HttpResponse, Responder};
use serde::Serialize;
use sqlx::PgPool;

#[derive(Serialize)]
struct Item {
    id: i32,
    name: String,
    description: Option<String>,
    base_price: i32,
}

#[get("/items")]
pub async fn list_items(db: web::Data<PgPool>) -> impl Responder {
    // Use in-memory cache if warmed; otherwise fall back to DB
    let defs: Vec<ItemDef> = if !ITEMS.is_empty() {
        ITEMS.iter().map(|e| e.value().clone()).collect()
    } else {
        // Rare fallback path before warm-up completes
        let rows =
            sqlx::query!(r#"SELECT id, name, description, base_price FROM items ORDER BY id"#)
                .fetch_all(&**db)
                .await
                .unwrap_or_default();

        rows.into_iter()
            .map(|r| ItemDef {
                id: r.id,
                name: r.name,
                description: r.description,
                base_price: r.base_price,
            })
            .collect()
    };

    // Map to the HTTP DTO
    let out: Vec<Item> = defs
        .into_iter()
        .map(|it| Item {
            id: it.id,
            name: it.name,
            description: it.description,
            base_price: it.base_price,
        })
        .collect();

    HttpResponse::Ok().json(out)
}

pub fn init_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(list_items);
}
