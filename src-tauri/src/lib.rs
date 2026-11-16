use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::Manager;

pub struct Db(Mutex<Connection>);

/// 收支类型：0 = 支出，1 = 收入（与 bills.bill_type、categories.kind 一致）
pub const KIND_EXPENSE: i64 = 0;
pub const KIND_INCOME: i64 = 1;

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
    kind: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Bill {
    id: i64,
    amount_cents: i64,
    bill_type: i64,
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
    bill_type: i64,
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
    bill_type: Option<i64>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddCategoryInput {
    name: String,
    icon: Option<String>,
    parent_id: Option<i64>,
    kind: i64,
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
    /// 支出合计
    total_cents: i64,
    /// 支出笔数
    count: i64,
    /// 支出一级分类小计
    by_major: Vec<MajorTotal>,
    /// 收入合计
    income_cents: i64,
    /// 收入笔数
    income_count: i64,
    /// 收入一级分类小计
    income_by_major: Vec<MajorTotal>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrendPoint {
    month: String,
    /// 支出合计
    total_cents: i64,
    /// 收入合计
    income_cents: i64,
    count: i64,
}

/// 预置两级支出分类（一级大类 + 二级小类），与 CLAUDE.md 第四节保持一致
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

/// 预置两级收入分类（V2.0 新增，用户拍板方案：独立收入分类）
const DEFAULT_INCOME_CATEGORIES: &[(&str, &str, &[&str])] = &[
    ("💰", "工资", &["工资", "奖金", "补贴"]),
    ("🧧", "红包礼金", &["红包", "礼金"]),
    ("📈", "投资理财", &["利息", "基金股票", "其他收益"]),
    ("🛒", "退款", &["退款"]),
    ("💻", "兼职外快", &["兼职", "外快"]),
    ("📦", "其他收入", &["其他收入"]),
];

const BILL_SELECT_SQL: &str = "
    SELECT b.id, b.amount_cents, b.bill_type, b.category_id, c.name,
           CASE WHEN c.parent_id IS NULL THEN c.icon ELSE COALESCE(p.icon, '') END,
           p.name, b.note, b.bill_date, b.created_at
    FROM bills b
    JOIN categories c ON b.category_id = c.id
    LEFT JOIN categories p ON c.parent_id = p.id
";

const CATEGORY_SELECT_SQL: &str = "
    SELECT id, name, icon, parent_id, sort_order, hidden, archived, kind
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
                 archived INTEGER NOT NULL DEFAULT 0,
                 kind INTEGER NOT NULL DEFAULT 0
             );
             CREATE TABLE IF NOT EXISTS bills (
                 id INTEGER PRIMARY KEY AUTOINCREMENT,
                 amount_cents INTEGER NOT NULL,
                 category_id INTEGER NOT NULL,
                 note TEXT NOT NULL DEFAULT '',
                 bill_date TEXT NOT NULL,
                 created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%S','now','localtime')),
                 bill_type INTEGER NOT NULL DEFAULT 0
             );",
        )?;
        Self::migrate(&conn)?;
        Self::seed_categories(&conn)?;
        Ok(Self(Mutex::new(conn)))
    }

    /// 旧库升级：为 V1 数据库补上 V2 新增的列（只加不减，不动旧数据）
    fn migrate(conn: &Connection) -> rusqlite::Result<()> {
        let cat_cols: Vec<String> = {
            let mut stmt = conn.prepare("PRAGMA table_info(categories)")?;
            stmt.query_map([], |r| r.get::<_, String>(1))?
                .collect::<Result<Vec<_>, _>>()?
        };
        if !cat_cols.iter().any(|c| c == "kind") {
            conn.execute(
                "ALTER TABLE categories ADD COLUMN kind INTEGER NOT NULL DEFAULT 0",
                [],
            )?;
        }
        let bill_cols: Vec<String> = {
            let mut stmt = conn.prepare("PRAGMA table_info(bills)")?;
            stmt.query_map([], |r| r.get::<_, String>(1))?
                .collect::<Result<Vec<_>, _>>()?
        };
        if !bill_cols.iter().any(|c| c == "bill_type") {
            conn.execute(
                "ALTER TABLE bills ADD COLUMN bill_type INTEGER NOT NULL DEFAULT 0",
                [],
            )?;
        }
        Ok(())
    }

    fn seed_categories(conn: &Connection) -> rusqlite::Result<()> {
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM categories", [], |r| r.get(0))?;
        if count == 0 {
            // 全新数据库：预置支出 + 收入两套分类
            Self::insert_presets(conn, DEFAULT_CATEGORIES, KIND_EXPENSE)?;
            Self::insert_presets(conn, DEFAULT_INCOME_CATEGORIES, KIND_INCOME)?;
            return Ok(());
        }
        // 旧库升级：已有支出分类时，若还没有任何收入分类则补充预置收入分类
        let income_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM categories WHERE kind = 1",
            [],
            |r| r.get(0),
        )?;
        if income_count == 0 {
            Self::insert_presets(conn, DEFAULT_INCOME_CATEGORIES, KIND_INCOME)?;
        }
        Ok(())
    }

    fn insert_presets(
        conn: &Connection,
        presets: &[(&str, &str, &[&str])],
        kind: i64,
    ) -> rusqlite::Result<()> {
        for (major_index, (icon, name, subs)) in presets.iter().enumerate() {
            conn.execute(
                "INSERT INTO categories (name, icon, parent_id, sort_order, kind) VALUES (?1, ?2, NULL, ?3, ?4)",
                rusqlite::params![name, icon, major_index as i64, kind],
            )?;
            let parent_id = conn.last_insert_rowid();
            for (sub_index, sub) in subs.iter().enumerate() {
                conn.execute(
                    "INSERT INTO categories (name, icon, parent_id, sort_order, kind) VALUES (?1, '', ?2, ?3, ?4)",
                    rusqlite::params![sub, parent_id, sub_index as i64, kind],
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
        bill_type: r.get(2)?,
        category_id: r.get(3)?,
        category_name: r.get(4)?,
        category_icon: r.get(5)?,
        parent_name: r.get(6)?,
        note: r.get(7)?,
        bill_date: r.get(8)?,
        created_at: r.get(9)?,
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
        kind: r.get(7)?,
    })
}

#[tauri::command]
fn get_categories(state: tauri::State<Db>) -> Result<Vec<Category>, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(&format!(
            "{} WHERE archived = 0 ORDER BY kind, sort_order, id",
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
              AND (?4 IS NULL OR b.bill_type = ?4)
         ORDER BY b.bill_date DESC, b.id DESC",
        BILL_SELECT_SQL
    );
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(
            rusqlite::params![
                filters.month,
                filters.category_id,
                filters.keyword,
                filters.bill_type
            ],
            row_to_bill,
        )
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

/// 校验分类存在且其收支类型与账单一致，返回分类 kind
fn category_kind(conn: &Connection, category_id: i64) -> rusqlite::Result<i64> {
    conn.query_row(
        "SELECT kind FROM categories WHERE id = ?1 AND archived = 0",
        [category_id],
        |r| r.get(0),
    )
}

#[tauri::command]
fn add_bill(state: tauri::State<Db>, input: AddBillInput) -> Result<Bill, String> {
    if input.amount_cents <= 0 {
        return Err("金额必须大于 0".into());
    }
    if input.bill_type != KIND_EXPENSE && input.bill_type != KIND_INCOME {
        return Err("收支类型无效".into());
    }
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    let kind = category_kind(&conn, input.category_id).map_err(|_| "分类不存在".to_string())?;
    if kind != input.bill_type {
        return Err("分类与收支类型不匹配".into());
    }
    conn.execute(
        "INSERT INTO bills (amount_cents, bill_type, category_id, note, bill_date) VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![
            input.amount_cents,
            input.bill_type,
            input.category_id,
            input.note,
            input.bill_date
        ],
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
    if input.bill_type != KIND_EXPENSE && input.bill_type != KIND_INCOME {
        return Err("收支类型无效".into());
    }
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    let kind = category_kind(&conn, input.category_id).map_err(|_| "分类不存在".to_string())?;
    if kind != input.bill_type {
        return Err("分类与收支类型不匹配".into());
    }
    let changed = conn
        .execute(
            "UPDATE bills SET amount_cents = ?1, bill_type = ?2, category_id = ?3, note = ?4, bill_date = ?5 WHERE id = ?6",
            rusqlite::params![
                input.amount_cents,
                input.bill_type,
                input.category_id,
                input.note,
                input.bill_date,
                id
            ],
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
    // 二级分类的收支类型跟随其一级大类；一级分类使用传入的 kind
    let kind = if let Some(pid) = input.parent_id {
        let parent_kind: i64 = conn
            .query_row(
                "SELECT kind FROM categories WHERE id = ?1 AND parent_id IS NULL AND archived = 0",
                [pid],
                |r| r.get(0),
            )
            .map_err(|_| "所属一级分类不存在".to_string())?;
        parent_kind
    } else {
        if input.kind != KIND_EXPENSE && input.kind != KIND_INCOME {
            return Err("收支类型无效".into());
        }
        input.kind
    };
    let max_sort: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(sort_order), -1) FROM categories WHERE COALESCE(parent_id, -1) = COALESCE(?1, -1) AND kind = ?2",
            rusqlite::params![input.parent_id, kind],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO categories (name, icon, parent_id, sort_order, kind) VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![name, input.icon.unwrap_or_default(), input.parent_id, max_sort + 1, kind],
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

/// 按收支类型统计某一期间内的一级分类小计
fn major_totals(
    conn: &Connection,
    period: &str,
    prefix_len: u32,
    bill_type: i64,
) -> rusqlite::Result<Vec<MajorTotal>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT COALESCE(p.id, c.id),
                CASE WHEN p.id IS NULL THEN c.name ELSE p.name END,
                CASE WHEN p.id IS NULL THEN c.icon ELSE p.icon END,
                SUM(b.amount_cents)
         FROM bills b
         JOIN categories c ON b.category_id = c.id
         LEFT JOIN categories p ON c.parent_id = p.id
         WHERE substr(b.bill_date, 1, {}) = ?1 AND b.bill_type = ?2
         GROUP BY 1
         ORDER BY 4 DESC",
        prefix_len
    ))?;
    let by_major = stmt
        .query_map(rusqlite::params![period, bill_type], |r| {
            Ok(MajorTotal {
                category_id: r.get(0)?,
                name: r.get(1)?,
                icon: r.get(2)?,
                total_cents: r.get(3)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(by_major)
}

/// 按期间（月或年）统计：支出/收入合计、笔数、各自的一级分类小计
fn summary_for(
    conn: &Connection,
    period: &str,
    prefix_len: u32,
) -> rusqlite::Result<MonthSummary> {
    let (total_cents, count, income_cents, income_count): (i64, i64, i64, i64) = conn.query_row(
        &format!(
            "SELECT COALESCE(SUM(CASE WHEN bill_type = 0 THEN amount_cents ELSE 0 END), 0),
                    COALESCE(SUM(CASE WHEN bill_type = 0 THEN 1 ELSE 0 END), 0),
                    COALESCE(SUM(CASE WHEN bill_type = 1 THEN amount_cents ELSE 0 END), 0),
                    COALESCE(SUM(CASE WHEN bill_type = 1 THEN 1 ELSE 0 END), 0)
             FROM bills WHERE substr(bill_date, 1, {}) = ?1",
            prefix_len
        ),
        [period],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
    )?;
    let by_major = major_totals(conn, period, prefix_len, KIND_EXPENSE)?;
    let income_by_major = major_totals(conn, period, prefix_len, KIND_INCOME)?;
    Ok(MonthSummary {
        month: period.to_string(),
        total_cents,
        count,
        by_major,
        income_cents,
        income_count,
        income_by_major,
    })
}

#[tauri::command]
fn get_month_summary(state: tauri::State<Db>, month: String) -> Result<MonthSummary, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    summary_for(&conn, &month, 7).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_year_summary(state: tauri::State<Db>, year: String) -> Result<MonthSummary, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    summary_for(&conn, &year, 4).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_trend(state: tauri::State<Db>, months: i64) -> Result<Vec<TrendPoint>, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT substr(bill_date, 1, 7) AS m,
                    COALESCE(SUM(CASE WHEN bill_type = 0 THEN amount_cents ELSE 0 END), 0),
                    COALESCE(SUM(CASE WHEN bill_type = 1 THEN amount_cents ELSE 0 END), 0),
                    COUNT(*)
             FROM bills
             GROUP BY m
             ORDER BY m DESC
             LIMIT ?1",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([months], |r| {
            Ok(TrendPoint {
                month: r.get(0)?,
                total_cents: r.get(1)?,
                income_cents: r.get(2)?,
                count: r.get(3)?,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

/// 导出全部账单为 Excel 文件，返回导出的账单条数
#[tauri::command]
fn export_excel(state: tauri::State<Db>, path: String) -> Result<i64, String> {
    use rust_xlsxwriter::{Format, FormatAlign, Workbook};

    let conn = state.0.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT b.bill_date, b.bill_type, c.name, p.name, b.amount_cents, b.note
             FROM bills b
             JOIN categories c ON b.category_id = c.id
             LEFT JOIN categories p ON c.parent_id = p.id
             ORDER BY b.bill_date DESC, b.id DESC",
        )
        .map_err(|e| e.to_string())?;
    let rows: Vec<(String, i64, String, Option<String>, i64, String)> = stmt
        .query_map([], |r| {
            Ok((
                r.get(0)?,
                r.get(1)?,
                r.get(2)?,
                r.get(3)?,
                r.get(4)?,
                r.get(5)?,
            ))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();
    worksheet.set_name("账单").map_err(|e| e.to_string())?;

    let header_fmt = Format::new().set_bold().set_align(FormatAlign::Center);
    let money_fmt = Format::new().set_num_format("0.00");
    let headers = ["日期", "收支", "一级分类", "二级分类", "金额（元）", "备注"];
    for (col, h) in headers.iter().enumerate() {
        worksheet
            .write_string_with_format(0, col as u16, *h, &header_fmt)
            .map_err(|e| e.to_string())?;
    }
    for (i, (date, bill_type, cat, parent, cents, note)) in rows.iter().enumerate() {
        let row = (i + 1) as u32;
        worksheet
            .write_string(row, 0, date)
            .map_err(|e| e.to_string())?;
        worksheet
            .write_string(
                row,
                1,
                if *bill_type == KIND_INCOME { "收入" } else { "支出" },
            )
            .map_err(|e| e.to_string())?;
        worksheet
            .write_string(row, 2, parent.as_deref().unwrap_or(""))
            .map_err(|e| e.to_string())?;
        worksheet
            .write_string(row, 3, cat)
            .map_err(|e| e.to_string())?;
        worksheet
            .write_number_with_format(row, 4, *cents as f64 / 100.0, &money_fmt)
            .map_err(|e| e.to_string())?;
        worksheet
            .write_string(row, 5, note)
            .map_err(|e| e.to_string())?;
    }
    worksheet.autofit();
    workbook.save(&path).map_err(|e| e.to_string())?;
    Ok(rows.len() as i64)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
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
            get_month_summary,
            get_year_summary,
            get_trend,
            export_excel
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
