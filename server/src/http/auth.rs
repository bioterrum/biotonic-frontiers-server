//! Magic-link authentication (JWT + refresh)

use actix_web::{get, post, web, HttpResponse, Responder};
use chrono::{Duration, Utc};
use jsonwebtoken::{encode, EncodingKey, Header};
use redis::{AsyncCommands, Client as RedisClient};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::env;
use uuid::Uuid;

use crate::config::settings;

//////////////////////////////////////////////////
// Data structs
//////////////////////////////////////////////////

#[derive(Deserialize)]
pub struct MagicLinkRequest {
    pub email: String,
}

#[derive(Deserialize)]
pub struct VerifyQuery {
    pub token: String,
}

#[derive(Deserialize)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String, // user_id
    pid: String, // player_id
    exp: usize,
}

#[derive(Serialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: i64,
}

//////////////////////////////////////////////////
// ─────────────  JwtAuth extractor  ─────────────
//////////////////////////////////////////////////

pub mod extractor {
    use super::Claims;
    use actix_web::{
        dev::Payload, error::ErrorUnauthorized, FromRequest, HttpRequest, Result as ActixResult,
    };
    use futures_util::future::{ready, Ready};
    use jsonwebtoken::{decode, DecodingKey, Validation};
    use std::env;
    use uuid::Uuid;

    /// Extracts and validates a Bearer-JWT, exposing user & player UUIDs.
    #[derive(Debug, Clone)]
    pub struct JwtAuth {
        pub user_id: Uuid,
        pub player_id: Uuid,
    }

    impl FromRequest for JwtAuth {
        type Error = actix_web::Error;
        type Future = Ready<ActixResult<Self, Self::Error>>;

        fn from_request(req: &HttpRequest, _pl: &mut Payload) -> Self::Future {
            let res = (|| {
                // Expect:  Authorization: Bearer <JWT>
                let hdr = req
                    .headers()
                    .get("Authorization")
                    .and_then(|v| v.to_str().ok())
                    .ok_or_else(|| ErrorUnauthorized("missing Authorization header"))?;

                let token = hdr
                    .strip_prefix("Bearer ")
                    .ok_or_else(|| ErrorUnauthorized("malformed Authorization header"))?;

                let secret =
                    env::var("JWT_SECRET").map_err(|_| ErrorUnauthorized("server mis-config"))?;
                let data = decode::<Claims>(
                    token,
                    &DecodingKey::from_secret(secret.as_bytes()),
                    &Validation::default(),
                )
                .map_err(|_| ErrorUnauthorized("invalid / expired token"))?;

                let user_id =
                    Uuid::parse_str(&data.claims.sub).map_err(|_| ErrorUnauthorized("bad sub"))?;
                let player_id =
                    Uuid::parse_str(&data.claims.pid).map_err(|_| ErrorUnauthorized("bad pid"))?;

                Ok(JwtAuth { user_id, player_id })
            })();

            ready(res)
        }
    }
}
pub use extractor::JwtAuth; // <-- makes path crate::http::auth::JwtAuth work

//////////////////////////////////////////////////
// POST /api/magic_link
//////////////////////////////////////////////////
#[post("/magic_link")]
pub async fn magic_link(
    info: web::Json<MagicLinkRequest>,
    redis: web::Data<RedisClient>,
) -> impl Responder {
    let token = Uuid::new_v4().to_string();
    if let Ok(mut conn) = redis.get_multiplexed_async_connection().await {
        let _: () = conn
            .set_ex(&token, &info.email, 15 * 60)
            .await
            .unwrap_or(());
    } else {
        return HttpResponse::InternalServerError().body("Redis unavailable");
    }
    log::info!(
        "Magic link for {}:\n  https://your-domain.com/api/verify?token={}",
        info.email,
        token
    );
    HttpResponse::Ok().body("Magic link sent; check your email")
}

