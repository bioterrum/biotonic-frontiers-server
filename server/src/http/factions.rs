//! Faction management (create / join / leave / list / promote / demote / info)

use actix_web::{get, post, web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::db::faction_repo;

//////////////////////////////////////////////////
// Data transfer objects
//////////////////////////////////////////////////

#[derive(Serialize)]
pub struct FactionRow {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub logo_url: Option<String>,
    pub member_count: i64,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Serialize)]
pub struct MemberRow {
    pub player_id: Uuid,
    pub nickname: String,
    pub role: String,
}

#[derive(Serialize)]
pub struct FactionInfo {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub logo_url: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub members: Vec<MemberRow>,
}

//////////////////////////////////////////////////
// Requests
//////////////////////////////////////////////////

#[derive(Deserialize)]
pub struct CreateReq {
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub founder_id: Uuid,
}

#[derive(Deserialize)]
pub struct JoinReq {
    pub faction_id: Uuid,
    pub player_id: Uuid,
}

#[derive(Deserialize)]
pub struct LeaveReq {
    pub faction_id: Uuid,
    pub player_id: Uuid,
}

#[derive(Deserialize)]
pub struct PromoteReq {
    pub faction_id: Uuid,
    pub actor_id: Uuid,
    pub target_id: Uuid,
}

#[derive(Deserialize)]
pub struct DemoteReq {
    pub faction_id: Uuid,
    pub actor_id: Uuid,
    pub target_id: Uuid,
}

//////////////////////////////////////////////////
// Handlers
//////////////////////////////////////////////////

/// POST /api/factions/create
#[post("/factions/create")]
pub async fn create(info: web::Json<CreateReq>, db: web::Data<PgPool>) -> impl Responder {
    let mut tx = match db.begin().await {
        Ok(t) => t,
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };

    let fid: Uuid = match sqlx::query_scalar!(
        r#"INSERT INTO factions (name, description)
           VALUES ($1,$2)
           RETURNING id"#,
        info.name,
        info.description
    )
    .fetch_one(&mut *tx)
    .await
    {
        Ok(id) => id,
        Err(sqlx::Error::Database(db_err)) if db_err.code() == Some("23505".into()) => {
            return HttpResponse::BadRequest().body("name already taken")
        }
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };

    let _ = sqlx::query!(
        "INSERT INTO faction_members (faction_id, player_id, role)
         VALUES ($1, $2, 'leader')",
        fid,
        info.founder_id,
    )
    .execute(&mut *tx)
    .await;

    tx.commit().await.ok();
    HttpResponse::Ok().json(serde_json::json!({ "faction_id": fid }))
}

/// GET /api/factions/list
#[get("/factions/list")]
pub async fn list(db: web::Data<PgPool>) -> impl Responder {
    let rows = sqlx::query_as!(
        FactionRow,
        r#"
        SELECT f.id, f.name, f.description, f.logo_url,
               COUNT(m.player_id) AS "member_count!",
               f.created_at
          FROM factions f
          LEFT JOIN faction_members m ON m.faction_id = f.id
         GROUP BY f.id
         ORDER BY f.created_at
        "#
    )
    .fetch_all(&**db)
    .await
    .unwrap_or_default();

    HttpResponse::Ok().json(rows)
}

/// GET /api/factions/of/{player_id}
#[get("/factions/of/{player_id}")]
pub async fn faction_of(path: web::Path<Uuid>, db: web::Data<PgPool>) -> impl Responder {
    let pid = path.into_inner();

    // Which faction?
    let fid = match sqlx::query_scalar!(
        "SELECT faction_id FROM faction_members WHERE player_id = $1",
        pid
    )
    .fetch_optional(&**db)
    .await
    .unwrap_or(None)
    {
        Some(id) => id,
        None => return HttpResponse::Ok().body("none"),
    };

    // Info + members
    let (name, description, logo_url, created_at) =
        sqlx::query_as::<
            _,
            (
                String,
                String,
                Option<String>,
                chrono::DateTime<chrono::Utc>,
            ),
        >("SELECT name, description, logo_url, created_at FROM factions WHERE id = $1")
        .bind(fid)
        .fetch_one(&**db)
        .await
        .unwrap();

    let members = sqlx::query!(
        r#"SELECT fm.player_id, p.nickname, fm.role
           FROM faction_members fm
           JOIN players p ON p.id = fm.player_id
           WHERE fm.faction_id = $1
           ORDER BY fm.role DESC, p.nickname"#,
        fid
    )
    .fetch_all(&**db)
    .await
    .unwrap()
    .into_iter()
    .map(|r| MemberRow {
        player_id: r.player_id,
        nickname: r.nickname,
        role: r.role,
    })
    .collect();

    HttpResponse::Ok().json(FactionInfo {
        id: fid,
        name,
        description,
        logo_url,
        created_at,
        members,
    })
}

