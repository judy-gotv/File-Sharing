use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use axum::{
    body::Body,
    extract::{ConnectInfo, DefaultBodyLimit, Multipart, Path, Query, Request, State},
    http::{header, HeaderMap, HeaderName, StatusCode, Uri},
    middleware::{self, Next},
    response::{IntoResponse, Redirect, Response},
    routing::{get, post, put},
    Json, Router,
};
use rand::{distributions::Alphanumeric, Rng};
use rust_embed::RustEmbed;
use s3::{creds::Credentials, Bucket, Region};
use serde::{Deserialize, Serialize};
use sqlx::{sqlite::SqlitePoolOptions, FromRow, SqlitePool};
use std::{
    collections::{HashMap, HashSet},
    env,
    net::SocketAddr,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Mutex, RwLock,
    },
    time::{Duration, Instant},
};
use tokio::{io::AsyncWriteExt, sync::Semaphore};
use tokio_util::io::ReaderStream;

#[derive(RustEmbed)]
#[folder = "static/"]
struct Assets;

struct Fail {
    count: u32,
    first: Instant,
    blocked_until: Option<Instant>,
}

#[derive(Clone)]
struct AppState {
    db: SqlitePool,
    fails: Arc<Mutex<HashMap<String, Fail>>>,
    blacklist: Arc<RwLock<HashSet<String>>>,
    trust_proxy: bool,
    requests: Arc<AtomicU64>,
    checking: Arc<AtomicBool>,
    http: reqwest::Client,
    upload_dir: Arc<PathBuf>,
    public_base: Arc<String>,
    s3: Option<Arc<Bucket>>,
    s3_public: Arc<String>,
    metrics_token: Option<Arc<String>>,
}

// ======================= 数据结构 =======================

#[derive(Serialize, FromRow)]
struct Item {
    id: i64,
    required: bool,
    name: String,
    url: String,
    description: String,
    sort: i64,
    category_id: Option<i64>,
    category_name: Option<String>,
    downloads: i64,
    copies: i64,
    alive: Option<bool>,
    checked_at: Option<String>,
}

#[derive(Deserialize)]
struct ItemIn {
    required: bool,
    name: String,
    url: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    sort: i64,
    #[serde(default)]
    category_id: Option<i64>,
}

#[derive(Serialize)]
struct Page {
    total: i64,
    page: i64,
    size: i64,
    items: Vec<Item>,
}

#[derive(Deserialize)]
struct ListQ {
    q: Option<String>,
    category: Option<i64>,
    page: Option<i64>,
    size: Option<i64>,
}

#[derive(Serialize, FromRow)]
struct Category {
    id: i64,
    name: String,
    sort: i64,
    count: i64,
}

#[derive(Deserialize)]
struct CatIn {
    name: String,
    #[serde(default)]
    sort: i64,
}

#[derive(Deserialize)]
struct LoginIn {
    username: String,
    password: String,
}

#[derive(Deserialize)]
struct PwIn {
    old_password: String,
    new_password: String,
}

#[derive(Deserialize)]
struct BatchIn {
    action: String,
    ids: Vec<i64>,
    #[serde(default)]
    category_id: Option<i64>,
}

#[derive(Deserialize)]
struct ReorderIn {
    ids: Vec<i64>,
    #[serde(default)]
    base: i64,
}

#[derive(Deserialize)]
struct StatsQ {
    days: Option<i64>,
}

#[derive(Serialize, FromRow)]
struct Daily {
    day: String,
    downloads: i64,
    copies: i64,
}

#[derive(Serialize, FromRow)]
struct Source {
    source: String,
    count: i64,
}

#[derive(Serialize, FromRow)]
struct Top {
    id: i64,
    name: String,
    downloads: i64,
    copies: i64,
}

#[derive(Serialize, FromRow)]
struct DeadItem {
    id: i64,
    name: String,
    url: String,
    checked_at: Option<String>,
}

#[derive(Serialize, FromRow)]
struct Black {
    ip: String,
    note: String,
    created_at: String,
}

#[derive(Deserialize)]
struct BlackIn {
    ip: String,
    #[serde(default)]
    note: String,
}

#[derive(Deserialize)]
struct IpQ {
    ip: String,
}

// ======================= 工具函数 =======================

fn db_err(e: sqlx::Error) -> StatusCode {
    if let sqlx::Error::Database(d) = &e {
        if d.is_unique_violation() {
            return StatusCode::CONFLICT;
        }
    }
    eprintln!("db error: {e}");
    StatusCode::INTERNAL_SERVER_ERROR
}

fn ise<E: std::fmt::Display>(e: E) -> StatusCode {
    eprintln!("error: {e}");
    StatusCode::INTERNAL_SERVER_ERROR
}

fn ct_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b).fold(0u8, |r, (x, y)| r | (x ^ y)) == 0
}

fn rand_str(n: usize) -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(n)
        .map(char::from)
        .collect()
}

fn hash_pw(pw: &str) -> String {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(pw.as_bytes(), &salt)
        .expect("密码哈希失败")
        .to_string()
}

fn verify_pw(pw: &str, hash: &str) -> bool {
    match PasswordHash::new(hash) {
        Ok(p) => Argon2::default().verify_password(pw.as_bytes(), &p).is_ok(),
        Err(_) => false,
    }
}

