use crate::models::*;
use crate::services::crypto::{decrypt, encrypt};
use crate::services::db::db;
use reqwest::header::{ACCEPT, CONTENT_TYPE, COOKIE, HeaderMap, HeaderValue, ORIGIN, REFERER, USER_AGENT};
use rusqlite::{OptionalExtension, params};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;

const BASE: &str = "https://pmvhaven.com/api";
const UA: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36";
const AUTH_PREFIXES: &[&str] = &["better-auth.", "__Secure-better-auth."];

#[derive(Debug, Error)]
pub enum PmvError {
    #[error("{0}")]
    Msg(String),
    #[error("HTTP {0}: {1}")]
    Status(u16, String),
    #[error(transparent)]
    Req(#[from] reqwest::Error),
}

pub type Result<T> = std::result::Result<T, PmvError>;

#[derive(Clone)]
pub struct PmvClient {
    http: reqwest::Client,
}

impl Default for PmvClient {
    fn default() -> Self {
        Self::new()
    }
}

impl PmvClient {
    pub fn new() -> Self {
        let http = reqwest::Client::builder()
            .user_agent(UA)
            .timeout(std::time::Duration::from_secs(30))
            .connect_timeout(std::time::Duration::from_secs(15))
            .redirect(reqwest::redirect::Policy::limited(5))
            .cookie_store(true)
            .build()
            .expect("http client");
        Self { http }
    }

    fn headers(&self, auth: bool, json_body: bool) -> HeaderMap {
        let mut h = HeaderMap::new();
        h.insert(USER_AGENT, HeaderValue::from_static(UA));
        h.insert(ACCEPT, HeaderValue::from_static("application/json"));
        h.insert(REFERER, HeaderValue::from_static("https://pmvhaven.com/"));
        h.insert(ORIGIN, HeaderValue::from_static("https://pmvhaven.com"));
        if json_body {
            h.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        }
        if auth {
            if let Some(cookie) = get_stored_cookie_header() {
                if let Ok(v) = HeaderValue::from_str(&cookie) {
                    h.insert(COOKIE, v);
                }
            }
        }
        h
    }

    async fn raw_request(
        &self,
        path: &str,
        method: reqwest::Method,
        auth: bool,
        query: &[(&str, String)],
        body: Option<Value>,
    ) -> Result<reqwest::Response> {
        let url = format!("{BASE}{path}");
        let mut req = self
            .http
            .request(method, &url)
            .headers(self.headers(auth, body.is_some()))
            .query(query);
        if let Some(b) = body {
            req = req.json(&b);
        }
        Ok(req.send().await?)
    }

    async fn request_json(
        &self,
        path: &str,
        method: reqwest::Method,
        auth: bool,
        query: &[(&str, String)],
        body: Option<Value>,
    ) -> Result<Value> {
        let mut res = self
            .raw_request(path, method.clone(), auth, query, body.clone())
            .await?;

        if auth && (res.status() == 401 || res.status() == 403) {
            if try_reauth(self).await {
                res = self
                    .raw_request(path, method, auth, query, body)
                    .await?;
            }
        }

        let status = res.status().as_u16();
        if !res.status().is_success() {
            let msg = res
                .json::<Value>()
                .await
                .ok()
                .and_then(|j| j.get("message")?.as_str().map(|s| s.to_string()))
                .unwrap_or_else(|| format!("PMVHaven request failed ({status})"));
            return Err(PmvError::Status(status, msg));
        }

        // Capture Set-Cookie on successful auth responses
        store_cookies_from_response(&res);
        Ok(res.json().await?)
    }

