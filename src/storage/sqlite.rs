use std::path::Path;
use rusqlite::{Connection, params, types::ToSql};
use uuid::Uuid;

pub struct Database {
    conn: Connection,
}

pub struct ChatRow {
    pub id: String,
    pub session_id: String,
    pub role: String,
    pub content: String,
    pub created_at: String,
}

pub struct SkillRow {
    pub id: String,
    pub name: String,
    pub definition: String,
    pub embedding: Option<Vec<u8>>,
    pub success_rate: f64,
    pub usage_count: u32,
    pub created_at: String,
}

pub struct MemoryRow {
    pub id: String,
    pub content: String,
    pub embedding: Option<Vec<u8>>,
    pub source: Option<String>,
    pub created_at: String,
}

macro_rules! q_map {
    ($stmt:expr, $params:expr, $row:ident, $body:block) => {{
        let rows = $stmt.query_map($params, |$row| {
            Ok($body)
        })?;
        rows.filter_map(|r| r.ok()).collect()
    }};
}

impl Database {
    pub fn open(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        std::fs::create_dir_all(
            path.as_ref().parent().unwrap_or(Path::new("."))
        )?;
        let conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
        let db = Self { conn };
        db.migrate()?;
        Ok(db)
    }

    pub fn open_in_memory() -> anyhow::Result<Self> {
        let conn = Connection::open_in_memory()?;
        let db = Self { conn };
        db.migrate()?;
        Ok(db)
    }

    fn migrate(&self) -> anyhow::Result<()> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS sessions (
                id TEXT PRIMARY KEY,
                name TEXT,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now'))
            );

