use anyhow::Context;
use sqlx::PgPool;
use uuid::Uuid;

/// Returns the owning faction ID for a land parcel at (x, y), if claimed.
pub async fn owner_faction_for_tile(db: &PgPool, x: i32, y: i32) -> anyhow::Result<Option<Uuid>> {
    // Note: owner_faction_id is a nullable column ⇒ SQLx maps it to Option<Uuid>
    // fetch_optional wraps that in a second Option
    let opt = sqlx::query_scalar!(
        "SELECT owner_faction_id FROM land_parcels WHERE x = $1 AND y = $2",
        x,
        y
    )
    .fetch_optional(db)
    .await
    .context("fetching owner_faction_id for tile")?;

    // flatten Option<Option<Uuid>> → Option<Uuid>
    Ok(opt.flatten())
}

/// Inserts a new land parcel; returns the new parcel ID.
pub async fn insert_land_parcel(
    db: &PgPool,
    biome_type: &str,
    faction_id: Uuid,
    x: i32,
    y: i32,
) -> anyhow::Result<i32> {
    let id = sqlx::query_scalar!(
        r#"
        INSERT INTO land_parcels (biome_type, owner_faction_id, x, y)
        VALUES ($1, $2, $3, $4)
        RETURNING id
        "#,
        biome_type,
        faction_id,
        x,
        y
    )
    .fetch_one(db)
    .await
    .context("inserting land parcel")?;

    Ok(id)
}