    pub async fn sign_in(&self, email: &str, password: &str) -> Result<AccountUser> {
        let body = serde_json::json!({
            "email": email,
            "password": password,
            "rememberMe": true,
        });
        tracing::info!("signing in to PMVHaven…");
        let res = self
            .raw_request(
                "/auth/sign-in/email",
                reqwest::Method::POST,
                false,
                &[],
                Some(body),
            )
            .await?;

        let status = res.status();
        let mut cookies = extract_auth_cookies(res.headers());
        let data: Value = res.json().await.map_err(|e| PmvError::Msg(e.to_string()))?;

        if !status.is_success() {
            let msg = data
                .get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("Invalid email or password");
            return Err(PmvError::Msg(msg.into()));
        }

        // Body may include a bare token; prefer full Set-Cookie values when present.
        if cookies.is_empty() {
            if let Some(token) = data.get("token").and_then(|t| t.as_str()) {
                // Better Auth cookie value is typically `token.signature`; bare token
                // alone is a last-resort fallback for same-process cookie_store use.
                cookies.insert(
                    "__Secure-better-auth.session_token".into(),
                    token.to_string(),
                );
            }
        }

        let now = chrono::Utc::now().timestamp_millis();
        // Encrypt before taking the DB lock — encrypt may load the crypto key
        // from settings, which also needs the DB mutex.
        let enc_email = encrypt(email);
        let enc_password = encrypt(password);
        let enc_cookies = if cookies.is_empty() {
            None
        } else {
            Some(encrypt(
                &serde_json::to_string(&cookies).unwrap_or_default(),
            ))
        };
        {
            let db = db();
            let conn = db.lock().map_err(|_| PmvError::Msg("db lock poisoned".into()))?;
            conn.execute(
                r#"INSERT INTO account (id, enc_email, enc_password, created_at, last_login_at, cookies)
                   VALUES (1, ?1, ?2, ?3, ?4, ?5)
                   ON CONFLICT(id) DO UPDATE SET enc_email = excluded.enc_email,
                     enc_password = excluded.enc_password, last_login_at = excluded.last_login_at,
                     cookies = COALESCE(excluded.cookies, account.cookies)"#,
                params![enc_email, enc_password, now, now, enc_cookies],
            )
            .map_err(|e| PmvError::Msg(format!("failed to store account: {e}")))?;
        }

        if let Some(u) = data.get("user").filter(|u| !u.is_null()) {
            let user = AccountUser {
                id: str_field(u, &["id", "_id", "customUserId"]),
                username: {
                    let n = str_field(u, &["username", "name"]);
                    if n.is_empty() {
                        email.to_string()
                    } else {
                        n
                    }
                },
                email: opt_str(u, "email").or_else(|| Some(email.to_string())),
                avatar_url: opt_str(u, "avatarUrl").or_else(|| opt_str(u, "image")),
            };
            {
                let db = db();
                let conn = db.lock().map_err(|_| PmvError::Msg("db lock poisoned".into()))?;
                let _ = conn.execute(
                    "UPDATE account SET pmv_user_id = ?1, username = ?2, email = ?3, avatar_url = ?4 WHERE id = 1",
                    params![user.id, user.username, user.email, user.avatar_url],
                );
            }
            tracing::info!("signed in as {}", user.username);
            let client = self.clone();
            tokio::spawn(async move {
                let _ = client.refresh_profile().await;
            });
            return Ok(user);
        }

        self.refresh_profile()
            .await
            .ok_or_else(|| PmvError::Msg("Signed in but profile missing".into()))
    }

    pub async fn sign_out(&self) {
        let _ = self
            .raw_request("/auth/sign-out", reqwest::Method::POST, true, &[], None)
            .await;
        let db = db();
        let conn = db.lock().unwrap();
        let _ = conn.execute("DELETE FROM account WHERE id = 1", []);
    }

    pub async fn refresh_profile(&self) -> Option<AccountUser> {
        let data = self
            .request_json("/auth/session", reqwest::Method::GET, true, &[], None)
            .await
            .ok()?;
        let u = data.get("user")?;
        if u.is_null() {
            return get_account_user();
        }
        let user = AccountUser {
            id: str_field(u, &["id", "_id"]),
            username: str_field(u, &["username", "name"]),
            email: opt_str(u, "email"),
            avatar_url: opt_str(u, "avatarUrl").or_else(|| opt_str(u, "image")),
        };
        {
            let db = db();
            let conn = db.lock().unwrap();
            let _ = conn.execute(
                "UPDATE account SET pmv_user_id = ?1, username = ?2, email = ?3, avatar_url = ?4 WHERE id = 1",
                params![user.id, user.username, user.email, user.avatar_url],
            );
        }
        Some(user)
    }