fn client_ip(h: &HeaderMap, addr: SocketAddr, trust: bool) -> String {
    if trust {
        if let Some(v) = h.get("x-real-ip").and_then(|v| v.to_str().ok()) {
            return v.trim().to_string();
        }
        if let Some(v) = h.get("x-forwarded-for").and_then(|v| v.to_str().ok()) {
            if let Some(f) = v.split(',').next() {
                return f.trim().to_string();
            }
        }
    }
    addr.ip().to_string()
}

fn ref_host(h: &HeaderMap) -> String {
    h.get(header::REFERER)
        .and_then(|v| v.to_str().ok())
        .and_then(|r| r.split("://").nth(1))
        .and_then(|r| r.split('/').next())
        .filter(|x| !x.is_empty())
        .map(|x| x.chars().take(100).collect())
        .unwrap_or_else(|| "direct".to_string())
}

const SCHEMES: &[&str] = &[
    "http://", "https://", "ftp://", "rtmp://", "rtsp://", "udp://", "rtp://", "mms://",
];

fn valid_url(u: &str) -> bool {
    let l = u.to_ascii_lowercase();
    u.len() <= 2048
        && SCHEMES.iter().any(|s| l.starts_with(s))
        && u.chars().all(|c| c.is_ascii() && !c.is_ascii_control() && c != ' ')
}

fn validate(i: &ItemIn) -> Result<(), StatusCode> {
    let n = i.name.trim();
    if n.is_empty() || n.chars().count() > 200 || !valid_url(i.url.trim()) {
        return Err(StatusCode::BAD_REQUEST);
    }
    if i.description.chars().count() > 1000 {
        return Err(StatusCode::BAD_REQUEST);
    }
    Ok(())
}

fn sanitize(name: &str) -> String {
    let s: String = name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_' { c } else { '_' })
        .collect();
    let s = s.trim_start_matches('.').to_string();
    let s = if s.is_empty() { "file".to_string() } else { s };
    if s.len() > 100 {
        s[s.len() - 100..].to_string()
    } else {
        s
    }
}

// ======================= 鉴权 / 登录 =======================

struct Auth {
    uid: i64,
    username: String,
    token: String,
}

async fn auth(s: &AppState, h: &HeaderMap) -> Result<Auth, StatusCode> {
    let tok = h
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or(StatusCode::UNAUTHORIZED)?;
    let row: Option<(i64, String)> = sqlx::query_as(
        "SELECT s.user_id, u.username FROM sessions s JOIN users u ON u.id = s.user_id
         WHERE s.token = ? AND s.expires_at > datetime('now')",
    )
    .bind(tok)
    .fetch_optional(&s.db)
    .await
    .map_err(db_err)?;
    row.map(|(uid, username)| Auth { uid, username, token: tok.to_string() })
        .ok_or(StatusCode::UNAUTHORIZED)
}

fn register_fail(s: &AppState, ip: &str) {
    let mut m = s.fails.lock().unwrap();
    let now = Instant::now();
    let f = m.entry(ip.to_string()).or_insert(Fail { count: 0, first: now, blocked_until: None });
    if now.duration_since(f.first) > Duration::from_secs(600) {
        f.count = 0;
        f.first = now;
    }
    f.count += 1;
    if f.count >= 5 {
        f.blocked_until = Some(now + Duration::from_secs(900));
        f.count = 0;
        f.first = now;
    }
    if m.len() > 10000 {
        m.retain(|_, v| match v.blocked_until {
            Some(b) => b > now,
            None => now.duration_since(v.first) <= Duration::from_secs(600),
        });
    }
}

async fn login(
    State(s): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(inp): Json<LoginIn>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let ip = client_ip(&headers, addr, s.trust_proxy);
    {
        let m = s.fails.lock().unwrap();
        if let Some(f) = m.get(&ip) {
            if let Some(b) = f.blocked_until {
                if b > Instant::now() {
                    return Err(StatusCode::TOO_MANY_REQUESTS);
                }
            }
        }
    }
    let row: Option<(i64, String)> =
        sqlx::query_as("SELECT id, password_hash FROM users WHERE username = ?")
            .bind(inp.username.trim())
            .fetch_optional(&s.db)
            .await
            .map_err(db_err)?;
    let (uid, ok) = match row {
        Some((uid, hash)) => {
            let pw = inp.password.clone();
            let ok = tokio::task::spawn_blocking(move || verify_pw(&pw, &hash))
                .await
                .unwrap_or(false);
            (uid, ok)
        }
        None => (0, false),
    };
    if !ok {
        register_fail(&s, &ip);
        return Err(StatusCode::UNAUTHORIZED);
    }
    s.fails.lock().unwrap().remove(&ip);

    sqlx::query("DELETE FROM sessions WHERE expires_at <= datetime('now')")
        .execute(&s.db)
        .await
        .map_err(db_err)?;
    let tok = rand_str(48);
    sqlx::query("INSERT INTO sessions(token, user_id, expires_at) VALUES(?, ?, datetime('now','+7 days'))")
        .bind(&tok)
        .bind(uid)
        .execute(&s.db)
        .await
        .map_err(db_err)?;
    Ok(Json(serde_json::json!({ "token": tok, "username": inp.username.trim(), "role": "admin" })))
}