/// POST /api/factions/join
#[post("/factions/join")]
pub async fn join(info: web::Json<JoinReq>, db: web::Data<PgPool>) -> impl Responder {
    match sqlx::query!(
        r#"INSERT INTO faction_members (faction_id, player_id, role)
            VALUES ($1,$2,'member')
            ON CONFLICT DO NOTHING"#,
        info.faction_id,
        info.player_id
    )
    .execute(&**db)
    .await
    {
        Ok(_) => HttpResponse::Ok().body("joined"),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

/// POST /api/factions/leave
#[post("/factions/leave")]
pub async fn leave(info: web::Json<LeaveReq>, db: web::Data<PgPool>) -> impl Responder {
    let rows = sqlx::query!(
        "DELETE FROM faction_members
         WHERE faction_id = $1 AND player_id = $2",
        info.faction_id,
        info.player_id
    )
    .execute(&**db)
    .await
    .map(|r| r.rows_affected())
    .unwrap_or(0);

    if rows == 0 {
        HttpResponse::BadRequest().body("not a member")
    } else {
        HttpResponse::Ok().body("left")
    }
}

/// POST /api/factions/promote
#[post("/factions/promote")]
pub async fn promote(info: web::Json<PromoteReq>, db: web::Data<PgPool>) -> impl Responder {
    match faction_repo::promote_member(db.get_ref(), info.faction_id, info.actor_id, info.target_id)
        .await
    {
        Ok(_) => HttpResponse::Ok().body("promoted"),
        Err(e) => {
            log::warn!("promote failed: {e:?}");
            HttpResponse::BadRequest().body(e.to_string())
        }
    }
}

/// POST /api/factions/demote
#[post("/factions/demote")]
pub async fn demote(info: web::Json<DemoteReq>, db: web::Data<PgPool>) -> impl Responder {
    match faction_repo::demote_member(db.get_ref(), info.faction_id, info.actor_id, info.target_id)
        .await
    {
        Ok(_) => HttpResponse::Ok().body("demoted"),
        Err(e) => {
            log::warn!("demote failed: {e:?}");
            HttpResponse::BadRequest().body(e.to_string())
        }
    }
}

//////////////////////////////////////////////////
// Mount
//////////////////////////////////////////////////
pub fn init_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(create)
        .service(list)
        .service(join)
        .service(leave)
        .service(promote)
        .service(demote)
        .service(faction_of)
        .service(invite)
        .service(accept)
        .service(kick);
}

// ---------- Requests ----------
#[derive(Deserialize)]
pub struct InviteReq {
    pub faction_id: Uuid,
    pub inviter_id: Uuid,
    pub target_player_id: Uuid,
}

#[derive(Deserialize)]
pub struct AcceptReq {
    pub invite_id: Uuid,
    pub player_id: Uuid,
}

#[derive(Deserialize)]
pub struct KickReq {
    pub faction_id: Uuid,
    pub actor_id: Uuid,
    pub target_id: Uuid,
}

// ---------- Invite ----------
#[post("/factions/invite")]
pub async fn invite(info: web::Json<InviteReq>, db: web::Data<PgPool>) -> impl Responder {
    match faction_repo::create_invite(
        db.get_ref(),
        info.faction_id,
        info.inviter_id,
        info.target_player_id,
    )
    .await
    {
        Ok(_) => HttpResponse::Ok().body("invited"),
        Err(e) => HttpResponse::BadRequest().body(e.to_string()),
    }
}

// ---------- Accept ----------
#[post("/factions/invites/accept")]
pub async fn accept(info: web::Json<AcceptReq>, db: web::Data<PgPool>) -> impl Responder {
    match faction_repo::accept_invite(db.get_ref(), info.invite_id, info.player_id).await {
        Ok(_) => HttpResponse::Ok().body("joined"),
        Err(e) => HttpResponse::BadRequest().body(e.to_string()),
    }
}

// ---------- Kick ----------
#[post("/factions/kick")]
pub async fn kick(info: web::Json<KickReq>, db: web::Data<PgPool>) -> impl Responder {
    match faction_repo::kick_member(db.get_ref(), info.faction_id, info.actor_id, info.target_id)
        .await
    {
        Ok(_) => HttpResponse::Ok().body("kicked"),
        Err(e) => HttpResponse::BadRequest().body(e.to_string()),
    }
}
