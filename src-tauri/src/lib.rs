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

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BillFilters {
    month: Option<String>,
    category_id: Option<i64>,
    keyword: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddCategoryInput {
    name: String,
    icon: Option<String>,
    parent_id: Option<i64>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCategoryInput {
    name: String,
    icon: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MajorTotal {
    category_id: i64,
    name: String,
    icon: String,
    total_cents: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MonthSummary {
    month: String,
    total_cents: i64,
    count: i64,
    by_major: Vec<MajorTotal>,
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

const CATEGORY_SELECT_SQL: &str = "
    SELECT id, name, icon, parent_id, sort_order, hidden, archived
    FROM categories
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

fn row_to_category(r: &rusqlite::Row) -> rusqlite::Result<Category> {
    Ok(Category {
        id: r.get(0)?,
        name: r.get(1)?,
        icon: r.get(2)?,
        parent_id: r.get(3)?,
        sort_order: r.get(4)?,
        hidden: r.get::<_, i64>(5)? != 0,
        archived: r.get::<_, i64>(6)? != 0,
    })
}

#[tauri::command]
fn get_categories(state: tauri::State<Db>) -> Result<Vec<Category>, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(&format!(
            "{} WHERE archived = 0 ORDER BY sort_order, id",
            CATEGORY_SELECT_SQL
        ))
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], row_to_category)
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

#[tauri::command]
fn get_bills(state: tauri::State<Db>, filters: BillFilters) -> Result<Vec<Bill>, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    let sql = format!(
        "{} WHERE (?1 IS NULL OR substr(b.bill_date, 1, 7) = ?1)
              AND (?2 IS NULL OR b.category_id IN (SELECT id FROM categories WHERE id = ?2 OR parent_id = ?2))
              AND (?3 IS NULL OR b.note LIKE '%' || ?3 || '%')
         ORDER BY b.bill_date DESC, b.id DESC",
        BILL_SELECT_SQL
    );
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(
            rusqlite::params![filters.month, filters.category_id, filters.keyword],
            row_to_bill,
        )
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
fn update_bill(state: tauri::State<Db>, id: i64, input: AddBillInput) -> Result<Bill, String> {
    if input.amount_cents <= 0 {
        return Err("金额必须大于 0".into());
    }
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    let changed = conn
        .execute(
            "UPDATE bills SET amount_cents = ?1, category_id = ?2, note = ?3, bill_date = ?4 WHERE id = ?5",
            rusqlite::params![input.amount_cents, input.category_id, input.note, input.bill_date, id],
        )
        .map_err(|e| e.to_string())?;
    if changed == 0 {
        return Err("账单不存在或已被删除".into());
    }
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

#[tauri::command]
fn add_category(state: tauri::State<Db>, input: AddCategoryInput) -> Result<Category, String> {
    let name = input.name.trim().to_string();
    if name.is_empty() {
        return Err("分类名称不能为空".into());
    }
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    if let Some(pid) = input.parent_id {
        let major_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM categories WHERE id = ?1 AND parent_id IS NULL AND archived = 0",
                [pid],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        if major_count == 0 {
            return Err("所属一级分类不存在".into());
        }
    }
    let max_sort: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(sort_order), -1) FROM categories WHERE COALESCE(parent_id, -1) = COALESCE(?1, -1)",
            [input.parent_id],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO categories (name, icon, parent_id, sort_order) VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![name, input.icon.unwrap_or_default(), input.parent_id, max_sort + 1],
    )
    .map_err(|e| e.to_string())?;
    let id = conn.last_insert_rowid();
    conn.query_row(
        &format!("{} WHERE id = ?1", CATEGORY_SELECT_SQL),
        [id],
        row_to_category,
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
fn update_category(
    state: tauri::State<Db>,
    id: i64,
    input: UpdateCategoryInput,
) -> Result<(), String> {
    let name = input.name.trim().to_string();
    if name.is_empty() {
        return Err("分类名称不能为空".into());
    }
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE categories SET name = ?1, icon = COALESCE(?2, icon) WHERE id = ?3",
        rusqlite::params![name, input.icon, id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn set_category_hidden(state: tauri::State<Db>, id: i64, hidden: bool) -> Result<(), String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE categories SET hidden = ?1 WHERE id = ?2",
        rusqlite::params![hidden as i64, id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn archive_category(state: tauri::State<Db>, id: i64) -> Result<(), String> {
    // 归档一级大类时，其二级小类一并归档；已有账单不受影响
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE categories SET archived = 1 WHERE id = ?1 OR parent_id = ?1",
        [id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn get_month_summary(state: tauri::State<Db>, month: String) -> Result<MonthSummary, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    let (total_cents, count): (i64, i64) = conn
        .query_row(
            "SELECT COALESCE(SUM(amount_cents), 0), COUNT(*) FROM bills WHERE substr(bill_date, 1, 7) = ?1",
            [&month],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT COALESCE(p.id, c.id),
                    CASE WHEN p.id IS NULL THEN c.name ELSE p.name END,
                    CASE WHEN p.id IS NULL THEN c.icon ELSE p.icon END,
                    SUM(b.amount_cents)
             FROM bills b
             JOIN categories c ON b.category_id = c.id
             LEFT JOIN categories p ON c.parent_id = p.id
             WHERE substr(b.bill_date, 1, 7) = ?1
             GROUP BY 1
             ORDER BY 4 DESC",
        )
        .map_err(|e| e.to_string())?;
    let by_major = stmt
        .query_map([&month], |r| {
            Ok(MajorTotal {
                category_id: r.get(0)?,
                name: r.get(1)?,
                icon: r.get(2)?,
                total_cents: r.get(3)?,
            })
        })
        .map_err(|e| e.to_string())?;
    Ok(MonthSummary {
        month,
        total_cents,
        count,
        by_major: by_major
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?,
    })
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
            update_bill,
            delete_bill,
            add_category,
            update_category,
            set_category_hidden,
            archive_category,
            get_month_summary
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