    pub async fn get_videos(&self, params: FeedParams) -> Result<Paged<VideoSummary>> {
        let owned = params.to_query();
        let auth = is_connected()
            && (params.watched_only
                || params.favorites_only
                || params.subscribed_only
                || params.personalized_only
                || params.exclude_watched);

        let mut req = self
            .http
            .get(format!("{BASE}/videos"))
            .headers(self.headers(auth, false));
        for (k, v) in &owned {
            req = req.query(&[(k.as_str(), v.as_str())]);
        }
        let res = req.send().await?;
        let status = res.status().as_u16();
        if !res.status().is_success() {
            return Err(PmvError::Status(status, format!("videos failed ({status})")));
        }
        let data: Value = res.json().await?;
        Ok(paged_from_list(&data))
    }

    pub async fn get_trending(&self) -> Result<Vec<VideoSummary>> {
        let data = self
            .request_json("/videos/trending", reqwest::Method::GET, false, &[], None)
            .await?;
        Ok(pick_list(&data).into_iter().map(normalize_summary).collect())
    }

    pub async fn get_video(&self, id: &str) -> Result<VideoDetail> {
        let auth = is_connected();
        let data = self
            .request_json(&format!("/videos/{id}"), reqwest::Method::GET, auth, &[], None)
            .await?;
        let raw = data.get("data").cloned().unwrap_or(data);
        Ok(normalize_detail(&raw))
    }

    pub async fn get_related(&self, id: &str) -> Result<Vec<VideoSummary>> {
        let data = self
            .request_json(
                &format!("/videos/{id}/related"),
                reqwest::Method::GET,
                false,
                &[],
                None,
            )
            .await?;
        Ok(pick_list(&data).into_iter().map(normalize_summary).collect())
    }

    pub async fn get_popular_tags(&self) -> Result<Vec<PopularTag>> {
        let data = self
            .request_json("/tags/popular", reqwest::Method::GET, false, &[], None)
            .await?;
        let arr = data
            .get("data")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        Ok(arr
            .iter()
            .map(|t| PopularTag {
                name: str_field(t, &["name"]),
                usage_count: num_field(t, "usageCount") as u64,
            })
            .collect())
    }

    pub async fn search(&self, q: &str, page: u32, limit: u32) -> Result<Paged<VideoSummary>> {
        // Site text search is `/api/videos/search?q=` (public).
        // `/api/search` is a separate API-key endpoint and is not what the website uses.
        let owned = [
            ("q".to_string(), q.to_string()),
            ("page".to_string(), page.to_string()),
            ("limit".to_string(), limit.to_string()),
        ];
        let mut req = self
            .http
            .get(format!("{BASE}/videos/search"))
            .headers(self.headers(is_connected(), false));
        for (k, v) in &owned {
            req = req.query(&[(k.as_str(), v.as_str())]);
        }
        let res = req.send().await?;
        let status = res.status().as_u16();
        if !res.status().is_success() {
            return Err(PmvError::Status(status, format!("search failed ({status})")));
        }
        let data: Value = res.json().await?;
        Ok(paged_from_list(&data))
    }

