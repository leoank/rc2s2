//! The hub's record: SQLite in WAL mode through sqlx, schema in `migrations/`. Queries are
//! checked at run time, so building needs no database.

use std::path::Path;

use anyhow::{Context, Result};
use rc2s2_proto::event::v1::{
    Event, EventSource, HealthSample, Launch, Metric, PendingKind, PendingRequest, ReleaseReason,
    Session, SessionStatus,
};
use sqlx::Row;
use sqlx::sqlite::{
    SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqlitePoolOptions, SqliteRow,
};

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
    /// the session under another seq. True when it was new.
    pub async fn store_event(&self, node_id: &str, e: &Event) -> Result<bool> {
        let done = sqlx::query(
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
        Ok(done.rows_affected() == 1)
    }

    /// A session's stored events after a seq, oldest first, at most `limit`.
    pub async fn session_events(
        &self,
        node_id: &str,
        session_id: &str,
        after_seq: u64,
        limit: i64,
    ) -> Result<Vec<Event>> {
        let rows = sqlx::query(
            "SELECT seq, session_id, time_ms, source, kind, payload, line_id FROM events
             WHERE node_id = ?1 AND session_id = ?2 AND seq > ?3 ORDER BY seq LIMIT ?4",
        )
        .bind(node_id)
        .bind(session_id)
        .bind(after_seq as i64)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .iter()
            .map(|r| Event {
                session_id: r.get("session_id"),
                seq: r.get::<i64, _>("seq") as u64,
                time_ms: r.get("time_ms"),
                source: EventSource::from_str_name(r.get("source"))
                    .unwrap_or_default()
                    .into(),
                kind: r.get("kind"),
                payload_json: r.get("payload"),
                line_id: r.get::<Option<String>, _>("line_id").unwrap_or_default(),
            })
            .collect())
    }

    /// A node's health samples since a time, oldest first.
    pub async fn health_since(&self, node_id: &str, since_ms: i64) -> Result<Vec<HealthSample>> {
        let rows: Vec<(i64, String, f64)> = sqlx::query_as(
            "SELECT time_ms, metric, value FROM health WHERE node_id = ?1 AND time_ms >= ?2
             ORDER BY time_ms, metric",
        )
        .bind(node_id)
        .bind(since_ms)
        .fetch_all(&self.pool)
        .await?;
        let mut samples: Vec<HealthSample> = Vec::new();
        for (time_ms, name, value) in rows {
            if samples.last().is_none_or(|s| s.time_ms != time_ms) {
                samples.push(HealthSample {
                    time_ms,
                    metrics: Vec::new(),
                });
            }
            let last = samples.last_mut().expect("pushed above");
            last.metrics.push(Metric { name, value });
        }
        Ok(samples)
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

    pub async fn store_pending(&self, node_id: &str, p: &PendingRequest) -> Result<()> {
        sqlx::query(
            "INSERT INTO pending (node_id, id, session_id, kind, tool_name, tool_input, created_ms)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7) ON CONFLICT DO NOTHING",
        )
        .bind(node_id)
        .bind(&p.id)
        .bind(&p.session_id)
        .bind(p.kind().as_str_name())
        .bind(&p.tool_name)
        .bind(&p.tool_input_json)
        .bind(p.created_ms)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn close_pending(
        &self,
        node_id: &str,
        id: &str,
        reason: ReleaseReason,
        now_ms: i64,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE pending SET closed_ms = ?3, reason = ?4
             WHERE node_id = ?1 AND id = ?2 AND closed_ms IS NULL",
        )
        .bind(node_id)
        .bind(id)
        .bind(now_ms)
        .bind(reason.as_str_name())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// After a (re)connect the node's list of held requests is the truth: stores the ones the
    /// hub missed and closes the ones the node no longer holds, returning those ids. How they
    /// ended is unknown.
    pub async fn sync_pending(
        &self,
        node_id: &str,
        held: &[PendingRequest],
        now_ms: i64,
    ) -> Result<Vec<String>> {
        for p in held {
            self.store_pending(node_id, p).await?;
        }
        let open: Vec<String> =
            sqlx::query_scalar("SELECT id FROM pending WHERE node_id = ?1 AND closed_ms IS NULL")
                .bind(node_id)
                .fetch_all(&self.pool)
                .await?;
        let gone: Vec<String> = open
            .into_iter()
            .filter(|id| !held.iter().any(|p| &p.id == id))
            .collect();
        for id in &gone {
            self.close_pending(node_id, id, ReleaseReason::Unspecified, now_ms)
                .await?;
        }
        Ok(gone)
    }

    pub async fn record_resolution(&self, node_id: &str, id: &str, json: &str) -> Result<()> {
        sqlx::query("UPDATE pending SET resolution = ?3 WHERE node_id = ?1 AND id = ?2")
            .bind(node_id)
            .bind(id)
            .bind(json)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// The open request `id` on a node, if there is one.
    pub async fn open_pending(&self, node_id: &str, id: &str) -> Result<Option<PendingRequest>> {
        let row = sqlx::query(
            "SELECT id, session_id, kind, tool_name, tool_input, created_ms FROM pending
             WHERE node_id = ?1 AND id = ?2 AND closed_ms IS NULL",
        )
        .bind(node_id)
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|r| pending_from_row(&r)))
    }

    /// Every open request, oldest first, with its node id.
    pub async fn open_pendings(&self) -> Result<Vec<(String, PendingRequest)>> {
        let rows = sqlx::query(
            "SELECT node_id, id, session_id, kind, tool_name, tool_input, created_ms FROM pending
             WHERE closed_ms IS NULL ORDER BY created_ms",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .iter()
            .map(|r| (r.get("node_id"), pending_from_row(r)))
            .collect())
    }

    /// (id, name, version, last seen) of every node.
    pub async fn nodes(&self) -> Result<Vec<(String, String, String, i64)>> {
        Ok(
            sqlx::query_as("SELECT id, name, version, last_seen_ms FROM nodes ORDER BY name")
                .fetch_all(&self.pool)
                .await?,
        )
    }

    /// Sessions of one node, or of every node when `node_id` is empty.
    pub async fn sessions(&self, node_id: &str, include_ended: bool) -> Result<Vec<Session>> {
        let rows = sqlx::query(
            "SELECT * FROM sessions WHERE (?1 = '' OR node_id = ?1) AND (?2 OR ended_ms IS NULL)
             ORDER BY started_ms",
        )
        .bind(node_id)
        .bind(include_ended)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .iter()
            .map(|r| Session {
                id: r.get("id"),
                node_id: r.get("node_id"),
                harness: r.get("harness"),
                cwd: r.get("cwd"),
                transcript_path: r.get("transcript_path"),
                launch: Launch::from_str_name(r.get("launch"))
                    .unwrap_or_default()
                    .into(),
                pid: r.get::<i64, _>("pid") as u32,
                account_label: r.get("account_label"),
                started_ms: r.get("started_ms"),
                ended_ms: r.get("ended_ms"),
                status: SessionStatus::from_str_name(r.get("status"))
                    .unwrap_or_default()
                    .into(),
            })
            .collect())
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

fn pending_from_row(r: &SqliteRow) -> PendingRequest {
    PendingRequest {
        id: r.get("id"),
        session_id: r.get("session_id"),
        kind: PendingKind::from_str_name(r.get("kind"))
            .unwrap_or_default()
            .into(),
        tool_name: r.get("tool_name"),
        tool_input_json: r.get("tool_input"),
        created_ms: r.get("created_ms"),
    }
}
