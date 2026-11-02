use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::Manager;

pub struct Db(Mutex<Connection>);

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Category {
    id: i64,
    name: String,
    icon: String,
    parent_id: Option<i64>,
    sort_order: i64,
    hidden: bool,
    archived: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Bill {
    id: i64,
    amount_cents: i64,
    category_id: i64,
    category_name: String,
    category_icon: String,
    parent_name: Option<String>,
    note: String,
    bill_date: String,
    created_at: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddBillInput {
    amount_cents: i64,
    category_id: i64,
    bill_date: String,
    note: String,
}

/// 预置两级分类（一级大类 + 二级小类），与 CLAUDE.md 第四节保持一致
const DEFAULT_CATEGORIES: &[(&str, &str, &[&str])] = &[
    ("🍜", "餐饮", &["早餐", "午餐", "晚餐", "零食饮料", "外卖", "聚餐"]),
    ("🚕", "交通", &["公交地铁", "打车", "加油", "停车费", "火车机票"]),
    ("🛒", "购物", &["日用百货", "服饰鞋包", "数码电器", "美妆护肤", "其他购物"]),
    ("🏠", "居住", &["房租", "水电燃气", "物业费", "家居维修", "宽带话费"]),
    ("🎮", "娱乐", &["电影演出", "游戏", "会员订阅", "旅游出行", "运动健身"]),
    ("💊", "医疗", &["药品", "门诊", "住院", "体检", "保健"]),
    ("📚", "教育", &["书籍", "课程培训", "文具", "学费"]),
    ("🎁", "人情", &["送礼", "请客", "红包", "捐赠"]),
    ("📦", "其他", &["其他"]),
];

const BILL_SELECT_SQL: &str = "
    SELECT b.id, b.amount_cents, b.category_id, c.name,
           CASE WHEN c.parent_id IS NULL THEN c.icon ELSE COALESCE(p.icon, '') END,
           p.name, b.note, b.bill_date, b.created_at
    FROM bills b
    JOIN categories c ON b.category_id = c.id
    LEFT JOIN categories p ON c.parent_id = p.id
";

impl Db {
    fn new(path: std::path::PathBuf) -> rusqlite::Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             CREATE TABLE IF NOT EXISTS categories (
                 id INTEGER PRIMARY KEY AUTOINCREMENT,
                 name TEXT NOT NULL,
                 icon TEXT NOT NULL DEFAULT '',
                 parent_id INTEGER,
                 sort_order INTEGER NOT NULL DEFAULT 0,
                 hidden INTEGER NOT NULL DEFAULT 0,
                 archived INTEGER NOT NULL DEFAULT 0
             );
             CREATE TABLE IF NOT EXISTS bills (
                 id INTEGER PRIMARY KEY AUTOINCREMENT,
                 amount_cents INTEGER NOT NULL,
                 category_id INTEGER NOT NULL,
                 note TEXT NOT NULL DEFAULT '',
                 bill_date TEXT NOT NULL,
                 created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%S','now','localtime'))
             );",
        )?;
        Self::seed_categories(&conn)?;
        Ok(Self(Mutex::new(conn)))
    }

    fn seed_categories(conn: &Connection) -> rusqlite::Result<()> {
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM categories", [], |r| r.get(0))?;
        if count > 0 {
            return Ok(());
        }
        for (major_index, (icon, name, subs)) in DEFAULT_CATEGORIES.iter().enumerate() {
            conn.execute(
                "INSERT INTO categories (name, icon, parent_id, sort_order) VALUES (?1, ?2, NULL, ?3)",
                rusqlite::params![name, icon, major_index as i64],
            )?;
            let parent_id = conn.last_insert_rowid();
            for (sub_index, sub) in subs.iter().enumerate() {
                conn.execute(
                    "INSERT INTO categories (name, icon, parent_id, sort_order) VALUES (?1, '', ?2, ?3)",
                    rusqlite::params![sub, parent_id, sub_index as i64],
                )?;
            }
        }
        Ok(())
    }
}

fn row_to_bill(r: &rusqlite::Row) -> rusqlite::Result<Bill> {
    Ok(Bill {
        id: r.get(0)?,
        amount_cents: r.get(1)?,
        category_id: r.get(2)?,
        category_name: r.get(3)?,
        category_icon: r.get(4)?,
        parent_name: r.get(5)?,
        note: r.get(6)?,
        bill_date: r.get(7)?,
        created_at: r.get(8)?,
    })
}

#[tauri::command]
fn get_categories(state: tauri::State<Db>) -> Result<Vec<Category>, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT id, name, icon, parent_id, sort_order, hidden, archived
             FROM categories
             WHERE archived = 0
             ORDER BY sort_order, id",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| {
            Ok(Category {
                id: r.get(0)?,
                name: r.get(1)?,
                icon: r.get(2)?,
                parent_id: r.get(3)?,
                sort_order: r.get(4)?,
                hidden: r.get::<_, i64>(5)? != 0,
                archived: r.get::<_, i64>(6)? != 0,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

#[tauri::command]
fn get_bills(state: tauri::State<Db>) -> Result<Vec<Bill>, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(&format!("{} ORDER BY b.bill_date DESC, b.id DESC", BILL_SELECT_SQL))
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], row_to_bill)
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

#[tauri::command]
fn add_bill(state: tauri::State<Db>, input: AddBillInput) -> Result<Bill, String> {
    if input.amount_cents <= 0 {
        return Err("金额必须大于 0".into());
    }
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO bills (amount_cents, category_id, note, bill_date) VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![input.amount_cents, input.category_id, input.note, input.bill_date],
    )
    .map_err(|e| e.to_string())?;
    let id = conn.last_insert_rowid();
    conn.query_row(
        &format!("{} WHERE b.id = ?1", BILL_SELECT_SQL),
        [id],
        row_to_bill,
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_bill(state: tauri::State<Db>, id: i64) -> Result<(), String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM bills WHERE id = ?1", [id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            let db = Db::new(data_dir.join("meteor.db"))?;
            app.manage(db);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_categories,
            get_bills,
            add_bill,
            delete_bill
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