async fn logout(State(s): State<AppState>, h: HeaderMap) -> Result<StatusCode, StatusCode> {
    let a = auth(&s, &h).await?;
    sqlx::query("DELETE FROM sessions WHERE token = ?")
        .bind(&a.token)
        .execute(&s.db)
        .await
        .map_err(db_err)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn check(State(s): State<AppState>, h: HeaderMap) -> Result<Json<serde_json::Value>, StatusCode> {
    let a = auth(&s, &h).await?;
    Ok(Json(serde_json::json!({ "username": a.username, "role": "admin" })))
}

async fn change_password(
    State(s): State<AppState>,
    h: HeaderMap,
    Json(inp): Json<PwIn>,
) -> Result<StatusCode, StatusCode> {
    let a = auth(&s, &h).await?;
    if inp.new_password.len() < 8 || inp.new_password.len() > 128 {
        return Err(StatusCode::BAD_REQUEST);
    }
    let hash: String = sqlx::query_scalar("SELECT password_hash FROM users WHERE id = ?")
        .bind(a.uid)
        .fetch_one(&s.db)
        .await
        .map_err(db_err)?;
    let old = inp.old_password.clone();
    let ok = tokio::task::spawn_blocking(move || verify_pw(&old, &hash))
        .await
        .unwrap_or(false);
    if !ok {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }
    let newpw = inp.new_password.clone();
    let nh = tokio::task::spawn_blocking(move || hash_pw(&newpw)).await.map_err(ise)?;
    sqlx::query("UPDATE users SET password_hash = ? WHERE id = ?")
        .bind(nh)
        .bind(a.uid)
        .execute(&s.db)
        .await
        .map_err(db_err)?;
    sqlx::query("DELETE FROM sessions WHERE user_id = ? AND token <> ?")
        .bind(a.uid)
        .bind(&a.token)
        .execute(&s.db)
        .await
        .map_err(db_err)?;
    Ok(StatusCode::NO_CONTENT)
}

// ======================= 中间件:黑名单 + 请求计数 =======================

async fn guard(
    State(s): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    req: Request,
    next: Next,
) -> Response {
    s.requests.fetch_add(1, Ordering::Relaxed);
    let ip = client_ip(req.headers(), addr, s.trust_proxy);
    if s.blacklist.read().unwrap().contains(&ip) {
        return StatusCode::FORBIDDEN.into_response();
    }
    next.run(req).await
}

// ======================= 条目 =======================

const ITEM_SELECT: &str = "SELECT i.id, i.required, i.name, i.url, i.description, i.sort,
    i.category_id, c.name AS category_name, i.downloads, i.copies, i.alive, i.checked_at
    FROM items i LEFT JOIN categories c ON c.id = i.category_id";

async fn list_items(
    State(s): State<AppState>,
    Query(q): Query<ListQ>,
) -> Result<Json<Page>, StatusCode> {
    let like = format!("%{}%", q.q.as_deref().unwrap_or("").trim());
    let size = q.size.unwrap_or(20).clamp(1, 200);
    let page = q.page.unwrap_or(1).max(1);
    let offset = (page - 1) * size;

    let total: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM items i
         WHERE (i.name LIKE ?1 OR i.description LIKE ?1)
           AND (?2 IS NULL OR i.category_id = ?2)",
    )
    .bind(&like)
    .bind(q.category)
    .fetch_one(&s.db)
    .await
    .map_err(db_err)?;

    let sql = format!(
        "{ITEM_SELECT}
         WHERE (i.name LIKE ?1 OR i.description LIKE ?1)
           AND (?2 IS NULL OR i.category_id = ?2)
         ORDER BY i.required DESC, i.sort ASC, i.id ASC
         LIMIT ?3 OFFSET ?4"
    );
    let items = sqlx::query_as::<_, Item>(&sql)
        .bind(&like)
        .bind(q.category)
        .bind(size)
        .bind(offset)
        .fetch_all(&s.db)
        .await
        .map_err(db_err)?;
    Ok(Json(Page { total, page, size, items }))
}

async fn create_item(
    State(s): State<AppState>,
    h: HeaderMap,
    Json(i): Json<ItemIn>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    auth(&s, &h).await?;
    validate(&i)?;
    let r = sqlx::query(
        "INSERT INTO items(required,name,url,description,sort,category_id) VALUES(?,?,?,?,?,?)",
    )
    .bind(i.required)
    .bind(i.name.trim())
    .bind(i.url.trim())
    .bind(i.description.trim())
    .bind(i.sort)
    .bind(i.category_id)
    .execute(&s.db)
    .await
    .map_err(db_err)?;
    Ok(Json(serde_json::json!({ "id": r.last_insert_rowid() })))
}

async fn update_item(
    State(s): State<AppState>,
    h: HeaderMap,
    Path(id): Path<i64>,
    Json(i): Json<ItemIn>,
) -> Result<StatusCode, StatusCode> {
    auth(&s, &h).await?;
    validate(&i)?;
    let r = sqlx::query(
        "UPDATE items SET required=?, name=?, url=?, description=?, sort=?, category_id=?,
                alive=NULL, fail_count=0, checked_at=NULL
         WHERE id=?",
    )
    .bind(i.required)
    .bind(i.name.trim())
    .bind(i.url.trim())
    .bind(i.description.trim())
    .bind(i.sort)
    .bind(i.category_id)
    .bind(id)
    .execute(&s.db)
    .await
    .map_err(db_err)?;
    if r.rows_affected() == 0 {
        return Err(StatusCode::NOT_FOUND);
    }
    Ok(StatusCode::NO_CONTENT)
}

