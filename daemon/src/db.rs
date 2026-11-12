use crate::config::Config;
use crate::error::{Error, Result};
use mysql_async::prelude::*;
use mysql_async::{params, Pool, Row};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Clone)]
pub struct Db {
    pool: Pool,
    cfg: Config,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageMapRow {
    pub id: i64,
    pub attachment_id: Option<i64>,
    pub post_id: Option<i64>,
    pub origin_url: String,
    pub origin_url_hash: String,
    pub blogger_url: Option<String>,
    pub blogger_image_id: Option<String>,
    pub transformed_params: Option<String>,
    pub status: String,
    pub redirect_active: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WpPostImage {
    pub attachment_id: Option<i64>,
    pub origin_url: String,
    pub local_path: Option<String>,
}

impl Db {
    pub fn new(cfg: Config) -> Result<Self> {
        let pool = Pool::new(cfg.mysql_opts());
        Ok(Self { pool, cfg })
    }

    pub fn pool(&self) -> &Pool { &self.pool }
    pub fn cfg(&self) -> &Config { &self.cfg }

    pub async fn ping(&self) -> Result<()> {
        let mut conn = self.pool.get_conn().await?;
        conn.query_drop("SELECT 1").await?;
        Ok(())
    }

    pub fn hash_url(url: &str) -> String {
        let norm = Self::normalize_url(url);
        let mut h = Sha256::new();
        h.update(norm.as_bytes());
        hex::encode(h.finalize())
    }

    pub fn normalize_url(url: &str) -> String {
        if let Ok(u) = url::Url::parse(url) {
            let mut s = format!("{}://{}{}", u.scheme().to_lowercase(), u.host_str().unwrap_or("").to_lowercase(), u.path());
            if let Some(q) = u.query() { s.push('?'); s.push_str(q); }
            s
        } else { url.to_string() }
    }

    fn images_table(&self) -> String { self.cfg.t("wp_opt_agents_images") }
    fn posts_table(&self) -> String { self.cfg.t("posts") }
    fn postmeta_table(&self) -> String { self.cfg.t("postmeta") }

    pub async fn ensure_post_images_registered(&self, post_id: i64) -> Result<Vec<ImageMapRow>> {
        let images = self.extract_post_images(post_id).await?;
        for img in images {
            self.upsert_image_mapping(post_id, img.attachment_id, &img.origin_url, img.local_path.as_deref()).await?;
        }
        self.list_images_for_post(post_id).await
    }

    pub async fn extract_post_images(&self, post_id: i64) -> Result<Vec<WpPostImage>> {
        let mut conn = self.pool.get_conn().await?;
        let row: Option<(String,)> = conn.exec_first(
            format!("SELECT post_content FROM {} WHERE ID = :id", self.posts_table()),
            params! { "id" => post_id },
        ).await?;
        let content = row.map(|r| r.0).unwrap_or_default();
        let mut out = Vec::new();
        let re = regex::Regex::new(r#"(?i)<img[^>]+src=["']([^"']+)["']"#).map_err(|e| Error::Other(e.to_string()))?;
        let uploads_base = format!("{}/wp-content/uploads/", self.cfg.site.url.trim_end_matches('/'));
        for cap in re.captures_iter(&content) {
            let url = cap.get(1).map(|m| m.as_str().to_string()).unwrap_or_default();
            if url.is_empty() { continue; }
            if !url.contains("/wp-content/uploads/") && !url.starts_with(&uploads_base) { continue; }
            let local_path = self.url_to_local_path(&url);
            let attachment_id = self.find_attachment_id_by_url(&url).await.ok().flatten();
            out.push(WpPostImage { attachment_id, origin_url: url, local_path });
        }
        // de-dup by hash
        out.sort_by(|a,b| Self::hash_url(&a.origin_url).cmp(&Self::hash_url(&b.origin_url)));
        out.dedup_by(|a,b| Self::hash_url(&a.origin_url) == Self::hash_url(&b.origin_url));
        Ok(out)
    }

    fn url_to_local_path(&self, url: &str) -> Option<String> {
        // map /wp-content/uploads/... to uploads_dir
        if let Some(idx) = url.find("/wp-content/uploads/") {
            let rel = &url[idx + "/wp-content/uploads/".len()..];
            let p = std::path::Path::new(&self.cfg.site.uploads_dir).join(rel.split('?').next().unwrap_or(rel));
            return Some(p.to_string_lossy().to_string());
        }
        None
    }

    pub async fn find_attachment_id_by_url(&self, url: &str) -> Result<Option<i64>> {
        let mut conn = self.pool.get_conn().await?;
        // try exact guid
        let row: Option<(i64,)> = conn.exec_first(
            format!("SELECT ID FROM {} WHERE guid = :g LIMIT 1", self.posts_table()),
            params!{ "g" => url },
        ).await?;
        if let Some((id,)) = row { return Ok(Some(id)); }
        // try basename match in attachment meta (best-effort)
        let name = url.split('/').last().unwrap_or("").split('?').next().unwrap_or("");
        if name.is_empty() { return Ok(None); }
        let like = format!("%{}%", name);
        let row: Option<(i64,)> = conn.exec_first(
            format!("SELECT post_id FROM {} WHERE meta_key='_wp_attached_file' AND meta_value LIKE :v ORDER BY meta_id DESC LIMIT 1", self.postmeta_table()),
            params!{ "v" => like },
        ).await?;
        Ok(row.map(|r| r.0))
    }

    pub async fn upsert_image_mapping(&self, post_id: i64, attachment_id: Option<i64>, origin_url: &str, local_path: Option<&str>) -> Result<i64> {
        let mut conn = self.pool.get_conn().await?;
        let hash = Self::hash_url(origin_url);
        let (w,h,mime,size) = if let Some(p) = local_path { image_meta_from_path(p) } else { (None,None,None,None) };
        let sql = format!(r#"
            INSERT INTO {} (attachment_id, post_id, origin_url, origin_url_hash, width, height, mime_type, original_size_bytes, status, metadata)
            VALUES (:attachment_id, :post_id, :origin_url, :hash, :w, :h, :mime, :size, 'pending', :meta)
            ON DUPLICATE KEY UPDATE
              post_id = COALESCE(VALUES(post_id), post_id),
              attachment_id = COALESCE(VALUES(attachment_id), attachment_id),
              width = COALESCE(VALUES(width), width),
              height = COALESCE(VALUES(height), height),
              mime_type = COALESCE(VALUES(mime_type), mime_type),
              original_size_bytes = COALESCE(VALUES(original_size_bytes), original_size_bytes),
              updated_at = CURRENT_TIMESTAMP
        "#, self.images_table());
        conn.exec_drop(sql, params!{
            "attachment_id" => attachment_id,
            "post_id" => post_id,
            "origin_url" => origin_url,
            "hash" => hash.clone(),
            "w" => w,
            "h" => h,
            "mime" => mime,
            "size" => size,
            "meta" => serde_json::json!({"local_path": local_path}).to_string(),
        }).await?;
        let row: Option<(i64,)> = conn.exec_first(
            format!("SELECT id FROM {} WHERE origin_url_hash=:h LIMIT 1", self.images_table()),
            params!{"h"=>hash}
        ).await?;
        row.map(|r| r.0).ok_or_else(|| Error::DatabaseError("failed to upsert image mapping".into()))
    }

    pub async fn list_images_for_post(&self, post_id: i64) -> Result<Vec<ImageMapRow>> {
        let mut conn = self.pool.get_conn().await?;
        let rows: Vec<Row> = conn.exec(
            format!("SELECT id, attachment_id, post_id, origin_url, origin_url_hash, blogger_url, blogger_image_id, transformed_params, status, redirect_active FROM {} WHERE post_id=:pid ORDER BY id ASC", self.images_table()),
            params!{"pid"=>post_id}
        ).await?;
        Ok(rows.into_iter().map(row_to_image).collect())
    }

    pub async fn list_pending_or_uploaded_images(&self, post_id: i64) -> Result<Vec<ImageMapRow>> {
        let mut conn = self.pool.get_conn().await?;
        let rows: Vec<Row> = conn.exec(
            format!("SELECT id, attachment_id, post_id, origin_url, origin_url_hash, blogger_url, blogger_image_id, transformed_params, status, redirect_active FROM {} WHERE post_id=:pid AND status IN ('pending','uploading','uploaded','failed') ORDER BY id ASC", self.images_table()),
            params!{"pid"=>post_id}
        ).await?;
        Ok(rows.into_iter().map(row_to_image).collect())
    }

    pub async fn mark_image_status(&self, image_id: i64, status: &str, err: Option<&str>) -> Result<()> {
        let mut conn = self.pool.get_conn().await?;
        conn.exec_drop(
            format!("UPDATE {} SET status=:s, error_message=:e, updated_at=CURRENT_TIMESTAMP WHERE id=:id", self.images_table()),
            params!{"s"=>status, "e"=>err, "id"=>image_id}
        ).await?;
        Ok(())
    }

    pub async fn mark_image_uploaded(&self, image_id: i64, blogger_url: &str, blogger_image_id: Option<&str>) -> Result<()> {
        let mut conn = self.pool.get_conn().await?;
        conn.exec_drop(
            format!("UPDATE {} SET blogger_url=:u, blogger_image_id=:bid, status='uploaded', error_message=NULL, updated_at=CURRENT_TIMESTAMP WHERE id=:id", self.images_table()),
            params!{"u"=>blogger_url, "bid"=>blogger_image_id, "id"=>image_id}
        ).await?;
        Ok(())
    }

    pub async fn mark_image_replaced(&self, image_id: i64) -> Result<()> {
        let mut conn = self.pool.get_conn().await?;
        conn.exec_drop(
            format!("UPDATE {} SET status='replaced', redirect_active=1, replaced_at=CURRENT_TIMESTAMP, updated_at=CURRENT_TIMESTAMP WHERE id=:id", self.images_table()),
            params!{"id"=>image_id}
        ).await?;
        Ok(())
    }

    pub async fn get_post_blogger_link(&self, post_id: i64) -> Result<Option<(String,String)>> {
        let mut conn = self.pool.get_conn().await?;
        let row: Option<(Option<String>, Option<String>)> = conn.exec_first(
            format!("SELECT MAX(CASE WHEN meta_key='_wpoa_blogger_post_id' THEN meta_value END), MAX(CASE WHEN meta_key='_wpoa_blogger_post_url' THEN meta_value END) FROM {} WHERE post_id=:pid", self.postmeta_table()),
            params!{"pid"=>post_id}
        ).await?;
        Ok(row.and_then(|(id,u)| match (id,u){ (Some(i), Some(u)) if !i.is_empty() || !u.is_empty() => Some((i,u)), _ => None }))
    }

    pub async fn save_post_blogger_link(&self, post_id: i64, blogger_post_id: &str, blogger_post_url: &str, blog_id: &str) -> Result<()> {
        self.upsert_postmeta(post_id, "_wpoa_blogger_post_id", blogger_post_id).await?;
        self.upsert_postmeta(post_id, "_wpoa_blogger_post_url", blogger_post_url).await?;
        if !blog_id.is_empty() { self.upsert_postmeta(post_id, "_wpoa_blogger_blog_id", blog_id).await?; }
        self.upsert_postmeta(post_id, "_wpoa_blogger_linked_at", &chrono::Utc::now().to_rfc3339()).await?;
        Ok(())
    }

    async fn upsert_postmeta(&self, post_id: i64, key: &str, value: &str) -> Result<()> {
        let mut conn = self.pool.get_conn().await?;
        let existing: Option<(i64,)> = conn.exec_first(
            format!("SELECT meta_id FROM {} WHERE post_id=:pid AND meta_key=:k LIMIT 1", self.postmeta_table()),
            params!{"pid"=>post_id, "k"=>key}
        ).await?;
        if let Some((mid,)) = existing {
            conn.exec_drop(format!("UPDATE {} SET meta_value=:v WHERE meta_id=:id", self.postmeta_table()), params!{"v"=>value, "id"=>mid}).await?;
        } else {
            conn.exec_drop(format!("INSERT INTO {} (post_id, meta_key, meta_value) VALUES (:pid,:k,:v)", self.postmeta_table()), params!{"pid"=>post_id,"k"=>key,"v"=>value}).await?;
        }
        Ok(())
    }

    pub async fn replace_urls_in_post_content(&self, post_id: i64, replacements: &[(String,String)], backup: bool) -> Result<usize> {
        let mut conn = self.pool.get_conn().await?;
        let row: Option<(String,)> = conn.exec_first(
            format!("SELECT post_content FROM {} WHERE ID=:id", self.posts_table()),
            params!{"id"=>post_id}
        ).await?;
        let Some((content,)) = row else { return Ok(0) };
        if backup {
            self.upsert_postmeta(post_id, "_wpoa_backup_post_content", &content).await?;
        }
        let mut new_content = content.clone();
        let mut applied = 0usize;
        for (from,to) in replacements {
            if new_content.contains(from) { new_content = new_content.replace(from, to); applied += 1; }
        }
        if applied > 0 {
            conn.exec_drop(format!("UPDATE {} SET post_content=:c WHERE ID=:id", self.posts_table()), params!{"c"=>new_content, "id"=>post_id}).await?;
        }
        Ok(applied)
    }

    pub async fn replace_urls_in_postmeta_and_options(&self, replacements: &[(String,String)]) -> Result<usize> {
        if replacements.is_empty() { return Ok(0); }
        let mut conn = self.pool.get_conn().await?;
        let mut total = 0usize;
        for (from,to) in replacements {
            let res = conn.exec_drop(
                format!("UPDATE {} SET meta_value = REPLACE(meta_value, :f, :t) WHERE meta_value LIKE :like", self.postmeta_table()),
                params!{"f"=>from, "t"=>to, "like"=>format!("%{}%", from)}
            ).await;
            if res.is_ok() { total += 1; }
            let _ = conn.exec_drop(
                format!("UPDATE {} SET option_value = REPLACE(option_value, :f, :t) WHERE option_value LIKE :like", self.cfg.t("options")),
                params!{"f"=>from, "t"=>to, "like"=>format!("%{}%", from)}
            ).await;
        }
        Ok(total)
    }

    pub async fn list_candidate_post_ids(&self, limit: usize) -> Result<Vec<i64>> {
        let mut conn = self.pool.get_conn().await?;
        let rows: Vec<(i64,)> = conn.exec(
            format!("SELECT ID FROM {} WHERE post_type='post' AND post_status IN ('publish','draft','future','private') ORDER BY ID DESC LIMIT :lim", self.posts_table()),
            params!{"lim"=> limit as u64}
        ).await?;
        Ok(rows.into_iter().map(|r| r.0).collect())
    }

    pub async fn get_post_title(&self, post_id: i64) -> Result<String> {
        let mut conn = self.pool.get_conn().await?;
        let row: Option<(String,)> = conn.exec_first(
            format!("SELECT post_title FROM {} WHERE ID=:id", self.posts_table()),
            params!{"id"=>post_id}
        ).await?;
        Ok(row.map(|r| r.0).unwrap_or_else(|| format!("Post {post_id}")))
    }
}

fn row_to_image(r: Row) -> ImageMapRow {
    ImageMapRow {
        id: r.get::<i64,_>("id").unwrap_or(0),
        attachment_id: r.get("attachment_id"),
        post_id: r.get("post_id"),
        origin_url: r.get::<String,_>("origin_url").unwrap_or_default(),
        origin_url_hash: r.get::<String,_>("origin_url_hash").unwrap_or_default(),
        blogger_url: r.get("blogger_url"),
        blogger_image_id: r.get("blogger_image_id"),
        transformed_params: r.get("transformed_params"),
        status: r.get::<String,_>("status").unwrap_or_else(|| "pending".into()),
        redirect_active: r.get::<i32,_>("redirect_active").unwrap_or(0),
    }
}

fn image_meta_from_path(p: &str) -> (Option<i32>, Option<i32>, Option<String>, Option<i64>) {
    let path = std::path::Path::new(p);
    if !path.exists() { return (None,None,None,None); }
    let size = std::fs::metadata(path).ok().map(|m| m.len() as i64);
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
    let mime = match ext.as_str() { "jpg"|"jpeg"=>"image/jpeg", "png"=>"image/png", "gif"=>"image/gif", "webp"=>"image/webp", _=>"application/octet-stream" }.to_string();
    // dimension best-effort skipped (avoid image crate dep)
    (None, None, Some(mime), size)
}
