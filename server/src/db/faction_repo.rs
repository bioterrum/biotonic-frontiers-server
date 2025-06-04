use anyhow::{anyhow, Context, Result};
use chrono::{Duration, Utc};
use sqlx::PgPool;
use uuid::Uuid;

/// Returns true if the given player belongs to the given faction.
pub async fn is_faction_member(db: &PgPool, faction: Uuid, player: Uuid) -> Result<bool> {
    Ok(sqlx::query_scalar!(
        r#"SELECT EXISTS(
               SELECT 1
                 FROM faction_members
                WHERE faction_id = $1
                  AND player_id  = $2
           )"#,
        faction,
        player
    )
    .fetch_one(db)
    .await
    .context("checking faction membership")?
    .unwrap_or(false))
}

/// Current role of a player inside a faction (if any).
pub async fn member_role(db: &PgPool, faction: Uuid, player: Uuid) -> Result<Option<String>> {
    sqlx::query_scalar!(
        "SELECT role FROM faction_members WHERE faction_id = $1 AND player_id = $2",
        faction,
        player
    )
    .fetch_optional(db)
    .await
    .context("fetching member role")
}

/// Promote a member one rank (member → officer).  Leader-only.
pub async fn promote_member(db: &PgPool, faction: Uuid, actor: Uuid, target: Uuid) -> Result<()> {
    // Ensure actor is the leader
    if member_role(db, faction, actor).await?.as_deref() != Some("leader") {
        anyhow::bail!("only leader may promote");
    }

    sqlx::query!(
        "UPDATE faction_members
            SET role = 'officer'
          WHERE faction_id = $1
            AND player_id  = $2
            AND role       = 'member'",
        faction,
        target
    )
    .execute(db)
    .await
    .context("promoting member")?;
    Ok(())
}

/// Demote an officer back to member.  Leader-only.
pub async fn demote_member(db: &PgPool, faction: Uuid, actor: Uuid, target: Uuid) -> Result<()> {
    if member_role(db, faction, actor).await?.as_deref() != Some("leader") {
        anyhow::bail!("only leader may demote");
    }

    sqlx::query!(
        "UPDATE faction_members
            SET role = 'member'
          WHERE faction_id = $1
            AND player_id  = $2
            AND role       = 'officer'",
        faction,
        target
    )
    .execute(db)
    .await
    .context("demoting member")?;
    Ok(())
}

/// Insert (or refresh) a pending invitation; expires in 3 days.
/// Only leader/officer may call.
pub async fn create_invite(db: &PgPool, faction: Uuid, inviter: Uuid, target: Uuid) -> Result<()> {
    let role = member_role(db, faction, inviter).await?;
    if !matches!(role.as_deref(), Some("leader" | "officer")) {
        return Err(anyhow!("insufficient privilege"));
    }

    sqlx::query!(
        r#"
        INSERT INTO faction_invites (faction_id, invited_player_id,
                                     invited_by, expires_at)
        VALUES ($1,$2,$3,$4)
        ON CONFLICT (faction_id, invited_player_id)
        DO UPDATE SET expires_at = EXCLUDED.expires_at,
                      invited_by = EXCLUDED.invited_by
        "#,
        faction,
        target,
        inviter,
        Utc::now() + Duration::days(3)
    )
    .execute(db)
    .await
    .context("creating invite")?;
    Ok(())
}

/// Accept an invite – adds member & deletes invite (transactional).
pub async fn accept_invite(db: &PgPool, invite_id: Uuid, player: Uuid) -> Result<()> {
    let mut tx = db.begin().await?;

    // Fetch the faction_id (or error if no such invite / expired)
    let fid: Uuid = sqlx::query_scalar!(
        "SELECT faction_id
           FROM faction_invites
          WHERE id = $1
            AND invited_player_id = $2
            AND expires_at > NOW()",
        invite_id,
        player
    )
    .fetch_one(&mut *tx)
    .await
    .context("invite not found or expired")?;

    // Insert into members
    sqlx::query!(
        r#"INSERT INTO faction_members (faction_id, player_id, role)
           VALUES ($1,$2,'member')
           ON CONFLICT DO NOTHING"#,
        fid,
        player
    )
    .execute(&mut *tx)
    .await?;

    // Delete the invite
    sqlx::query!("DELETE FROM faction_invites WHERE id = $1", invite_id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;
    Ok(())
}

/// Remove a member (kick). Leader OR officer may kick *members*.
/// Only leader may kick officers.
pub async fn kick_member(db: &PgPool, faction: Uuid, actor: Uuid, target: Uuid) -> Result<()> {
    let actor_role = member_role(db, faction, actor).await?;
    let target_role = member_role(db, faction, target).await?;

    match (actor_role.as_deref(), target_role.as_deref()) {
        (Some("leader"), Some("leader")) => {
            return Err(anyhow!("cannot kick yourself (leader)"));
        }
        (Some("leader"), _) => { /* leader can kick anyone else */ }
        (Some("officer"), Some("member")) => { /* ok */ }
        _ => return Err(anyhow!("insufficient privilege")),
    }

    let rows = sqlx::query!(
        "DELETE FROM faction_members
          WHERE faction_id = $1 AND player_id = $2",
        faction,
        target
    )
    .execute(db)
    .await?
    .rows_affected();

    if rows == 0 {
        Err(anyhow!("target not a member"))
    } else {
        Ok(())
    }
}