            CREATE TABLE IF NOT EXISTS chats (
                id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS skills (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                definition TEXT NOT NULL,
                embedding BLOB,
                success_rate REAL DEFAULT 0.0,
                usage_count INTEGER DEFAULT 0,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );

            CREATE TABLE IF NOT EXISTS memories (
                id TEXT PRIMARY KEY,
                content TEXT NOT NULL,
                embedding BLOB,
                source TEXT,
                importance REAL DEFAULT 0.5,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );

            CREATE INDEX IF NOT EXISTS idx_chats_session ON chats(session_id);
            CREATE INDEX IF NOT EXISTS idx_skills_name ON skills(name);
            CREATE INDEX IF NOT EXISTS idx_memories_importance ON memories(importance DESC);
            "
        )?;
        Ok(())
    }

    // ── Sessions ──

    pub fn create_session(&self) -> anyhow::Result<String> {
        let id = Uuid::new_v4().to_string();
        self.conn.execute(
            "INSERT INTO sessions (id) VALUES (?1)",
            params![id],
        )?;
        Ok(id)
    }

    pub fn list_sessions(&self) -> anyhow::Result<Vec<(String, String, String)>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, COALESCE(name, id), updated_at FROM sessions ORDER BY updated_at DESC"
        )?;
        let rows = stmt.query_map(params![], |row| -> rusqlite::Result<(String, String, String)> {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
            ))
        })?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    pub fn delete_session(&self, session_id: &str) -> anyhow::Result<()> {
        self.conn.execute("DELETE FROM chats WHERE session_id = ?1", params![session_id])?;
        self.conn.execute("DELETE FROM sessions WHERE id = ?1", params![session_id])?;
        Ok(())
    }

    pub fn touch_session(&self, session_id: &str) -> anyhow::Result<()> {
        self.conn.execute(
            "UPDATE sessions SET updated_at = datetime('now') WHERE id = ?1",
            params![session_id],
        )?;
        Ok(())
    }

    // ── Chats ──

    pub fn insert_chat(&self, session_id: &str, role: &str, content: &str) -> anyhow::Result<String> {
        let id = Uuid::new_v4().to_string();
        self.conn.execute(
            "INSERT INTO chats (id, session_id, role, content) VALUES (?1, ?2, ?3, ?4)",
            params![id, session_id, role, content],
        )?;
        self.touch_session(session_id)?;
        Ok(id)
    }

    pub fn load_chats(&self, session_id: &str) -> anyhow::Result<Vec<ChatRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, session_id, role, content, created_at FROM chats WHERE session_id = ?1 ORDER BY created_at ASC"
        )?;
        let rows = stmt.query_map(params![session_id], |row| -> rusqlite::Result<ChatRow> {
            Ok(ChatRow {
                id: row.get(0)?,
                session_id: row.get(1)?,
                role: row.get(2)?,
                content: row.get(3)?,
                created_at: row.get(4)?,
            })
        })?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    pub fn count_chats(&self, session_id: &str) -> anyhow::Result<usize> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM chats WHERE session_id = ?1",
            params![session_id],
            |row| row.get(0),
        )?;
        Ok(count as usize)
    }

    // ── Skills ──

    pub fn upsert_skill(&self, name: &str, definition: &str, embedding: Option<&[u8]>) -> anyhow::Result<String> {
        let existing: Option<(String, u32)> = self.conn.query_row(
            "SELECT id, usage_count FROM skills WHERE name = ?1",
            params![name],
            |row| Ok((row.get(0)?, row.get(1)?)),
        ).ok();

        if let Some((id, count)) = existing {
            self.conn.execute(
                "UPDATE skills SET definition = ?1, embedding = ?2, usage_count = ?3 WHERE id = ?4",
                params![definition, embedding, count + 1, id],
            )?;
            Ok(id)
        } else {
            let id = Uuid::new_v4().to_string();
            self.conn.execute(
                "INSERT INTO skills (id, name, definition, embedding) VALUES (?1, ?2, ?3, ?4)",
                params![id, name, definition, embedding],
            )?;
            Ok(id)
        }
    }

    pub fn list_skills(&self) -> anyhow::Result<Vec<SkillRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, definition, embedding, success_rate, usage_count, created_at FROM skills ORDER BY usage_count DESC"
        )?;
        let rows = stmt.query_map(params![], |row| -> rusqlite::Result<SkillRow> {
            Ok(SkillRow {
                id: row.get(0)?,
                name: row.get(1)?,
                definition: row.get(2)?,
                embedding: row.get(3)?,
                success_rate: row.get(4)?,
                usage_count: row.get(5)?,
                created_at: row.get(6)?,
            })
        })?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    pub fn find_skill_by_name(&self, name: &str) -> anyhow::Result<Option<SkillRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, definition, embedding, success_rate, usage_count, created_at FROM skills WHERE name = ?1"
        )?;
        let mut rows = stmt.query_map(params![name], |row| -> rusqlite::Result<SkillRow> {
            Ok(SkillRow {
                id: row.get(0)?,
                name: row.get(1)?,
                definition: row.get(2)?,
                embedding: row.get(3)?,
                success_rate: row.get(4)?,
                usage_count: row.get(5)?,
                created_at: row.get(6)?,
            })
        })?;
        match rows.next() {
            Some(Ok(r)) => Ok(Some(r)),
            Some(Err(_)) => Ok(None),
            None => Ok(None),
        }
    }

    // ── Memories ──

    pub fn insert_memory(&self, content: &str, embedding: Option<&[u8]>, source: Option<&str>, importance: f64) -> anyhow::Result<String> {
        let id = Uuid::new_v4().to_string();
        self.conn.execute(
            "INSERT INTO memories (id, content, embedding, source, importance) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![id, content, embedding, source, importance],
        )?;
        Ok(id)
    }

    pub fn search_memories(&self, query_keywords: &[&str], limit: usize) -> anyhow::Result<Vec<MemoryRow>> {
        let mut conditions = Vec::new();
        for kw in query_keywords {
            conditions.push(format!("content LIKE '%{}%'", kw.replace('\'', "''")));
        }
        let where_clause = if conditions.is_empty() {
            "1=1".to_string()
        } else {
            conditions.join(" OR ")
        };

        let sql = format!(
            "SELECT id, content, embedding, source, created_at FROM memories WHERE {} ORDER BY importance DESC LIMIT ?1",
            where_clause
        );

        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(params![limit as i64], |row| -> rusqlite::Result<MemoryRow> {
            Ok(MemoryRow {
                id: row.get(0)?,
                content: row.get(1)?,
                embedding: row.get(2)?,
                source: row.get(3)?,
                created_at: row.get(4)?,
            })
        })?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    // ── Raw access for embeddings ──

    pub fn all_skills_with_embeddings(&self) -> anyhow::Result<Vec<(String, Vec<u8>)>> {
        let mut stmt = self.conn.prepare(
            "SELECT name, embedding FROM skills WHERE embedding IS NOT NULL"
        )?;
        let rows = stmt.query_map(params![], |row| -> rusqlite::Result<(String, Vec<u8>)> {
            Ok((row.get(0)?, row.get(1)?))
        })?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    pub fn inner(&self) -> &Connection {
        &self.conn
    }
}