//////////////////////////////////////////////////
// GET /api/verify
//////////////////////////////////////////////////
#[get("/verify")]
pub async fn verify(
    query: web::Query<VerifyQuery>,
    redis: web::Data<RedisClient>,
    db: web::Data<PgPool>,
) -> impl Responder {
    // 1) resolve token → email
    let email = match redis.get_multiplexed_async_connection().await {
        Ok(mut conn) => {
            if let Ok(Some(e)) = conn.get::<_, Option<String>>(&query.token).await {
                let _: () = conn.del(&query.token).await.unwrap_or(());
                e
            } else {
                return HttpResponse::BadRequest().body("Invalid or expired token");
            }
        }
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };

    // 2) upsert user
    let user_id: Uuid = sqlx::query_scalar!(
        r#"INSERT INTO users (email)
           VALUES ($1)
           ON CONFLICT (email) DO UPDATE SET email = EXCLUDED.email
           RETURNING id"#,
        email
    )
    .fetch_one(&**db)
    .await
    .unwrap();

    // 3) upsert / fetch player
    let player_id: Uuid =
        match sqlx::query_scalar!("SELECT id FROM players WHERE user_id = $1", user_id)
            .fetch_optional(&**db)
            .await
            .unwrap()
        {
            Some(pid) => pid,
            None => {
                let nickname = email.split('@').next().unwrap_or("player");
                sqlx::query_scalar!(
                    r#"INSERT INTO players (user_id, nickname)
                   VALUES ($1, $2)
                   RETURNING id"#,
                    user_id,
                    nickname
                )
                .fetch_one(&**db)
                .await
                .unwrap()
            }
        };

    // 4) presence key
    if let Ok(mut conn) = redis.get_multiplexed_async_connection().await {
        let key = format!("session:{player_id}");
        // pass u64 directly
        let _: () = conn
            .set_ex(&key, "1", settings().presence_ttl)
            .await
            .unwrap_or(());
    }

    // 5) issue JWT
    let secret = env::var("JWT_SECRET").expect("JWT_SECRET must be set");
    let exp = Utc::now()
        .checked_add_signed(Duration::minutes(15))
        .unwrap()
        .timestamp() as usize;
    let claims = Claims {
        sub: user_id.to_string(),
        pid: player_id.to_string(),
        exp,
    };
    let access_token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .expect("JWT encode failed");

    // 6) store refresh token (30 days)
    let refresh_token = Uuid::new_v4().to_string();
    if let Ok(mut conn) = redis.get_multiplexed_async_connection().await {
        let key = format!("refresh:{refresh_token}");
        let _: () = conn
            .set_ex(&key, user_id.to_string(), 30 * 24 * 3_600)
            .await
            .unwrap_or(());
    }

    HttpResponse::Ok().json(TokenResponse {
        access_token,
        refresh_token,
        expires_in: 15 * 60,
    })
}

//////////////////////////////////////////////////
// POST /api/refresh
//////////////////////////////////////////////////
#[post("/refresh")]
pub async fn refresh(
    info: web::Json<RefreshRequest>,
    redis: web::Data<RedisClient>,
    db: web::Data<PgPool>,
) -> impl Responder {
    // 1) consume old refresh → user_id
    let user_id_str = match redis.get_multiplexed_async_connection().await {
        Ok(mut conn) => {
            let key = format!("refresh:{}", info.refresh_token);
            if let Ok(Some(uid)) = conn.get::<_, Option<String>>(&key).await {
                let _: () = conn.del(&key).await.unwrap_or(());
                uid
            } else {
                return HttpResponse::Unauthorized().body("invalid refresh");
            }
        }
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };
    let user_id = Uuid::parse_str(&user_id_str).unwrap();

    // 2) player_id lookup
    let player_id: Uuid = sqlx::query_scalar!("SELECT id FROM players WHERE user_id = $1", user_id)
        .fetch_one(&**db)
        .await
        .unwrap();

    // 3) refresh presence TTL
    if let Ok(mut conn) = redis.get_multiplexed_async_connection().await {
        let key = format!("session:{player_id}");
        // pass u64 directly
        let _: () = conn
            .set_ex(&key, "1", settings().presence_ttl)
            .await
            .unwrap_or(());
    }

    // 4) new access token
    let secret = env::var("JWT_SECRET").expect("JWT_SECRET must be set");
    let exp = Utc::now()
        .checked_add_signed(Duration::minutes(15))
        .unwrap()
        .timestamp() as usize;
    let claims = Claims {
        sub: user_id.to_string(),
        pid: player_id.to_string(),
        exp,
    };
    let access_token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .expect("JWT encode failed");

    // 5) mint new refresh
    let new_refresh = Uuid::new_v4().to_string();
    if let Ok(mut conn) = redis.get_multiplexed_async_connection().await {
        let key = format!("refresh:{new_refresh}");
        let _: () = conn
            .set_ex(&key, user_id.to_string(), 30 * 24 * 3_600)
            .await
            .unwrap_or(());
    }

    HttpResponse::Ok().json(TokenResponse {
        access_token,
        refresh_token: new_refresh,
        expires_in: 15 * 60,
    })
}

//////////////////////////////////////////////////
// Mount
//////////////////////////////////////////////////
pub fn init_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(magic_link).service(verify).service(refresh);
}
