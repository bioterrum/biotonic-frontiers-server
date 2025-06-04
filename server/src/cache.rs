//! Simple in-memory warm cache for static lookup tables (Sprint 1)
//!
//! Currently caches the entire `items` table at start-up so that high-traffic
//! read-only endpoints (shop catalogue, item look-ups) no longer hit Postgres
//! on every request.  This keeps latency low and is an easy first-step perf
//! win while we explore Redis or CDN-based caches later in Beta.

use once_cell::sync::Lazy;
use dashmap::DashMap;
use sqlx::PgPool;

/// One immutable row from the `items` table.
#[derive(Debug, Clone)]
pub struct ItemDef {
    pub id: i32,
    pub name: String,
    pub description: Option<String>,
    pub base_price: i32,
}

/// Global map id → ItemDef (read-only once warmed).
pub static ITEMS: Lazy<DashMap<i32, ItemDef>> = Lazy::new(DashMap::new);

/// Fetch the `items` table and populate [`ITEMS`]. Idempotent.
pub async fn warm_items(db: &PgPool) -> anyhow::Result<()> {
    let rows = sqlx::query!(
        r#"SELECT id, name, description, base_price FROM items"#
    )
    .fetch_all(db)
    .await?;

    for r in rows {
        ITEMS.insert(
            r.id,
            ItemDef {
                id: r.id,
                name: r.name,
                description: r.description,
                base_price: r.base_price,
            },
        );
    }
    Ok(())
}

/// Retrieve a cached item definition by ID.
pub fn get_item(id: i32) -> Option<ItemDef> {
    ITEMS.get(&id).map(|e| e.value().clone())
}

/// Warm every in-memory cache we have (called once at startup).
pub async fn warm_all(db: &PgPool) {
    if let Err(e) = warm_items(db).await {
        log::warn!("cache warm-up failed: {e:?}");
    }
}
