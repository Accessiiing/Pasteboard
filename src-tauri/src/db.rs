use rusqlite::Connection;
use serde::Serialize;
use std::path::Path;

#[derive(Serialize, Clone)]
pub struct Record {
    pub id: i64,
    pub kind: String,
    pub content: String,
    pub preview: String,
    pub thumb_path: Option<String>,
    pub pinned: bool,
    pub created_at: i64,
    pub last_used_at: i64,
}

pub fn init(path: &Path) -> Connection {
    let conn = Connection::open(path).expect("无法打开数据库");
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS records (
            id           INTEGER PRIMARY KEY AUTOINCREMENT,
            kind         TEXT NOT NULL,
            content      TEXT NOT NULL,
            preview      TEXT NOT NULL,
            thumb_path   TEXT,
            pinned       INTEGER NOT NULL DEFAULT 0,
            created_at   INTEGER NOT NULL,
            last_used_at INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_order ON records(pinned DESC, last_used_at DESC);",
    )
    .expect("建表失败");
    conn
}

fn row_to_record(row: &rusqlite::Row) -> rusqlite::Result<Record> {
    Ok(Record {
        id: row.get(0)?,
        kind: row.get(1)?,
        content: row.get(2)?,
        preview: row.get(3)?,
        thumb_path: row.get(4)?,
        pinned: row.get::<_, i64>(5)? != 0,
        created_at: row.get(6)?,
        last_used_at: row.get(7)?,
    })
}

pub fn insert(
    conn: &Connection,
    kind: &str,
    content: &str,
    preview: &str,
    thumb_path: Option<&str>,
    now: i64,
) -> i64 {
    conn.execute(
        "INSERT INTO records (kind, content, preview, thumb_path, pinned, created_at, last_used_at)
         VALUES (?1, ?2, ?3, ?4, 0, ?5, ?5)",
        rusqlite::params![kind, content, preview, thumb_path, now],
    )
    .expect("插入失败");
    conn.last_insert_rowid()
}

pub fn list(conn: &Connection) -> Vec<Record> {
    let mut stmt = conn
        .prepare(
            "SELECT id, kind, content, preview, thumb_path, pinned, created_at, last_used_at
             FROM records ORDER BY pinned DESC, last_used_at DESC",
        )
        .unwrap();
    let rows = stmt.query_map([], row_to_record).unwrap();
    rows.filter_map(|r| r.ok()).collect()
}

pub fn get(conn: &Connection, id: i64) -> Option<Record> {
    conn.query_row(
        "SELECT id, kind, content, preview, thumb_path, pinned, created_at, last_used_at
         FROM records WHERE id = ?1",
        [id],
        row_to_record,
    )
    .ok()
}

pub fn touch(conn: &Connection, id: i64, now: i64) {
    let _ = conn.execute(
        "UPDATE records SET last_used_at = ?2 WHERE id = ?1",
        rusqlite::params![id, now],
    );
}

pub fn set_pinned(conn: &Connection, id: i64, pinned: bool) {
    let _ = conn.execute(
        "UPDATE records SET pinned = ?2 WHERE id = ?1",
        rusqlite::params![id, pinned as i64],
    );
}

/// 删除单条记录，返回其落盘文件路径（原图、缩略图）以便调用方清理。
pub fn delete(conn: &Connection, id: i64) -> Vec<String> {
    let mut files = Vec::new();
    if let Some(rec) = get(conn, id) {
        if rec.kind == "image" {
            files.push(rec.content);
        }
        if let Some(t) = rec.thumb_path {
            files.push(t);
        }
    }
    let _ = conn.execute("DELETE FROM records WHERE id = ?1", [id]);
    files
}

/// 超过上限时删除最旧的非固定记录，返回需清理的文件路径。
pub fn trim(conn: &Connection, max: u32) -> Vec<String> {
    let mut files = Vec::new();
    let count: u32 = conn
        .query_row("SELECT COUNT(*) FROM records", [], |r| r.get(0))
        .unwrap_or(0);
    if count <= max {
        return files;
    }
    let overflow = (count - max) as i64;
    let ids: Vec<i64> = {
        let mut stmt = conn
            .prepare(
                "SELECT id FROM records WHERE pinned = 0
                 ORDER BY last_used_at ASC LIMIT ?1",
            )
            .unwrap();
        stmt.query_map([overflow], |r| r.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect()
    };
    for id in ids {
        files.extend(delete(conn, id));
    }
    files
}

/// 清空记录。keep_pinned 为真时保留固定项。返回需清理的文件路径。
pub fn clear_all(conn: &Connection, keep_pinned: bool) -> Vec<String> {
    let where_clause = if keep_pinned { "WHERE pinned = 0" } else { "" };
    let sql_select = format!(
        "SELECT id, kind, content, preview, thumb_path, pinned, created_at, last_used_at
         FROM records {}",
        where_clause
    );
    let mut files = Vec::new();
    {
        let mut stmt = conn.prepare(&sql_select).unwrap();
        let recs = stmt
            .query_map([], row_to_record)
            .unwrap()
            .filter_map(|r| r.ok());
        for rec in recs {
            if rec.kind == "image" {
                files.push(rec.content);
            }
            if let Some(t) = rec.thumb_path {
                files.push(t);
            }
        }
    }
    let sql_del = format!("DELETE FROM records {}", where_clause);
    let _ = conn.execute(&sql_del, []);
    files
}