async fn delete_item(
    State(s): State<AppState>,
    h: HeaderMap,
    Path(id): Path<i64>,
) -> Result<StatusCode, StatusCode> {
    auth(&s, &h).await?;
    sqlx::query("DELETE FROM items WHERE id=?")
        .bind(id)
        .execute(&s.db)
        .await
        .map_err(db_err)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn batch_items(
    State(s): State<AppState>,
    h: HeaderMap,
    Json(b): Json<BatchIn>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    auth(&s, &h).await?;
    if b.ids.is_empty() || b.ids.len() > 5000 {
        return Err(StatusCode::BAD_REQUEST);
    }
    let mut tx = s.db.begin().await.map_err(db_err)?;
    let mut affected = 0u64;
    for id in &b.ids {
        let r = match b.action.as_str() {
            "delete" => sqlx::query("DELETE FROM items WHERE id = ?").bind(id).execute(&mut *tx).await,
            "move" => {
                sqlx::query("UPDATE items SET category_id = ? WHERE id = ?")
                    .bind(b.category_id)
                    .bind(id)
                    .execute(&mut *tx)
                    .await
            }
            _ => return Err(StatusCode::BAD_REQUEST),
        }
        .map_err(db_err)?;
        affected += r.rows_affected();
    }
    tx.commit().await.map_err(db_err)?;
    Ok(Json(serde_json::json!({ "affected": affected })))
}

async fn reorder_items(
    State(s): State<AppState>,
    h: HeaderMap,
    Json(b): Json<ReorderIn>,
) -> Result<StatusCode, StatusCode> {
    auth(&s, &h).await?;
    if b.ids.len() > 5000 {
        return Err(StatusCode::BAD_REQUEST);
    }
    let mut tx = s.db.begin().await.map_err(db_err)?;
    for (idx, id) in b.ids.iter().enumerate() {
        sqlx::query("UPDATE items SET sort = ? WHERE id = ?")
            .bind(b.base + idx as i64)
            .bind(id)
            .execute(&mut *tx)
            .await
            .map_err(db_err)?;
    }
    tx.commit().await.map_err(db_err)?;
    Ok(StatusCode::NO_CONTENT)
}

// ======================= 下载统计 / 复制统计 =======================

// 站点设置(键值表)。读取失败时默认开启,保持与旧版本一致的行为。
async fn setting_on(db: &sqlx::SqlitePool, key: &str) -> bool {
    sqlx::query_scalar::<_, String>("SELECT value FROM settings WHERE key = ?")
        .bind(key)
        .fetch_optional(db)
        .await
        .ok()
        .flatten()
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(true)
}

async fn go(
    State(s): State<AppState>,
    Path(id): Path<i64>,
    headers: HeaderMap,
) -> Result<Redirect, StatusCode> {
    if !setting_on(&s.db, "name_download").await {
        return Err(StatusCode::FORBIDDEN);
    }
    let url: Option<String> =
        sqlx::query_scalar("UPDATE items SET downloads = downloads + 1 WHERE id = ? RETURNING url")
            .bind(id)
            .fetch_optional(&s.db)
            .await
            .map_err(db_err)?;
    match url {
        Some(u) if valid_url(&u) => {
            let _ = sqlx::query(
                "INSERT INTO stats_daily(day,item_id,downloads,copies)
                 VALUES(date('now','localtime'),?,1,0)
                 ON CONFLICT(day,item_id) DO UPDATE SET downloads = downloads + 1",
            )
            .bind(id)
            .execute(&s.db)
            .await;
            let _ = sqlx::query(
                "INSERT INTO stats_source(day,source,count)
                 VALUES(date('now','localtime'),?,1)
                 ON CONFLICT(day,source) DO UPDATE SET count = count + 1",
            )
            .bind(ref_host(&headers))
            .execute(&s.db)
            .await;
            Ok(Redirect::temporary(&u))
        }
        Some(_) => Err(StatusCode::BAD_REQUEST),
        None => Err(StatusCode::NOT_FOUND),
    }
}

async fn count_copy(State(s): State<AppState>, Path(id): Path<i64>) -> Result<StatusCode, StatusCode> {
    let r = sqlx::query("UPDATE items SET copies = copies + 1 WHERE id = ?")
        .bind(id)
        .execute(&s.db)
        .await
        .map_err(db_err)?;
    if r.rows_affected() == 0 {
        return Err(StatusCode::NOT_FOUND);
    }
    let _ = sqlx::query(
        "INSERT INTO stats_daily(day,item_id,downloads,copies)
         VALUES(date('now','localtime'),?,0,1)
         ON CONFLICT(day,item_id) DO UPDATE SET copies = copies + 1",
    )
    .bind(id)
    .execute(&s.db)
    .await;
    Ok(StatusCode::NO_CONTENT)
}

async fn stats(
    State(s): State<AppState>,
    h: HeaderMap,
    Query(q): Query<StatsQ>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    auth(&s, &h).await?;
    let days = q.days.unwrap_or(30).clamp(1, 365);
    let modifier = format!("-{} days", days - 1);

    let daily = sqlx::query_as::<_, Daily>(
        "SELECT day, COALESCE(SUM(downloads),0) AS downloads, COALESCE(SUM(copies),0) AS copies
         FROM stats_daily WHERE day >= date('now','localtime', ?)
         GROUP BY day ORDER BY day ASC",
    )
    .bind(&modifier)
    .fetch_all(&s.db)
    .await
    .map_err(db_err)?;

    let sources = sqlx::query_as::<_, Source>(
        "SELECT source, COALESCE(SUM(count),0) AS count FROM stats_source
         WHERE day >= date('now','localtime', ?)
         GROUP BY source ORDER BY count DESC LIMIT 20",
    )
    .bind(&modifier)
    .fetch_all(&s.db)
    .await
    .map_err(db_err)?;

    let top = sqlx::query_as::<_, Top>(
        "SELECT id, name, downloads, copies FROM items ORDER BY downloads DESC, copies DESC LIMIT 10",
    )
    .fetch_all(&s.db)
    .await
    .map_err(db_err)?;

    Ok(Json(serde_json::json!({ "daily": daily, "sources": sources, "top": top })))
}

// ======================= 仪表盘 =======================

async fn dashboard(
    State(s): State<AppState>,
    h: HeaderMap,
) -> Result<Json<serde_json::Value>, StatusCode> {
    auth(&s, &h).await?;
    let (items, downloads, copies, dead, unchecked, uncategorized): (i64, i64, i64, i64, i64, i64) =
        sqlx::query_as(
            "SELECT COUNT(*),
                    COALESCE(SUM(downloads),0),
                    COALESCE(SUM(copies),0),
                    COALESCE(SUM(CASE WHEN alive = 0 THEN 1 ELSE 0 END),0),
                    COALESCE(SUM(CASE WHEN alive IS NULL THEN 1 ELSE 0 END),0),
                    COALESCE(SUM(CASE WHEN category_id IS NULL THEN 1 ELSE 0 END),0)
             FROM items",
        )
        .fetch_one(&s.db)
        .await
        .map_err(db_err)?;
    let categories: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM categories")
        .fetch_one(&s.db)
        .await
        .map_err(db_err)?;
    let (today_downloads, today_copies): (i64, i64) = sqlx::query_as(
        "SELECT COALESCE(SUM(downloads),0), COALESCE(SUM(copies),0)
         FROM stats_daily WHERE day = date('now','localtime')",
    )
    .fetch_one(&s.db)
    .await
    .map_err(db_err)?;
    let dead_list = sqlx::query_as::<_, DeadItem>(
        "SELECT id, name, url, checked_at FROM items WHERE alive = 0
         ORDER BY checked_at DESC LIMIT 20",
    )
    .fetch_all(&s.db)
    .await
    .map_err(db_err)?;

    Ok(Json(serde_json::json!({
        "items": items, "categories": categories,
        "downloads": downloads, "copies": copies,
        "today_downloads": today_downloads, "today_copies": today_copies,
        "dead": dead, "unchecked": unchecked, "uncategorized": uncategorized,
        "dead_list": dead_list
    })))
}

// ======================= 分类 =======================

async fn list_cats(State(s): State<AppState>) -> Result<Json<Vec<Category>>, StatusCode> {
    let rows = sqlx::query_as::<_, Category>(
        "SELECT c.id, c.name, c.sort,
                (SELECT COUNT(*) FROM items i WHERE i.category_id = c.id) AS count
         FROM categories c ORDER BY c.sort ASC, c.id ASC",
    )
    .fetch_all(&s.db)
    .await
    .map_err(db_err)?;
    Ok(Json(rows))
}

async fn create_cat(
    State(s): State<AppState>,
    h: HeaderMap,
    Json(c): Json<CatIn>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    auth(&s, &h).await?;
    let n = c.name.trim();
    if n.is_empty() || n.chars().count() > 100 {
        return Err(StatusCode::BAD_REQUEST);
    }
    let r = sqlx::query("INSERT INTO categories(name, sort) VALUES(?, ?)")
        .bind(n)
        .bind(c.sort)
        .execute(&s.db)
        .await
        .map_err(db_err)?;
    Ok(Json(serde_json::json!({ "id": r.last_insert_rowid() })))
}

async fn update_cat(
    State(s): State<AppState>,
    h: HeaderMap,
    Path(id): Path<i64>,
    Json(c): Json<CatIn>,
) -> Result<StatusCode, StatusCode> {
    auth(&s, &h).await?;
    let n = c.name.trim();
    if n.is_empty() || n.chars().count() > 100 {
        return Err(StatusCode::BAD_REQUEST);
    }
    let r = sqlx::query("UPDATE categories SET name=?, sort=? WHERE id=?")
        .bind(n)
        .bind(c.sort)
        .bind(id)
        .execute(&s.db)
        .await
        .map_err(db_err)?;
    if r.rows_affected() == 0 {
        return Err(StatusCode::NOT_FOUND);
    }
    Ok(StatusCode::NO_CONTENT)
}

async fn delete_cat(
    State(s): State<AppState>,
    h: HeaderMap,
    Path(id): Path<i64>,
) -> Result<StatusCode, StatusCode> {
    auth(&s, &h).await?;
    let mut tx = s.db.begin().await.map_err(db_err)?;
    sqlx::query("UPDATE items SET category_id = NULL WHERE category_id = ?")
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(db_err)?;
    sqlx::query("DELETE FROM categories WHERE id = ?")
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(db_err)?;
    tx.commit().await.map_err(db_err)?;
    Ok(StatusCode::NO_CONTENT)
}

// ======================= IP 黑名单 =======================

// ======================= IP 黑名单 =======================

// ======================= 站点设置 =======================

#[derive(Deserialize)]
struct SettingsIn {
    #[serde(default)]
    name_download: Option<bool>,
}

async fn get_settings(
    State(s): State<AppState>,
    h: HeaderMap,
) -> Result<Json<serde_json::Value>, StatusCode> {
    auth(&s, &h).await?;
    Ok(Json(serde_json::json!({ "name_download": setting_on(&s.db, "name_download").await })))
}

async fn put_settings(
    State(s): State<AppState>,
    h: HeaderMap,
    Json(b): Json<SettingsIn>,
) -> Result<StatusCode, StatusCode> {
    auth(&s, &h).await?;
    if let Some(v) = b.name_download {
        sqlx::query(
            "INSERT INTO settings(key, value) VALUES('name_download', ?)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        )
        .bind(if v { "1" } else { "0" })
        .execute(&s.db)
        .await
        .map_err(db_err)?;
    }
    Ok(StatusCode::NO_CONTENT)
}

// 前台公开读取(无需登录):首页据此决定名字是否可点击下载
async fn public_settings(State(s): State<AppState>) -> Json<serde_json::Value> {
    Json(serde_json::json!({ "name_download": setting_on(&s.db, "name_download").await }))
}

async fn list_black(State(s): State<AppState>, h: HeaderMap) -> Result<Json<Vec<Black>>, StatusCode> {
    auth(&s, &h).await?;
    let rows = sqlx::query_as::<_, Black>(
        "SELECT ip, note, created_at FROM ip_blacklist ORDER BY created_at DESC",
    )
    .fetch_all(&s.db)
    .await
    .map_err(db_err)?;
    Ok(Json(rows))
}

async fn add_black(
    State(s): State<AppState>,
    h: HeaderMap,
    Json(b): Json<BlackIn>,
) -> Result<StatusCode, StatusCode> {
    auth(&s, &h).await?;
    let ip: std::net::IpAddr = b.ip.trim().parse().map_err(|_| StatusCode::BAD_REQUEST)?;
    let ip = ip.to_string();
    sqlx::query("INSERT OR REPLACE INTO ip_blacklist(ip, note) VALUES(?, ?)")
        .bind(&ip)
        .bind(b.note.chars().take(200).collect::<String>())
        .execute(&s.db)
        .await
        .map_err(db_err)?;
    s.blacklist.write().unwrap().insert(ip);
    Ok(StatusCode::NO_CONTENT)
}

async fn del_black(
    State(s): State<AppState>,
    h: HeaderMap,
    Query(q): Query<IpQ>,
) -> Result<StatusCode, StatusCode> {
    auth(&s, &h).await?;
    sqlx::query("DELETE FROM ip_blacklist WHERE ip = ?")
        .bind(&q.ip)
        .execute(&s.db)
        .await
        .map_err(db_err)?;
    s.blacklist.write().unwrap().remove(&q.ip);
    Ok(StatusCode::NO_CONTENT)
}

// ======================= 失效链接检测 =======================

async fn probe(c: &reqwest::Client, url: &str) -> bool {
    if let Ok(r) = c.head(url).send().await {
        if r.status().as_u16() < 400 {
            return true;
        }
    }
    match c.get(url).header("Range", "bytes=0-0").send().await {
        Ok(r) => r.status().as_u16() < 400,
        Err(_) => false,
    }
}

async fn check_all(db: SqlitePool, client: reqwest::Client) {
    let rows: Vec<(i64, String, i64)> = sqlx::query_as(
        "SELECT id, url, fail_count FROM items WHERE url LIKE 'http://%' OR url LIKE 'https://%'",
    )
    .fetch_all(&db)
    .await
    .unwrap_or_default();
    let sem = Arc::new(Semaphore::new(8));
    let mut handles = Vec::new();
    for (id, url, fc) in rows {
        let permit = sem.clone().acquire_owned().await.unwrap();
        let db = db.clone();
        let c = client.clone();
        handles.push(tokio::spawn(async move {
            let ok = probe(&c, &url).await;
            let fc2 = if ok { 0 } else { fc + 1 };
            let _ = sqlx::query(
                "UPDATE items SET fail_count = ?1,
                    alive = CASE WHEN ?2 = 1 THEN 1 WHEN ?1 >= 2 THEN 0 ELSE alive END,
                    checked_at = datetime('now')
                 WHERE id = ?3",
            )
            .bind(fc2)
            .bind(ok as i64)
            .bind(id)
            .execute(&db)
            .await;
            drop(permit);
        }));
    }
    for h in handles {
        let _ = h.await;
    }
}

async fn run_check(s: &AppState) -> bool {
    if s.checking.swap(true, Ordering::SeqCst) {
        return false;
    }
    check_all(s.db.clone(), s.http.clone()).await;
    s.checking.store(false, Ordering::SeqCst);
    true
}

async fn trigger_check(State(s): State<AppState>, h: HeaderMap) -> Result<StatusCode, StatusCode> {
    auth(&s, &h).await?;
    if s.checking.load(Ordering::SeqCst) {
        return Err(StatusCode::CONFLICT);
    }
    let st = s.clone();
    tokio::spawn(async move {
        run_check(&st).await;
    });
    Ok(StatusCode::ACCEPTED)
}

// ======================= 文件上传(本地 / S3 / R2) =======================

// 根据本次请求的 Host 自动拼出对外地址:域名访问显示域名,IP+端口访问显示IP+端口。
// TRUST_PROXY=true 时信任 X-Forwarded-Proto 判定 http/https;Host 非法或缺失时回退 PUBLIC_BASE_URL。
fn request_base(h: &HeaderMap, s: &AppState) -> String {
    let host = h
        .get(header::HOST)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .split(',')
        .next()
        .unwrap_or("")
        .trim();
    let host_ok = !host.is_empty()
        && host.len() <= 253
        && host
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | ':' | '[' | ']'));
    if !host_ok {
        return s.public_base.trim_end_matches('/').to_string();
    }
    let scheme = if s.trust_proxy {
        h.get("x-forwarded-proto")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.split(',').next())
            .map(|v| v.trim().to_ascii_lowercase())
            .filter(|v| v == "https" || v == "http")
            .unwrap_or_else(|| "http".to_string())
    } else {
        "http".to_string()
    };
    format!("{scheme}://{host}")
}

