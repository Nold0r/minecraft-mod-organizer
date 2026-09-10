use crate::domain::category::Category;
use rusqlite::{params, Connection, OptionalExtension, Transaction};
use std::{collections::HashMap, path::PathBuf};

#[derive(Clone)]
pub struct SqliteRepository {
    path: PathBuf,
}

impl SqliteRepository {
    pub fn new(path: PathBuf) -> Self { Self { path } }

    fn conn(&self) -> rusqlite::Result<Connection> {
        let conn = Connection::open(&self.path)?;
        conn.execute_batch("PRAGMA foreign_keys = ON;")?;
        Ok(conn)
    }

    pub fn init(&self) -> rusqlite::Result<()> {
        let conn = self.conn()?;
        conn.execute_batch(r#"
            CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS categories (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                parent_id TEXT NULL REFERENCES categories(id) ON DELETE CASCADE,
                sort_index INTEGER NOT NULL DEFAULT 0,
                color TEXT NULL,
                icon_hex TEXT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_categories_parent_sort
                ON categories(parent_id, sort_index);
            CREATE TABLE IF NOT EXISTS mod_layout (
                layout_key TEXT PRIMARY KEY,
                category_id TEXT NULL REFERENCES categories(id) ON DELETE SET NULL,
                sort_index INTEGER NOT NULL DEFAULT 0
            );
            CREATE INDEX IF NOT EXISTS idx_mod_layout_category_sort
                ON mod_layout(category_id, sort_index);
        "#)?;

        // Migrate databases created by older builds.
        if !has_column(&conn, "categories", "color")? {
            conn.execute("ALTER TABLE categories ADD COLUMN color TEXT NULL", [])?;
        }
        if !has_column(&conn, "categories", "icon_hex")? {
            conn.execute("ALTER TABLE categories ADD COLUMN icon_hex TEXT NULL", [])?;
        }
        Ok(())
    }

    pub fn get_setting(&self, key: &str) -> rusqlite::Result<Option<String>> {
        self.conn()?.query_row(
            "SELECT value FROM settings WHERE key=?1",
            params![key],
            |row| row.get(0),
        ).optional()
    }

    pub fn set_setting(&self, key: &str, value: &str) -> rusqlite::Result<()> {
        self.conn()?.execute(
            "INSERT INTO settings(key,value) VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value",
            params![key, value],
        )?;
        Ok(())
    }

    pub fn list_categories(&self) -> rusqlite::Result<Vec<Category>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT id,name,parent_id,sort_index,color,icon_hex FROM categories ORDER BY sort_index,name"
        )?;
        stmt.query_map([], |row| Ok(Category {
            id: row.get(0)?,
            name: row.get(1)?,
            parent_id: row.get(2)?,
            sort_index: row.get(3)?,
            color: row.get(4)?,
            icon_hex: row.get(5)?,
        }))?.collect()
    }

    pub fn get_category(&self, id: &str) -> rusqlite::Result<Option<Category>> {
        self.conn()?.query_row(
            "SELECT id,name,parent_id,sort_index,color,icon_hex FROM categories WHERE id=?1",
            params![id],
            |row| Ok(Category {
                id: row.get(0)?,
                name: row.get(1)?,
                parent_id: row.get(2)?,
                sort_index: row.get(3)?,
                color: row.get(4)?,
                icon_hex: row.get(5)?,
            })
        ).optional()
    }

    pub fn count_category_siblings(&self, parent_id: Option<&str>) -> rusqlite::Result<i64> {
        let conn = self.conn()?;
        match parent_id {
            Some(p) => conn.query_row("SELECT COUNT(*) FROM categories WHERE parent_id=?1", params![p], |r| r.get(0)),
            None => conn.query_row("SELECT COUNT(*) FROM categories WHERE parent_id IS NULL", [], |r| r.get(0)),
        }
    }

    pub fn insert_category(&self, category: &Category) -> rusqlite::Result<()> {
        self.conn()?.execute(
            "INSERT INTO categories(id,name,parent_id,sort_index,color,icon_hex) VALUES(?1,?2,?3,?4,?5,?6)",
            params![
                category.id,
                category.name,
                category.parent_id,
                category.sort_index,
                category.color,
                category.icon_hex,
            ]
        )?;
        Ok(())
    }

    pub fn delete_category(&self, id: &str) -> rusqlite::Result<()> {
        let mut conn = self.conn()?;
        let tx = conn.transaction()?;
        let old_parent: Option<String> = tx.query_row(
            "SELECT parent_id FROM categories WHERE id=?1",
            params![id],
            |r| r.get(0),
        )?;
        tx.execute("DELETE FROM categories WHERE id=?1", params![id])?;
        normalize_category_siblings(&tx, old_parent.as_deref(), None, None)?;
        tx.commit()
    }

