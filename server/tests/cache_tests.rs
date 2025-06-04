// tests/cache_tests.rs

use biotonic_server::cache::{warm_all, get_item, ITEMS};
use dotenvy::dotenv;
use sqlx::PgPool;

#[tokio::test]
async fn test_warm_and_get_item() {
    // Load .env so DATABASE_URL is available
    dotenv().ok();

    // Clear any existing cache
    ITEMS.clear();

    // Connect to DB
    let database_url =
        std::env::var("DATABASE_URL").expect("DATABASE_URL must be set in .env for tests");
    let pool = PgPool::connect(&database_url)
        .await
        .expect("DB connection failed");

    // Insert or update test item
    let test_id: i32 = -42;
    sqlx::query!(
        r#"
        INSERT INTO items (id, name, description, base_price)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (id) DO UPDATE
          SET name        = EXCLUDED.name,
              description = EXCLUDED.description,
              base_price  = EXCLUDED.base_price
        "#,
        test_id,
        "test_item",
        Some("test_desc"),
        123,
    )
    .execute(&pool)
    .await
    .expect("insert test item");

    // Warm cache
    warm_all(&pool).await;

    // Retrieve from cache
    let item = get_item(test_id).expect("get_item should return our test item");
    assert_eq!(item.id, test_id);
    assert_eq!(&item.name, "test_item");
    assert_eq!(item.description.as_deref(), Some("test_desc"));
    assert_eq!(item.base_price, 123);

    // Clean up DB
    sqlx::query!("DELETE FROM items WHERE id = $1", test_id)
        .execute(&pool)
        .await
        .expect("delete test item");
}

#[tokio::test]
async fn test_get_missing_item() {
    // Load .env, though not strictly needed here
    dotenv().ok();

    // Clear cache to ensure empty state
    ITEMS.clear();

    // Attempt to retrieve nonexistent id
    let missing = get_item(-9999);
    assert!(missing.is_none(), "get_item should return None for missing id");
}