async fn upload(
    State(s): State<AppState>,
    h: HeaderMap,
    mut mp: Multipart,
) -> Result<Json<serde_json::Value>, StatusCode> {
    auth(&s, &h).await?;
    while let Some(mut field) = mp.next_field().await.map_err(|_| StatusCode::BAD_REQUEST)? {
        if field.name() != Some("file") {
            continue;
        }
        let orig = field.file_name().unwrap_or("file").to_string();
        let key = format!("{}-{}", rand_str(8), sanitize(&orig));

        let url = if let Some(bucket) = &s.s3 {
            let mut data: Vec<u8> = Vec::new();
            while let Some(c) = field.chunk().await.map_err(|_| StatusCode::BAD_REQUEST)? {
                data.extend_from_slice(&c);
            }
            let ct = mime_guess::from_path(&key).first_or_octet_stream().to_string();
            let r = bucket
                .put_object_with_content_type(format!("/{key}"), &data, &ct)
                .await
                .map_err(|e| {
                    eprintln!("s3 error: {e}");
                    StatusCode::BAD_GATEWAY
                })?;
            if r.status_code() >= 300 {
                eprintln!("s3 status: {}", r.status_code());
                return Err(StatusCode::BAD_GATEWAY);
            }
            format!("{}/{}", s.s3_public.trim_end_matches('/'), key)
        } else {
            let path = s.upload_dir.join(&key);
            let mut f = tokio::fs::File::create(&path).await.map_err(ise)?;
            while let Some(c) = field.chunk().await.map_err(|_| StatusCode::BAD_REQUEST)? {
                f.write_all(&c).await.map_err(ise)?;
            }
            f.flush().await.map_err(ise)?;
            format!("{}/files/{}", request_base(&h, &s).trim_end_matches('/'), key)
        };
        return Ok(Json(serde_json::json!({ "url": url, "name": orig })));
    }
    Err(StatusCode::BAD_REQUEST)
}

