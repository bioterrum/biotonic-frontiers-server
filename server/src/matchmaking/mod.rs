//! Background worker that pairs waiting players and notifies them.
//
//  Redis keys / channels
//  ---------------------
//  mm:queue                – ZSET  member = <player_id>, score = <elo> + ε(time)
//  player:<player_id>:events – PUB/SUB channel for one-off pushes (JSON)

use std::time::Duration;

use redis::{AsyncCommands, Client as RedisClient};
use serde::Serialize;
use sqlx::PgPool;
use tokio::time::sleep;
use uuid::Uuid;

#[derive(Serialize)]
struct MatchFound {
    game_id: Uuid,
    opponent_id: Uuid,
}

/// Spawn the infinite matchmaking loop as a Tokio task.
pub fn start(redis: RedisClient, db: PgPool) {
    tokio::spawn(async move {
        loop {
            if let Err(e) = tick(&redis, &db).await {
                log::error!("matchmaking tick failed: {e:?}");
            }
            sleep(Duration::from_secs(1)).await;
        }
    });
}

/// One “tick”: try to pop two players and, if successful, create a game row
/// and publish a `MatchFound` event to both private channels.
async fn tick(redis: &RedisClient, db: &PgPool) -> redis::RedisResult<()> {
    let mut conn = redis.get_multiplexed_async_connection().await?;

    // Prefer eldest pair with closest Elo (score = elo + 1e-6 * join-unix-ms)
    let pair: Vec<(String, f64)> = conn.zpopmin("mm:queue", 2).await?;
    if pair.len() == 2 {
        let p1 = Uuid::parse_str(&pair[0].0).expect("bad uuid p1");
        let p2 = Uuid::parse_str(&pair[1].0).expect("bad uuid p2");

        // Persist the match in `games` (state = Lobby)
        let game_id: Uuid = match sqlx::query_scalar!(
            r#"INSERT INTO games (player1_id, player2_id, state)
                VALUES ($1, $2, 'Lobby')
                RETURNING id"#,
            p1,
            p2
        )
        .fetch_one(db)
        .await
        {
            Ok(id) => id,
            Err(e) => {
                log::warn!("Could not create game for {p1} vs {p2}: {e}");
                // put the two players back in the queue so they aren’t lost
                let _: () = conn.zadd("mm:queue", p1.to_string(), pair[0].1).await?;
                let _: () = conn.zadd("mm:queue", p2.to_string(), pair[1].1).await?;
                return Ok(()); // continue with next tick
            }
        };

        let msg_p1 = serde_json::to_string(&MatchFound {
            game_id,
            opponent_id: p2,
        })
        .unwrap();
        let msg_p2 = serde_json::to_string(&MatchFound {
            game_id,
            opponent_id: p1,
        })
        .unwrap();

        let _: () = conn.publish(format!("player:{p1}:events"), msg_p1).await?;
        let _: () = conn.publish(format!("player:{p2}:events"), msg_p2).await?;
    }
    Ok(())
}
