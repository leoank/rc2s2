//! The hub's record: SQLite in WAL mode through sqlx, schema in `migrations/`. Queries are
//! checked at run time, so building needs no database.

use std::path::Path;

use anyhow::{Context, Result};
use rc2s2_proto::event::v1::{Event, HealthSample, Session};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqlitePoolOptions};

#[derive(Clone)]
pub struct Db {
    pool: SqlitePool,
}

impl Db {
    pub async fn open(path: &Path) -> Result<Self> {
        let options = SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal)
            .foreign_keys(true);
        let pool = SqlitePoolOptions::new()
            .connect_with(options)
            .await
            .with_context(|| format!("opening {}", path.display()))?;
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .context("migrating")?;
        Ok(Self { pool })
    }

    /// The node's id, created on first registration, and the highest seq stored for it.
    pub async fn register_node(
        &self,
        name: &str,
        version: &str,
        now_ms: i64,
    ) -> Result<(String, u64)> {
        let id: String = sqlx::query_scalar(
            "INSERT INTO nodes (id, name, version, first_seen_ms, last_seen_ms) VALUES (?1, ?2, ?3, ?4, ?4)
             ON CONFLICT (name) DO UPDATE SET version = excluded.version, last_seen_ms = excluded.last_seen_ms
             RETURNING id",
        )
        .bind(uuid::Uuid::new_v4().simple().to_string())
        .bind(name)
        .bind(version)
        .bind(now_ms)
        .fetch_one(&self.pool)
        .await?;
        let last: Option<i64> =
            sqlx::query_scalar("SELECT MAX(seq) FROM events WHERE node_id = ?1")
                .bind(&id)
                .fetch_one(&self.pool)
                .await?;
        Ok((id, last.unwrap_or(0) as u64))
    }

    pub async fn touch_node(&self, node_id: &str, now_ms: i64) -> Result<()> {
        sqlx::query("UPDATE nodes SET last_seen_ms = ?2 WHERE id = ?1")
            .bind(node_id)
            .bind(now_ms)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Stores an event unless the node sent it before: the same seq, or the same line of
    /// the session under another seq.
    pub async fn store_event(&self, node_id: &str, e: &Event) -> Result<()> {
        sqlx::query(
            "INSERT INTO events (node_id, seq, session_id, time_ms, source, kind, payload, line_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8) ON CONFLICT DO NOTHING",
        )
        .bind(node_id)
        .bind(e.seq as i64)
        .bind(&e.session_id)
        .bind(e.time_ms)
        .bind(e.source().as_str_name())
        .bind(&e.kind)
        .bind(&e.payload_json)
        .bind((!e.line_id.is_empty()).then_some(&e.line_id))
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn store_session(&self, node_id: &str, s: &Session) -> Result<()> {
        sqlx::query(
            "INSERT INTO sessions (node_id, id, harness, cwd, transcript_path, launch, pid, account_label, status,
                                   started_ms, ended_ms)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
             ON CONFLICT (node_id, id) DO UPDATE SET
               harness = excluded.harness, cwd = excluded.cwd, transcript_path = excluded.transcript_path,
               launch = excluded.launch, pid = excluded.pid, account_label = excluded.account_label,
               status = excluded.status, started_ms = excluded.started_ms, ended_ms = excluded.ended_ms",
        )
        .bind(node_id)
        .bind(&s.id)
        .bind(&s.harness)
        .bind(&s.cwd)
        .bind(&s.transcript_path)
        .bind(s.launch().as_str_name())
        .bind(s.pid as i64)
        .bind(&s.account_label)
        .bind(s.status().as_str_name())
        .bind(s.started_ms)
        .bind(s.ended_ms)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn store_health(&self, node_id: &str, h: &HealthSample) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        for m in &h.metrics {
            sqlx::query(
                "INSERT INTO health (node_id, time_ms, metric, value) VALUES (?1, ?2, ?3, ?4)",
            )
            .bind(node_id)
            .bind(h.time_ms)
            .bind(&m.name)
            .bind(m.value)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    /// Every stored seq for a node, in order.
    pub async fn event_seqs(&self, node_name: &str) -> Result<Vec<u64>> {
        let seqs: Vec<i64> = sqlx::query_scalar(
            "SELECT seq FROM events JOIN nodes ON nodes.id = events.node_id WHERE nodes.name = ?1 ORDER BY seq",
        )
        .bind(node_name)
        .fetch_all(&self.pool)
        .await?;
        Ok(seqs.into_iter().map(|s| s as u64).collect())
    }

    /// Session ids stored for a node.
    pub async fn session_ids(&self, node_name: &str) -> Result<Vec<String>> {
        Ok(sqlx::query_scalar(
            "SELECT sessions.id FROM sessions JOIN nodes ON nodes.id = sessions.node_id WHERE nodes.name = ?1 ORDER BY 1",
        )
        .bind(node_name)
        .fetch_all(&self.pool)
        .await?)
    }
}