async fn serve_file(State(s): State<AppState>, Path(name): Path<String>) -> Response {
    if name.contains('/') || name.contains('\\') || name.contains("..") {
        return StatusCode::NOT_FOUND.into_response();
    }
    let path = s.upload_dir.join(&name);
    let f = match tokio::fs::File::open(&path).await {
        Ok(f) => f,
        Err(_) => return StatusCode::NOT_FOUND.into_response(),
    };
    let len = f.metadata().await.map(|m| m.len()).unwrap_or(0);
    let ct = mime_guess::from_path(&name).first_or_octet_stream().to_string();
    (
        [
            (header::CONTENT_TYPE, ct),
            (header::CONTENT_LENGTH, len.to_string()),
            (header::CONTENT_DISPOSITION, "attachment".to_string()),
            (HeaderName::from_static("x-content-type-options"), "nosniff".to_string()),
        ],
        Body::from_stream(ReaderStream::new(f)),
    )
        .into_response()
}

// ======================= 健康检查 / Prometheus =======================

async fn healthz(State(s): State<AppState>) -> Response {
    match sqlx::query("SELECT 1").execute(&s.db).await {
        Ok(_) => "ok".into_response(),
        Err(_) => StatusCode::SERVICE_UNAVAILABLE.into_response(),
    }
}