    pub async fn fetch_remote_history(&self) -> Result<RemoteHistory> {
        let data = self
            .request_json("/auth/session", reqwest::Method::GET, true, &[], None)
            .await?;
        let Some(u) = data.get("user").filter(|u| !u.is_null()) else {
            return Ok(RemoteHistory {
                total_retained: 0,
                entries: vec![],
            });
        };

        let mut progress_by: HashMap<String, f64> = HashMap::new();
        if let Some(arr) = u.get("watchProgress").and_then(|v| v.as_array()) {
            for p in arr {
                let id = str_field(p, &["videoId"]);
                if id.is_empty() {
                    continue;
                }
                let dur = num_field(p, "duration");
                let secs = num_field(p, "progress");
                let fraction = if dur > 0.0 {
                    (secs / dur).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                progress_by.insert(id, fraction);
            }
        }

        let entries = u
            .get("watchHistory")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default()
            .iter()
            .filter_map(|h| {
                let id = str_field(h, &["videoId"]);
                if id.is_empty() {
                    return None;
                }
                let watched_at = str_field(h, &["watchedAt"]);
                let progress = progress_by.get(&id).copied().unwrap_or(0.0);
                Some(RemoteHistoryEntry {
                    video_id: id,
                    watched_at: if watched_at.is_empty() {
                        chrono::Utc::now().to_rfc3339()
                    } else {
                        watched_at
                    },
                    progress,
                })
            })
            .collect::<Vec<_>>();

        let total_retained = u
            .get("watchHistoryCount")
            .and_then(|v| v.as_u64())
            .unwrap_or(entries.len() as u64);

        Ok(RemoteHistory {
            total_retained,
            entries,
        })
    }

    pub async fn get_videos_bulk(&self, ids: &[String]) -> Result<Vec<VideoSummary>> {
        let mut out = Vec::new();
        for chunk in ids.chunks(100) {
            if chunk.is_empty() {
                continue;
            }
            let joined = chunk.join(",");
            let mut req = self
                .http
                .get(format!("{BASE}/videos/bulk"))
                .headers(self.headers(false, false))
                .query(&[("ids", joined.as_str())]);
            let res = req.send().await?;
            if !res.status().is_success() {
                continue;
            }
            let data: Value = res.json().await?;
            out.extend(pick_list(&data).into_iter().map(normalize_summary));
        }
        Ok(out)
    }

    pub async fn get_remote_favorites(&self, limit: u32) -> Result<Vec<VideoSummary>> {
        let mut req = self
            .http
            .get(format!("{BASE}/user/favorites"))
            .headers(self.headers(true, false))
            .query(&[("limit", limit.to_string())]);
        let res = req.send().await?;
        let data: Value = res.json().await?;
        Ok(pick_list(&data).into_iter().map(normalize_summary).collect())
    }

    pub async fn get_remote_watch_later(&self, limit: u32) -> Result<Vec<VideoSummary>> {
        let mut req = self
            .http
            .get(format!("{BASE}/user/watch-later"))
            .headers(self.headers(true, false))
            .query(&[("limit", limit.to_string())]);
        let res = req.send().await?;
        let data: Value = res.json().await?;
        Ok(pick_list(&data).into_iter().map(normalize_summary).collect())
    }

    pub async fn set_favorite(&self, id: &str, on: bool) -> Result<()> {
        let method = if on {
            reqwest::Method::POST
        } else {
            reqwest::Method::DELETE
        };
        let _ = self
            .request_json(&format!("/videos/{id}/favorite"), method, true, &[], None)
            .await?;
        Ok(())
    }

    pub async fn set_watch_later(&self, id: &str, on: bool) -> Result<()> {
        let method = if on {
            reqwest::Method::POST
        } else {
            reqwest::Method::DELETE
        };
        let _ = self
            .request_json(
                &format!("/videos/{id}/watch-later"),
                method,
                true,
                &[],
                None,
            )
            .await?;
        Ok(())
    }

    /// Bump remote view + add to PMVHaven watch history (authenticated when possible).
    pub async fn record_view(&self, id: &str) {
        let auth = is_connected();
        // Site player: POST /api/videos/:id/view
        let _ = self
            .raw_request(
                &format!("/videos/{id}/view"),
                reqwest::Method::POST,
                auth,
                &[],
                Some(serde_json::json!({
                    "source": "pmvheaven-desktop",
                    "referrer": "app",
                })),
            )
            .await;
        // Legacy fallback used by v1.
        let _ = self
            .raw_request(
                &format!("/videos/{id}"),
                reqwest::Method::POST,
                auth,
                &[],
                None,
            )
            .await;
    }

    /// Push watch progress to PMVHaven (`PUT /api/users/watch-progress`).
    /// `progress_pct` is 0–100 (site convention); `duration_secs` is video length.
    pub async fn push_watch_progress(
        &self,
        video_id: &str,
        progress_pct: u32,
        duration_secs: u32,
    ) -> Result<()> {
        if !is_connected() {
            return Err(PmvError::Msg("Not signed in".into()));
        }
        let pct = progress_pct.min(100);
        self.request_json(
            "/users/watch-progress",
            reqwest::Method::PUT,
            true,
            &[],
            Some(serde_json::json!({
                "videoId": video_id,
                "progress": pct,
                "duration": duration_secs,
            })),
        )
        .await?;
        Ok(())
    }
}

/* ----------------------------- account helpers ----------------------------- */

struct AccountRow {
    username: Option<String>,
    email: Option<String>,
    avatar_url: Option<String>,
    pmv_user_id: Option<String>,
    enc_email: Option<String>,
    enc_password: Option<String>,
    cookies: Option<String>,
}

fn read_account() -> Option<AccountRow> {
    let db = db();
    let conn = db.lock().unwrap();
    conn.query_row("SELECT * FROM account WHERE id = 1", [], |r| {
        Ok(AccountRow {
            username: r.get("username")?,
            email: r.get("email")?,
            avatar_url: r.get("avatar_url")?,
            pmv_user_id: r.get("pmv_user_id")?,
            enc_email: r.get("enc_email")?,
            enc_password: r.get("enc_password")?,
            cookies: r.get("cookies")?,
        })
    })
    .optional()
    .ok()
    .flatten()
}

pub fn get_account_user() -> Option<AccountUser> {
    let row = read_account()?;
    if row.cookies.is_none() || row.username.is_none() {
        return None;
    }
    Some(AccountUser {
        id: row.pmv_user_id.unwrap_or_default(),
        username: row.username.unwrap_or_default(),
        email: row.email,
        avatar_url: row.avatar_url,
    })
}

pub fn is_connected() -> bool {
    read_account()
        .map(|r| r.cookies.is_some())
        .unwrap_or(false)
}

fn get_stored_cookie_header() -> Option<String> {
    let row = read_account()?;
    let cookies = row.cookies?;
    let plain = decrypt(&cookies).ok()?;
    let map: HashMap<String, String> = serde_json::from_str(&plain).ok()?;
    let pairs: Vec<String> = map.iter().map(|(k, v)| format!("{k}={v}")).collect();
    if pairs.is_empty() {
        None
    } else {
        Some(pairs.join("; "))
    }
}

fn extract_auth_cookies(headers: &reqwest::header::HeaderMap) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for val in headers.get_all(reqwest::header::SET_COOKIE) {
        if let Ok(line) = val.to_str() {
            let first = line.split(';').next().unwrap_or("");
            if let Some((name, value)) = first.split_once('=') {
                let name = name.trim();
                if AUTH_PREFIXES.iter().any(|p| name.starts_with(p)) {
                    map.insert(name.to_string(), value.trim().to_string());
                }
            }
        }
    }
    map
}

fn persist_cookie_map(map: HashMap<String, String>) {
    if map.is_empty() {
        return;
    }
    if let Ok(json) = serde_json::to_string(&map) {
        let enc = encrypt(&json);
        let db = db();
        let conn = match db.lock() {
            Ok(c) => c,
            Err(_) => return,
        };
        let _ = conn.execute(
            "UPDATE account SET cookies = ?1 WHERE id = 1",
            params![enc],
        );
    }
}

fn store_cookies_from_response(res: &reqwest::Response) {
    let mut map = HashMap::new();
    if let Some(existing) = get_stored_cookie_header() {
        for pair in existing.split("; ") {
            if let Some((k, v)) = pair.split_once('=') {
                map.insert(k.to_string(), v.to_string());
            }
        }
    }
    map.extend(extract_auth_cookies(res.headers()));
    persist_cookie_map(map);
}

async fn try_reauth(client: &PmvClient) -> bool {
    let Some(row) = read_account() else {
        return false;
    };
    let (Some(enc_email), Some(enc_password)) = (row.enc_email, row.enc_password) else {
        return false;
    };
    let Ok(email) = decrypt(&enc_email) else {
        return false;
    };
    let Ok(password) = decrypt(&enc_password) else {
        return false;
    };
    let body = serde_json::json!({
        "email": email,
        "password": password,
        "rememberMe": true,
    });
    let Ok(res) = client
        .raw_request(
            "/auth/sign-in/email",
            reqwest::Method::POST,
            false,
            &[],
            Some(body),
        )
        .await
    else {
        return false;
    };
    if !res.status().is_success() {
        return false;
    }
    store_cookies_from_response(&res);
    let db = db();
    let conn = db.lock().unwrap();
    let _ = conn.execute(
        "UPDATE account SET last_login_at = ?1 WHERE id = 1",
        params![chrono::Utc::now().timestamp_millis()],
    );
    true
}

/* ----------------------------- normalizers ----------------------------- */

fn num_field(v: &Value, key: &str) -> f64 {
    v.get(key)
        .and_then(|x| x.as_f64().or_else(|| x.as_i64().map(|i| i as f64)))
        .unwrap_or(0.0)
}

fn str_field(v: &Value, keys: &[&str]) -> String {
    for k in keys {
        if let Some(s) = v.get(*k).and_then(|x| x.as_str()) {
            if !s.is_empty() {
                return s.to_string();
            }
        }
    }
    String::new()
}

fn opt_str(v: &Value, key: &str) -> Option<String> {
    v.get(key)
        .and_then(|x| x.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
}

fn arr_str(v: &Value, key: &str) -> Vec<String> {
    v.get(key)
        .and_then(|x| x.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|i| i.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default()
}

fn pick_list(r: &Value) -> Vec<Value> {
    r.get("data")
        .or_else(|| r.get("videos"))
        .or_else(|| r.get("results"))
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default()
}

fn normalize_summary(v: Value) -> VideoSummary {
    let tags = {
        let t = arr_str(&v, "tags");
        if t.is_empty() {
            arr_str(&v, "top5Tags")
        } else {
            t
        }
    };
    VideoSummary {
        id: str_field(&v, &["_id", "id"]),
        title: {
            let t = str_field(&v, &["title"]);
            if t.is_empty() {
                "Untitled".into()
            } else {
                t
            }
        },
        uploader: str_field(&v, &["uploader"]),
        uploader_username: {
            let u = str_field(&v, &["uploaderUsername"]);
            if u.is_empty() {
                str_field(&v, &["uploader"])
            } else {
                u
            }
        },
        thumbnail_url: str_field(&v, &["thumbnailUrl"]),
        preview_url: opt_str(&v, "previewUrl"),
        views: num_field(&v, "views") as u64,
        duration: str_field(&v, &["duration"]),
        duration_seconds: num_field(&v, "durationSeconds") as u32,
        aspect_ratio: {
            let a = num_field(&v, "aspectRatio");
            if a == 0.0 { 1.7778 } else { a }
        },
        likes: num_field(&v, "likes") as u64,
        dislikes: num_field(&v, "dislikes") as u64,
        rating: num_field(&v, "bayesianRating"),
        tags,
        upload_date: {
            let d = str_field(&v, &["uploadDate"]);
            if d.is_empty() {
                str_field(&v, &["releaseDate"])
            } else {
                d
            }
        },
        is_remix: v.get("isRemix").and_then(|x| x.as_bool()).unwrap_or(false),
        has_voice_over: v
            .get("hasVoiceOver")
            .and_then(|x| x.as_bool())
            .unwrap_or(false),
        has_extreme_content: v
            .get("hasExtremeContent")
            .and_then(|x| x.as_bool())
            .unwrap_or(false),
    }
}

fn normalize_detail(v: &Value) -> VideoDetail {
    let summary = normalize_summary(v.clone());
    let hls_variants = v
        .get("hlsVariants")
        .and_then(|x| x.as_array())
        .cloned()
        .unwrap_or_default()
        .iter()
        .map(|h| HlsVariant {
            resolution: str_field(h, &["resolution"]),
            width: num_field(h, "width") as u32,
            height: num_field(h, "height") as u32,
            bandwidth: num_field(h, "bandwidth") as u32,
            playlist_url: str_field(h, &["playlistUrl"]),
        })
        .collect();
    let music = v
        .get("music")
        .and_then(|x| x.as_array())
        .cloned()
        .unwrap_or_default()
        .iter()
        .map(|m| MusicTrack {
            artist: str_field(m, &["artist"]),
            song: str_field(m, &["song"]),
        })
        .collect();
    let timeline_thumbnails = v
        .get("timelineThumbnails")
        .and_then(|x| x.as_array())
        .cloned()
        .unwrap_or_default()
        .iter()
        .map(|t| TimelineThumbnail {
            url: str_field(t, &["url"]),
            capture_time: num_field(t, "captureTime"),
        })
        .collect();

    VideoDetail {
        summary,
        description: str_field(v, &["description"]),
        video_url: str_field(v, &["videoUrl"]),
        hls_master_playlist_url: opt_str(v, "hlsMasterPlaylistUrl"),
        hls_enabled: v.get("hlsEnabled").and_then(|x| x.as_bool()).unwrap_or(false),
        hls_variants,
        width: num_field(v, "width") as u32,
        height: num_field(v, "height") as u32,
        favorites: num_field(v, "favorites") as u64,
        creator: arr_str(v, "creator"),
        stars: arr_str(v, "starsTags"),
        music,
        timeline_thumbnails,
        uploader_avatar_url: opt_str(v, "uploaderAvatarUrl"),
        uploader_id: opt_str(v, "uploaderId"),
        watch_progress: num_field(v, "watchProgress"),
        is_liked: v.get("isLiked").and_then(|x| x.as_bool()).unwrap_or(false),
        is_disliked: v.get("isDisliked").and_then(|x| x.as_bool()).unwrap_or(false),
        is_favorited: v
            .get("isFavorited")
            .and_then(|x| x.as_bool())
            .unwrap_or(false),
        is_watch_later: v
            .get("isWatchLater")
            .and_then(|x| x.as_bool())
            .unwrap_or(false),
    }
}

fn paged_from_list(data: &Value) -> Paged<VideoSummary> {
    let list = pick_list(data);
    let items: Vec<_> = list.into_iter().map(normalize_summary).collect();
    let p = data.get("pagination");
    let len = items.len() as u64;
    Paged {
        pagination: Pagination {
            page: p
                .and_then(|x| x.get("page"))
                .and_then(|x| x.as_u64())
                .unwrap_or(1) as u32,
            limit: p
                .and_then(|x| x.get("limit"))
                .and_then(|x| x.as_u64())
                .unwrap_or(len) as u32,
            total: p
                .and_then(|x| x.get("total").or_else(|| x.get("totalVideos")))
                .and_then(|x| x.as_u64())
                .unwrap_or(len),
            total_pages: p
                .and_then(|x| x.get("totalPages"))
                .and_then(|x| x.as_u64())
                .unwrap_or(1) as u32,
            has_next: p
                .and_then(|x| x.get("hasNext"))
                .and_then(|x| x.as_bool())
                .unwrap_or(false),
            has_prev: p
                .and_then(|x| x.get("hasPrev"))
                .and_then(|x| x.as_bool())
                .unwrap_or(false),
        },
        items,
    }
}

pub fn shared_client() -> Arc<PmvClient> {
    use once_cell::sync::Lazy;
    static CLIENT: Lazy<Arc<PmvClient>> = Lazy::new(|| Arc::new(PmvClient::new()));
    CLIENT.clone()
}