    pub fn set_category_color(&self, id: &str, color: Option<&str>) -> rusqlite::Result<()> {
        self.conn()?.execute(
            "UPDATE categories SET color=?1 WHERE id=?2",
            params![color, id],
        )?;
        Ok(())
    }

    pub fn set_category_icon(&self, id: &str, icon_hex: Option<&str>) -> rusqlite::Result<()> {
        self.conn()?.execute(
            "UPDATE categories SET icon_hex=?1 WHERE id=?2",
            params![icon_hex, id],
        )?;
        Ok(())
    }

    pub fn move_category(&self, id: &str, new_parent: Option<&str>, index: usize) -> rusqlite::Result<()> {
        let mut conn = self.conn()?;
        let tx = conn.transaction()?;
        let old_parent: Option<String> = tx.query_row(
            "SELECT parent_id FROM categories WHERE id=?1", params![id], |r| r.get(0)
        )?;
        tx.execute("UPDATE categories SET parent_id=?1 WHERE id=?2", params![new_parent, id])?;
        normalize_category_siblings(&tx, old_parent.as_deref(), None, None)?;
        normalize_category_siblings(&tx, new_parent, Some(id), Some(index))?;
        tx.commit()
    }


    pub fn list_mod_layouts(&self) -> rusqlite::Result<HashMap<String, (Option<String>, i64)>> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare("SELECT layout_key, category_id, sort_index FROM mod_layout")?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                (row.get::<_, Option<String>>(1)?, row.get::<_, i64>(2)?),
            ))
        })?;
        rows.collect()
    }

    pub fn set_mod_layouts(&self, entries: &[(String, Option<String>, i64)]) -> rusqlite::Result<()> {
        let mut conn = self.conn()?;
        let tx = conn.transaction()?;
        {
            let mut stmt = tx.prepare(
                "INSERT INTO mod_layout(layout_key,category_id,sort_index) VALUES(?1,?2,?3)\n                 ON CONFLICT(layout_key) DO UPDATE SET category_id=excluded.category_id, sort_index=excluded.sort_index"
            )?;
            for (key, category_id, sort_index) in entries {
                stmt.execute(params![key, category_id, sort_index])?;
            }
        }
        tx.commit()
    }

    pub fn get_mod_layout(&self, key: &str) -> rusqlite::Result<Option<(Option<String>, i64)>> {
        self.conn()?.query_row(
            "SELECT category_id,sort_index FROM mod_layout WHERE layout_key=?1",
            params![key],
            |r| Ok((r.get(0)?, r.get(1)?))
        ).optional()
    }

    pub fn set_mod_layout(&self, key: &str, category_id: Option<&str>, sort_index: i64) -> rusqlite::Result<()> {
        self.conn()?.execute(
            "INSERT INTO mod_layout(layout_key,category_id,sort_index) VALUES(?1,?2,?3)\n             ON CONFLICT(layout_key) DO UPDATE SET category_id=excluded.category_id, sort_index=excluded.sort_index",
            params![key, category_id, sort_index]
        )?;
        Ok(())
    }
}

fn has_column(conn: &Connection, table: &str, column: &str) -> rusqlite::Result<bool> {
    let sql = format!("PRAGMA table_info({table})");
    let mut stmt = conn.prepare(&sql)?;
    let names = stmt.query_map([], |row| row.get::<_, String>(1))?;
    for name in names {
        if name? == column {
            return Ok(true);
        }
    }
    Ok(false)
}

fn normalize_category_siblings(
    tx: &Transaction<'_>,
    parent_id: Option<&str>,
    moving_id: Option<&str>,
    target_index: Option<usize>,
) -> rusqlite::Result<()> {
    let mut ids: Vec<String> = match parent_id {
        Some(p) => {
            let mut stmt = tx.prepare("SELECT id FROM categories WHERE parent_id=?1 ORDER BY sort_index,name")?;
            let rows = stmt.query_map(params![p], |r| r.get(0))?;
            rows.collect::<rusqlite::Result<Vec<_>>>()?
        }
        None => {
            let mut stmt = tx.prepare("SELECT id FROM categories WHERE parent_id IS NULL ORDER BY sort_index,name")?;
            let rows = stmt.query_map([], |r| r.get(0))?;
            rows.collect::<rusqlite::Result<Vec<_>>>()?
        }
    };
    if let Some(moving) = moving_id {
        ids.retain(|x| x != moving);
        ids.insert(target_index.unwrap_or(ids.len()).min(ids.len()), moving.to_string());
    }
    for (i, category_id) in ids.iter().enumerate() {
        tx.execute("UPDATE categories SET sort_index=?1 WHERE id=?2", params![i as i64, category_id])?;
    }
    Ok(())
}