async fn metrics(State(s): State<AppState>, h: HeaderMap) -> Result<Response, StatusCode> {
    if let Some(tk) = &s.metrics_token {
        let ok = h
            .get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .map(|t| ct_eq(t.as_bytes(), tk.as_bytes()))
            .unwrap_or(false);
        if !ok {
            return Err(StatusCode::UNAUTHORIZED);
        }
    }
    let (items, dl, cp, dead): (i64, i64, i64, i64) = sqlx::query_as(
        "SELECT COUNT(*), COALESCE(SUM(downloads),0), COALESCE(SUM(copies),0),
                COALESCE(SUM(CASE WHEN alive = 0 THEN 1 ELSE 0 END),0) FROM items",
    )
    .fetch_one(&s.db)
    .await
    .map_err(db_err)?;
    let body = format!(
        "# TYPE fileshare_http_requests_total counter\nfileshare_http_requests_total {}\n\
         # TYPE fileshare_items gauge\nfileshare_items {}\n\
         # TYPE fileshare_items_dead gauge\nfileshare_items_dead {}\n\
         # TYPE fileshare_downloads_total counter\nfileshare_downloads_total {}\n\
         # TYPE fileshare_copies_total counter\nfileshare_copies_total {}\n",
        s.requests.load(Ordering::Relaxed),
        items,
        dead,
        dl,
        cp
    );
    Ok(([(header::CONTENT_TYPE, "text/plain; version=0.0.4")], body).into_response())
}

// ======================= 静态资源 =======================

async fn static_handler(uri: Uri) -> Response {
    let p = uri.path().trim_start_matches('/');
    let p = if p.is_empty() { "index.html" } else { p };
    match Assets::get(p) {
        Some(f) => {
            let m = mime_guess::from_path(p).first_or_octet_stream();
            (
                [
                    (header::CONTENT_TYPE, m.to_string()),
                    (header::CACHE_CONTROL, "no-cache".to_string()),
                ],
                f.data,
            )
                .into_response()
        }
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

// ======================= 启动 =======================

fn env_or(k: &str, d: &str) -> String {
    env::var(k).unwrap_or_else(|_| d.to_string())
}

#[tokio::main]
async fn main() {
    let db_url = env_or("DATABASE_URL", "sqlite://data/fileshare.db?mode=rwc");
    let addr = env_or("LISTEN", "0.0.0.0:8080");
    let upload_dir = PathBuf::from(env_or("UPLOAD_DIR", "data/uploads"));
    let max_mb: usize = env_or("UPLOAD_MAX_MB", "200").parse().unwrap_or(200);
    let interval: u64 = env_or("CHECK_INTERVAL_MIN", "360").parse().unwrap_or(360);

    std::fs::create_dir_all("data").ok();
    std::fs::create_dir_all(&upload_dir).ok();

    let db = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await
        .expect("数据库连接失败");
    sqlx::migrate!().run(&db).await.expect("数据库迁移失败");

    let users: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(&db)
        .await
        .unwrap();
    if users == 0 {
        let user = env_or("ADMIN_USER", "admin");
        let pw = env::var("ADMIN_PASSWORD").expect("首次启动必须设置 ADMIN_PASSWORD");
        if pw.len() < 8 {
            panic!("ADMIN_PASSWORD 至少 8 位");
        }
        sqlx::query("INSERT INTO users(username, password_hash) VALUES(?, ?)")
            .bind(&user)
            .bind(hash_pw(&pw))
            .execute(&db)
            .await
            .unwrap();
        println!("已创建管理员账号: {user}");
    }

    let blacklist: Vec<String> = sqlx::query_scalar("SELECT ip FROM ip_blacklist")
        .fetch_all(&db)
        .await
        .unwrap_or_default();

    let s3 = match env::var("S3_BUCKET") {
        Ok(name) => {
            let region = Region::Custom {
                region: env_or("S3_REGION", "auto"),
                endpoint: env::var("S3_ENDPOINT").expect("使用 S3 必须设置 S3_ENDPOINT"),
            };
            let creds = Credentials::new(
                Some(&env::var("S3_ACCESS_KEY").expect("缺少 S3_ACCESS_KEY")),
                Some(&env::var("S3_SECRET_KEY").expect("缺少 S3_SECRET_KEY")),
                None,
                None,
                None,
            )
            .expect("S3 凭证错误");
            let b = Bucket::new(&name, region, creds).expect("S3 初始化失败").with_path_style();
            Some(Arc::new(b))
        }
        Err(_) => None,
    };

    let http = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .redirect(reqwest::redirect::Policy::limited(5))
        .user_agent("fileshare-checker/0.4")
        .build()
        .unwrap();

    let state = AppState {
        db,
        fails: Arc::new(Mutex::new(HashMap::new())),
        blacklist: Arc::new(RwLock::new(blacklist.into_iter().collect())),
        trust_proxy: env_or("TRUST_PROXY", "false") == "true",
        requests: Arc::new(AtomicU64::new(0)),
        checking: Arc::new(AtomicBool::new(false)),
        http,
        upload_dir: Arc::new(upload_dir),
        public_base: Arc::new(env_or("PUBLIC_BASE_URL", "http://127.0.0.1:8080")),
        s3,
        s3_public: Arc::new(env_or("S3_PUBLIC_BASE", "")),
        metrics_token: env::var("METRICS_TOKEN").ok().map(Arc::new),
    };

    if interval > 0 {
        let st = state.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(60)).await;
            loop {
                run_check(&st).await;
                tokio::time::sleep(Duration::from_secs(interval * 60)).await;
            }
        });
    }

    let app = Router::new()
        .route("/api/auth/login", post(login))
        .route("/api/auth/logout", post(logout))
        .route("/api/auth/check", get(check))
        .route("/api/auth/password", post(change_password))
        .route("/api/items", get(list_items).post(create_item))
        .route("/api/items/:id", put(update_item).delete(delete_item))
        .route("/api/items/:id/copy", post(count_copy))
        .route("/api/settings", get(public_settings))
        .route("/api/admin/settings", get(get_settings).put(put_settings))
        .route("/api/categories", get(list_cats).post(create_cat))
        .route("/api/categories/:id", put(update_cat).delete(delete_cat))
        .route("/api/admin/items/batch", post(batch_items))
        .route("/api/admin/items/reorder", post(reorder_items))
        .route("/api/admin/check", post(trigger_check))
        .route("/api/admin/dashboard", get(dashboard))
        .route("/api/admin/stats", get(stats))
        .route(
            "/api/admin/blacklist",
            get(list_black).post(add_black).delete(del_black),
        )
        .route(
            "/api/admin/upload",
            post(upload).layer(DefaultBodyLimit::max(max_mb * 1024 * 1024)),
        )
        .route("/files/:name", get(serve_file))
        .route("/go/:id", get(go))
        .route("/healthz", get(healthz))
        .route("/metrics", get(metrics))
        .route("/admin", get(|| async { Redirect::temporary("/admin.html") }))
        .fallback(static_handler)
        .layer(middleware::from_fn_with_state(state.clone(), guard))
        .with_state(state);

    let l = tokio::net::TcpListener::bind(&addr).await.expect("端口绑定失败");
    println!("listening on {addr}");
    axum::serve(l, app.into_make_service_with_connect_info::<SocketAddr>())
        .await
        .unwrap();
}
