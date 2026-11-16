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
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub mime_type: Option<String>,
    pub original_size_bytes: Option<i64>,
    pub offload_size_bytes: Option<i64>,
    pub status: String,
    pub redirect_active: i32,
    pub error_message: Option<String>,
    pub metadata: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub replaced_at: Option<String>,
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

    pub async fn get_stats(&self) -> Result<serde_json::Value> {
        let mut conn = self.pool.get_conn().await?;
        let total: Option<(u64,)> = conn.exec_first(
            format!("SELECT COUNT(*) FROM {}", self.images_table()),
            ()
        ).await?;
        let pending: Option<(u64,)> = conn.exec_first(
            format!("SELECT COUNT(*) FROM {} WHERE status='pending'", self.images_table()),
            ()
        ).await?;
        let uploading: Option<(u64,)> = conn.exec_first(
            format!("SELECT COUNT(*) FROM {} WHERE status='uploading'", self.images_table()),
            ()
        ).await?;
        let uploaded: Option<(u64,)> = conn.exec_first(
            format!("SELECT COUNT(*) FROM {} WHERE status='uploaded'", self.images_table()),
            ()
        ).await?;
        let replacing: Option<(u64,)> = conn.exec_first(
            format!("SELECT COUNT(*) FROM {} WHERE status='replacing'", self.images_table()),
            ()
        ).await?;
        let replaced: Option<(u64,)> = conn.exec_first(
            format!("SELECT COUNT(*) FROM {} WHERE status='replaced'", self.images_table()),
            ()
        ).await?;
        let failed: Option<(u64,)> = conn.exec_first(
            format!("SELECT COUNT(*) FROM {} WHERE status='failed'", self.images_table()),
            ()
        ).await?;

        Ok(serde_json::json!({
            "total": total.map(|r| r.0).unwrap_or(0),
            "pending": pending.map(|r| r.0).unwrap_or(0),
            "uploading": uploading.map(|r| r.0).unwrap_or(0),
            "uploaded": uploaded.map(|r| r.0).unwrap_or(0),
            "replacing": replacing.map(|r| r.0).unwrap_or(0),
            "replaced": replaced.map(|r| r.0).unwrap_or(0),
            "failed": failed.map(|r| r.0).unwrap_or(0),
        }))
    }

    pub async fn list_images(&self, limit: usize, offset: usize, status_filter: Option<String>) -> Result<Vec<ImageMapRow>> {
        let mut conn = self.pool.get_conn().await?;
        let status_cond = status_filter.map(|s| format!("WHERE status = '{}'", s)).unwrap_or_default();
        let rows: Vec<Row> = conn.exec(
            format!("SELECT * FROM {} {} ORDER BY id DESC LIMIT :lim OFFSET :off", self.images_table(), status_cond),
            params!{"lim"=> limit as u64, "off"=> offset as u64}
        ).await?;
        Ok(rows.into_iter().map(row_to_image).collect())
    }

    pub async fn list_posts_with_stats(&self, limit: usize, offset: usize) -> Result<Vec<PostStatsRow>> {
        let mut conn = self.pool.get_conn().await?;
        // Include any post types that use images (posts, pages, products, custom CPTs), excluding revisions/nav items.
        let rows: Vec<Row> = conn.exec(
            format!(
                "SELECT p.ID, p.post_title, p.post_status, p.post_type,
                        COUNT(i.id) as total_images,
                        SUM(CASE WHEN i.status IN ('uploaded','replaced') THEN 1 ELSE 0 END) as offloaded_images,
                        SUM(CASE WHEN i.status = 'replaced' THEN 1 ELSE 0 END) as replaced_images,
                        MAX(CASE WHEN pm.meta_key='_wpoa_blogger_post_id' THEN pm.meta_value END) as blogger_post_id,
                        MAX(CASE WHEN pm.meta_key='_wpoa_blogger_post_url' THEN pm.meta_value END) as blogger_post_url,
                        MAX(CASE WHEN pm.meta_key='_wpoa_blogger_blog_id' THEN pm.meta_value END) as blogger_blog_id,
                        MAX(CASE WHEN pm.meta_key='_wpoa_blogger_linked_at' THEN pm.meta_value END) as linked_at
                 FROM {} p
                 LEFT JOIN {} i ON i.post_id = p.ID
                 LEFT JOIN {} pm ON pm.post_id = p.ID AND pm.meta_key LIKE '_wpoa_blogger_%'
                 WHERE p.post_type NOT IN ('attachment','revision','nav_menu_item','shop_order','shop_order_refund')
                   AND p.post_status NOT IN ('auto-draft','trash')
                 GROUP BY p.ID
                 ORDER BY p.ID DESC
                 LIMIT :lim OFFSET :off",
                self.posts_table(), self.images_table(), self.postmeta_table()
            ),
            params!{"lim"=> limit as u64, "off"=> offset as u64}
        ).await?;

        Ok(rows.into_iter().map(|r| PostStatsRow {
            id: r.get::<i64,_>("ID").unwrap_or(0),
            title: r.get::<String,_>("post_title").unwrap_or_default(),
            status: r.get::<String,_>("post_status").unwrap_or_else(|| "publish".into()),
            post_type: r.get::<String,_>("post_type").unwrap_or_else(|| "post".into()),
            total_images: r.get::<u64,_>("total_images").unwrap_or(0) as i64,
            offloaded_images: r.get::<u64,_>("offloaded_images").unwrap_or(0) as i64,
            replaced_images: r.get::<u64,_>("replaced_images").unwrap_or(0) as i64,
            blogger_post_id: r.get::<Option<String>,_>("blogger_post_id").unwrap_or(None),
            blogger_post_url: r.get::<Option<String>,_>("blogger_post_url").unwrap_or(None),
            blogger_blog_id: r.get::<Option<String>,_>("blogger_blog_id").unwrap_or(None),
            linked_at: r.get::<Option<String>,_>("linked_at").unwrap_or(None),
        }).collect())
    }

    // --- Pipeline tables for imgbb backup + blogger queue + URL version history ---
    pub fn url_versions_table(&self) -> String { self.cfg.t("wp_opt_agents_url_versions") }
    pub fn blogger_queue_table(&self) -> String { self.cfg.t("wp_opt_agents_blogger_queue") }
    pub fn cdn_map_table(&self) -> String { self.cfg.t("wp_opt_agents_cdn_map") }

    pub async fn ensure_pipeline_tables(&self) -> Result<()> {
        let mut conn = self.pool.get_conn().await?;
        conn.query_drop(format!(r#"
            CREATE TABLE IF NOT EXISTS {} (
                id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
                attachment_id BIGINT UNSIGNED NOT NULL,
                image_size VARCHAR(64) NOT NULL,
                provider VARCHAR(32) NOT NULL,
                old_url TEXT NOT NULL,
                old_url_hash CHAR(64) NOT NULL,
                new_url TEXT NOT NULL,
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                PRIMARY KEY (id),
                KEY idx_attachment_size (attachment_id, image_size),
                KEY idx_provider (provider),
                KEY idx_old_hash (old_url_hash)
            ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4
        "#, self.url_versions_table())).await?;

        conn.query_drop(format!(r#"
            CREATE TABLE IF NOT EXISTS {} (
                id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
                attachment_id BIGINT UNSIGNED NOT NULL,
                post_id BIGINT UNSIGNED DEFAULT NULL,
                image_size VARCHAR(64) NOT NULL,
                source_url TEXT NOT NULL,
                local_path TEXT DEFAULT NULL,
                status VARCHAR(32) NOT NULL DEFAULT 'queued',
                attempts INT NOT NULL DEFAULT 0,
                last_error TEXT DEFAULT NULL,
                blogger_url TEXT DEFAULT NULL,
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
                PRIMARY KEY (id),
                KEY idx_status (status),
                KEY idx_attachment_size (attachment_id, image_size),
                KEY idx_post_id (post_id)
            ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4
        "#, self.blogger_queue_table())).await?;

        conn.query_drop(format!(r#"
            CREATE TABLE IF NOT EXISTS {} (
                id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT,
                attachment_id BIGINT UNSIGNED NOT NULL,
                image_size VARCHAR(64) NOT NULL,
                wp_url TEXT NOT NULL,
                wp_path VARCHAR(1024) NOT NULL,
                cdn_path VARCHAR(1024) NOT NULL,
                cdn_url TEXT NOT NULL,
                target_provider VARCHAR(32) NOT NULL,
                target_url TEXT NOT NULL,
                agent_uuid VARCHAR(64) NOT NULL,
                is_active TINYINT(1) NOT NULL DEFAULT 1,
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
                PRIMARY KEY (id),
                UNIQUE KEY uniq_attachment_size (attachment_id, image_size),
                UNIQUE KEY uniq_cdn_path (cdn_path(191)),
                KEY idx_wp_path (wp_path(191)),
                KEY idx_provider (target_provider)
            ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4
        "#, self.cdn_map_table())).await?;
        Ok(())
    }

    pub async fn insert_url_version(&self, attachment_id: i64, image_size: &str, provider: &str, old_url: &str, new_url: &str) -> Result<()> {
        let mut conn = self.pool.get_conn().await?;
        let old_hash = Self::hash_url(old_url);
        conn.exec_drop(
            format!("INSERT INTO {} (attachment_id, image_size, provider, old_url, old_url_hash, new_url) VALUES (:aid,:sz,:p,:o,:oh,:n)", self.url_versions_table()),
            params!{"aid"=>attachment_id, "sz"=>image_size, "p"=>provider, "o"=>old_url, "oh"=>old_hash, "n"=>new_url}
        ).await?;
        Ok(())
    }

    pub async fn upsert_cdn_mapping(&self, attachment_id: i64, image_size: &str, wp_url: &str, target_url: &str, provider: &str) -> Result<String> {
        let mut conn = self.pool.get_conn().await?;
        let wp_path = Self::uploads_rel_path_from_url(wp_url).unwrap_or_else(|| wp_url.trim_start_matches('/').to_string());
        let agent_uuid = Self::short_uuid_for_url(wp_url, image_size);
        let cdn_path = Self::pretty_cdn_path(&wp_path, &agent_uuid);
        let cdn_url = format!("{}://{}/{}", self.cfg.cdn.scheme.trim_end_matches(':'), self.cfg.cdn.host.trim_end_matches('/'), cdn_path.trim_start_matches('/'));
        conn.exec_drop(
            format!(r#"
                INSERT INTO {} (attachment_id, image_size, wp_url, wp_path, cdn_path, cdn_url, target_provider, target_url, agent_uuid, is_active)
                VALUES (:aid,:sz,:wu,:wp,:cp,:cu,:prov,:tu,:uuid,1)
                ON DUPLICATE KEY UPDATE
                  wp_url=VALUES(wp_url),
                  wp_path=VALUES(wp_path),
                  cdn_path=VALUES(cdn_path),
                  cdn_url=VALUES(cdn_url),
                  target_provider=VALUES(target_provider),
                  target_url=VALUES(target_url),
                  agent_uuid=VALUES(agent_uuid),
                  is_active=1,
                  updated_at=CURRENT_TIMESTAMP
            "#, self.cdn_map_table()),
            params!{
                "aid"=>attachment_id, "sz"=>image_size, "wu"=>wp_url, "wp"=>wp_path, "cp"=>cdn_path,
                "cu"=>cdn_url.clone(), "prov"=>provider, "tu"=>target_url, "uuid"=>agent_uuid
            }
        ).await?;
        Ok(cdn_url)
    }

    pub async fn set_cdn_target_latest_blogger(&self, attachment_id: i64, image_size: &str, blogger_url: &str) -> Result<()> {
        let mut conn = self.pool.get_conn().await?;
        conn.exec_drop(
            format!("UPDATE {} SET target_provider='blogger', target_url=:u, updated_at=CURRENT_TIMESTAMP WHERE attachment_id=:aid AND image_size=:sz", self.cdn_map_table()),
            params!{"u"=>blogger_url, "aid"=>attachment_id, "sz"=>image_size}
        ).await?;
        Ok(())
    }

    pub async fn get_cdn_target_by_path(&self, path: &str) -> Result<Option<String>> {
        let mut conn = self.pool.get_conn().await?;
        let norm = path.trim_start_matches('/');
        // try exact cdn_path
        let row: Option<(String,)> = conn.exec_first(
            format!("SELECT target_url FROM {} WHERE cdn_path=:p AND is_active=1 LIMIT 1", self.cdn_map_table()),
            params!{"p"=>norm}
        ).await?;
        if let Some((u,)) = row { return Ok(Some(u)); }
        // try matching by wp_path (legacy WP URL path hitting CDN)
        let row: Option<(String,)> = conn.exec_first(
            format!("SELECT target_url FROM {} WHERE wp_path=:p AND is_active=1 ORDER BY updated_at DESC LIMIT 1", self.cdn_map_table()),
            params!{"p"=>norm}
        ).await?;
        Ok(row.map(|r| r.0))
    }

    pub fn uploads_rel_path_from_url(url: &str) -> Option<String> {
        if let Some(idx) = url.find("/wp-content/uploads/") {
            let rel = &url[idx + 1..]; // keep wp-content/uploads/...
            return Some(rel.split('?').next().unwrap_or(rel).to_string());
        }
        None
    }

    fn pretty_cdn_path(wp_path: &str, agent_uuid: &str) -> String {
        let p = std::path::Path::new(wp_path);
        let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("image");
        let ext = p.extension().and_then(|e| e.to_str()).map(|e| format!(".{e}")).unwrap_or_default();
        let parent = p.parent().map(|d| d.to_string_lossy().to_string()).unwrap_or_default();
        let file = format!("{}-{}{}", stem, agent_uuid, ext);
        if parent.is_empty() || parent == "." { file } else { format!("{}/{}", parent.trim_end_matches('/'), file) }
    }

    fn short_uuid_for_url(url: &str, image_size: &str) -> String {
        let mut h = Sha256::new();
        h.update(url.as_bytes());
        h.update(b"|");
        h.update(image_size.as_bytes());
        let hexs = hex::encode(h.finalize());
        hexs[..12].to_string()
    }

    pub async fn enqueue_blogger_upload(&self, attachment_id: i64, post_id: Option<i64>, image_size: &str, source_url: &str, local_path: &str) -> Result<()> {
        let mut conn = self.pool.get_conn().await?;
        conn.exec_drop(
            format!("INSERT INTO {} (attachment_id, post_id, image_size, source_url, local_path, status) VALUES (:aid,:pid,:sz,:src,:lp,'queued')", self.blogger_queue_table()),
            params!{"aid"=>attachment_id, "pid"=>post_id, "sz"=>image_size, "src"=>source_url, "lp"=>local_path}
        ).await?;
        Ok(())
    }

    pub async fn get_queued_blogger_jobs(&self, limit: usize) -> Result<Vec<(i64,i64,Option<i64>,String,String,Option<String>)>> {
        let mut conn = self.pool.get_conn().await?;
        let rows: Vec<Row> = conn.exec(
            format!("SELECT id, attachment_id, post_id, image_size, source_url, local_path FROM {} WHERE status='queued' ORDER BY id ASC LIMIT :lim", self.blogger_queue_table()),
            params!{"lim"=> limit as u64}
        ).await?;
        Ok(rows.into_iter().map(|r| (
            r.get::<i64,_>("id").unwrap_or(0),
            r.get::<i64,_>("attachment_id").unwrap_or(0),
            r.get::<Option<i64>,_>("post_id").unwrap_or(None),
            r.get::<String,_>("image_size").unwrap_or_default(),
            r.get::<String,_>("source_url").unwrap_or_default(),
            r.get::<Option<String>,_>("local_path").unwrap_or(None),
        )).collect())
    }

    pub async fn mark_blogger_job_status(&self, id: i64, status: &str, blogger_url: Option<&str>, err: Option<&str>) -> Result<()> {
        let mut conn = self.pool.get_conn().await?;
        conn.exec_drop(
            format!("UPDATE {} SET status=:s, blogger_url=:b, last_error=:e, attempts=attempts+1, updated_at=CURRENT_TIMESTAMP WHERE id=:id", self.blogger_queue_table()),
            params!{"s"=>status, "b"=>blogger_url, "e"=>err, "id"=>id}
        ).await?;
        Ok(())
    }


    pub async fn list_attachments_with_offload(&self, limit: usize, offset: usize) -> Result<Vec<serde_json::Value>> {
        let mut conn = self.pool.get_conn().await?;
        let uploads_base = format!("{}/", self.cfg.site.url.trim_end_matches('/'));
        let rows: Vec<Row> = conn.exec(
            format!(
                "SELECT p.ID as attachment_id,
                        p.post_parent as parent_post_id,
                        p.post_title,
                        p.post_mime_type,
                        p.guid,
                        p.post_date,
                        MAX(CASE WHEN pm.meta_key='_wp_attached_file' THEN pm.meta_value END) as attached_file,
                        MAX(CASE WHEN pm.meta_key='_wp_attachment_metadata' THEN pm.meta_value END) as attachment_meta,
                        COALESCE(agg.total_mappings, 0) as total_mappings,
                        COALESCE(agg.offloaded_mappings, 0) as offloaded_mappings,
                        COALESCE(agg.replaced_mappings, 0) as replaced_mappings,
                        COALESCE(agg.failed_mappings, 0) as failed_mappings,
                        MAX(i.status) as latest_status
                 FROM {} p
                 LEFT JOIN {} pm ON pm.post_id = p.ID AND pm.meta_key IN ('_wp_attached_file','_wp_attachment_metadata')
                 LEFT JOIN (
                    SELECT attachment_id,
                           COUNT(*) as total_mappings,
                           SUM(CASE WHEN status IN ('uploaded','replaced') THEN 1 ELSE 0 END) as offloaded_mappings,
                           SUM(CASE WHEN status='replaced' THEN 1 ELSE 0 END) as replaced_mappings,
                           SUM(CASE WHEN status='failed' THEN 1 ELSE 0 END) as failed_mappings
                    FROM {} GROUP BY attachment_id
                 ) agg ON agg.attachment_id = p.ID
                 LEFT JOIN {} i ON i.attachment_id = p.ID
                 WHERE p.post_type='attachment'
                   AND (p.post_mime_type LIKE 'image/%' OR p.post_mime_type = '' )
                 GROUP BY p.ID
                 ORDER BY p.ID DESC
                 LIMIT :lim OFFSET :off",
                self.posts_table(), self.postmeta_table(), self.images_table(), self.images_table()
            ),
            params!{"lim"=> limit as u64, "off"=> offset as u64}
        ).await?;

        let mut out = Vec::new();
        for r in rows {
            let attachment_id: i64 = r.get("attachment_id").unwrap_or(0);
            let attached_file: Option<String> = r.get("attached_file");
            let guid: Option<String> = r.get("guid");
            let attachment_meta: Option<String> = r.get("attachment_meta");
            let mut origin_url = if let Some(f) = attached_file.as_ref() {
                if f.starts_with("http://") || f.starts_with("https://") { f.clone() } else { format!("{}/wp-content/uploads/{}", self.cfg.site.url.trim_end_matches('/'), f.trim_start_matches('/')) }
            } else {
                guid.clone().unwrap_or_default()
            };
            if origin_url.is_empty() { origin_url = format!("{}?attachment_id={}", uploads_base.trim_end_matches('/'), attachment_id); }
            let sizes_count = parse_attachment_sizes_count(attachment_meta.as_deref());
            let parent_post_id: Option<i64> = r.get("parent_post_id");
            let affected = self.find_posts_using_urls(&[origin_url.clone()]).await.unwrap_or_default();
            let mut affected_types: Vec<String> = Vec::new();
            if !affected.is_empty() {
                affected_types = self.get_post_types_by_ids(&affected).await.unwrap_or_default();
            }
            out.push(serde_json::json!({
                "id": attachment_id,
                "attachmentId": attachment_id,
                "parentPostId": parent_post_id,
                "title": r.get::<String,_>("post_title").unwrap_or_default(),
                "mimeType": r.get::<Option<String>,_>("post_mime_type").unwrap_or(None),
                "originUrl": origin_url,
                "uploadedAt": r.get::<Option<String>,_>("post_date").unwrap_or(None),
                "sizesCount": sizes_count,
                "totalMappings": r.get::<i64,_>("total_mappings").unwrap_or(0),
                "offloadedMappings": r.get::<i64,_>("offloaded_mappings").unwrap_or(0),
                "replacedMappings": r.get::<i64,_>("replaced_mappings").unwrap_or(0),
                "failedMappings": r.get::<i64,_>("failed_mappings").unwrap_or(0),
                "latestStatus": r.get::<Option<String>,_>("latest_status").unwrap_or(None),
                "affectedPostIds": affected,
                "affectedPostTypes": affected_types,
            }));
        }
        Ok(out)
    }

    pub async fn get_post_types_by_ids(&self, ids: &[i64]) -> Result<Vec<String>> {
        if ids.is_empty() { return Ok(vec![]); }
        let mut conn = self.pool.get_conn().await?;
        let mut types = std::collections::BTreeSet::new();
        for id in ids {
            let row: Option<(String,)> = conn.exec_first(
                format!("SELECT post_type FROM {} WHERE ID=:id", self.posts_table()),
                params!{"id"=>*id}
            ).await?;
            if let Some((t,)) = row { if !t.is_empty() { types.insert(t); } }
        }
        Ok(types.into_iter().collect())
    }

    pub async fn get_attachment_payload(&self, attachment_id: i64) -> Result<Option<crate::webhook::MediaUploadPayload>> {
        let mut conn = self.pool.get_conn().await?;
        let row: Option<Row> = conn.exec_first(
            format!(
                "SELECT p.ID, p.post_parent, p.guid, p.post_mime_type,
                        MAX(CASE WHEN pm.meta_key='_wp_attached_file' THEN pm.meta_value END) as attached_file,
                        MAX(CASE WHEN pm.meta_key='_wp_attachment_metadata' THEN pm.meta_value END) as meta
                 FROM {posts} p
                 LEFT JOIN {pm} pm ON pm.post_id=p.ID AND pm.meta_key IN ('_wp_attached_file','_wp_attachment_metadata')
                 WHERE p.ID=:id AND p.post_type='attachment'
                 GROUP BY p.ID",
                posts=self.posts_table(), pm=self.postmeta_table()
            ),
            params!{"id"=>attachment_id}
        ).await?;
        let Some(r) = row else { return Ok(None); };
        let attached_file: Option<String> = r.get("attached_file");
        let meta_raw: Option<String> = r.get("meta");
        let guid: Option<String> = r.get("guid");
        let mime: Option<String> = r.get("post_mime_type");
        let post_parent: Option<i64> = r.get("post_parent");

        let base_url = format!("{}/wp-content/uploads", self.cfg.site.url.trim_end_matches('/'));
        let full_url = if let Some(f)=attached_file.as_ref() {
            if f.starts_with("http://") || f.starts_with("https://") { f.clone() } else { format!("{}/{}", base_url, f.trim_start_matches('/')) }
        } else { guid.clone().unwrap_or_default() };
        if full_url.is_empty() { return Ok(None); }
        let full_path = if let Some(f)=attached_file.as_ref() {
            if f.starts_with("http://") || f.starts_with("https://") { self.url_to_local_path(&full_url) } else { Some(std::path::Path::new(&self.cfg.site.uploads_dir).join(f).to_string_lossy().to_string()) }
        } else { self.url_to_local_path(&full_url) };

        let mut sizes: Vec<crate::webhook::ImageSizePayload> = Vec::new();
        // full
        sizes.push(crate::webhook::ImageSizePayload {
            size: "full".into(),
            url: full_url.clone(),
            path: full_path.clone(),
            width: None, height: None, mime_type: mime.clone(), file_size: full_path.as_ref().and_then(|p| std::fs::metadata(p).ok().map(|m| m.len() as i64)),
        });

        if let Some(meta) = meta_raw.as_deref() {
            // parse sizes from php serialized metadata: key => array(file,width,height,mime-type,filesize)
            let rel_dir = attached_file.as_deref().and_then(|f| std::path::Path::new(f).parent().map(|p| p.to_string_lossy().to_string())).unwrap_or_default();
            for cap in regex::Regex::new(r#"s:\d+:"([^"]+)";a:\d+:\{[^}]*?s:4:"file";s:\d+:"([^"]+)""#).unwrap().captures_iter(meta) {
                let size_name = cap.get(1).map(|m| m.as_str().to_string()).unwrap_or_default();
                let file = cap.get(2).map(|m| m.as_str().to_string()).unwrap_or_default();
                if size_name.is_empty() || file.is_empty() { continue; }
                if sizes.iter().any(|s| s.size == size_name) { continue; }
                let rel = if rel_dir.is_empty() { file.clone() } else { format!("{}/{}", rel_dir.trim_end_matches('/'), file) };
                let url = format!("{}/{}", base_url, rel.trim_start_matches('/'));
                let path = std::path::Path::new(&self.cfg.site.uploads_dir).join(&rel).to_string_lossy().to_string();
                // try to parse dimensions/mime/filesize nearby (best-effort)
                sizes.push(crate::webhook::ImageSizePayload {
                    size: size_name,
                    url,
                    path: Some(path),
                    width: None, height: None, mime_type: None, file_size: None,
                });
            }
        }

        Ok(Some(crate::webhook::MediaUploadPayload {
            attachment_id,
            post_id: post_parent.filter(|p| *p > 0),
            file: full_path,
            url: Some(full_url),
            mime_type: mime,
            width: None,
            height: None,
            file_size: None,
            sizes,
        }))
    }



    pub async fn get_latest_url_for_size(&self, attachment_id: i64, image_size: &str, provider: &str) -> Result<Option<String>> {
        let mut conn = self.pool.get_conn().await?;
        let row: Option<(String,)> = conn.exec_first(
            format!("SELECT new_url FROM {} WHERE attachment_id=:aid AND image_size=:sz AND provider=:p ORDER BY id DESC LIMIT 1", self.url_versions_table()),
            params!{"aid"=>attachment_id, "sz"=>image_size, "p"=>provider}
        ).await?;
        Ok(row.map(|r| r.0))
    }

    pub async fn find_posts_using_urls(&self, urls: &[String]) -> Result<Vec<i64>> {
        if urls.is_empty() {
            return Ok(vec![]);
        }
        let mut conn = self.pool.get_conn().await?;
        let mut ids = std::collections::HashSet::new();
        // Search across all post types that can use images (posts, products, pages, CPTs, etc.)
        // using IN over URL list for exact/fast matching chunks.
        for chunk in urls.chunks(100) {
            // batched LIKE scan per URL (covers all post types/CPTs that may use image URLs)
            for u in chunk {
                let like = format!("%{}%", u);
                let rows: Vec<(i64,)> = conn
                    .exec(
                        format!(
                            "SELECT ID FROM {} WHERE (post_content LIKE :like OR post_excerpt LIKE :like) AND post_type NOT IN ('revision','nav_menu_item')",
                            self.posts_table()
                        ),
                        params! { "like" => like },
                    )
                    .await?;
                for (id,) in rows {
                    ids.insert(id);
                }
            }
        }
        // Also detect usage in postmeta for any of these URLs (thumbnails/builders may store there)
        // find_posts_using_urls is focused on wp_posts IDs; meta-only refs still replaced globally later.
        let mut v: Vec<i64> = ids.into_iter().collect();
        v.sort();
        Ok(v)
    }

    pub async fn replace_urls_in_posts_by_in(&self, replacements: &[(String,String)]) -> Result<usize> {
        if replacements.is_empty() { return Ok(0); }
        let urls: Vec<String> = replacements.iter().map(|(o,_)| o.clone()).collect();
        let post_ids = self.find_posts_using_urls(&urls).await?;
        let mut total = 0usize;
        for pid in post_ids {
            total += self.replace_urls_in_post_content(pid, replacements, self.cfg.agent.backup_before_replace).await?;
        }
        Ok(total)
    }



}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PostStatsRow {
    pub id: i64,
    pub title: String,
    pub status: String,
    #[serde(default)]
    pub post_type: String,
    pub total_images: i64,
    pub offloaded_images: i64,
    pub replaced_images: i64,
    pub blogger_post_id: Option<String>,
    pub blogger_post_url: Option<String>,
    pub blogger_blog_id: Option<String>,
    pub linked_at: Option<String>,
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
        width: r.get("width"),
        height: r.get("height"),
        mime_type: r.get("mime_type"),
        original_size_bytes: r.get("original_size_bytes"),
        offload_size_bytes: r.get("offload_size_bytes"),
        status: r.get::<String,_>("status").unwrap_or_else(|| "pending".into()),
        redirect_active: r.get::<i32,_>("redirect_active").unwrap_or(0),
        error_message: r.get("error_message"),
        metadata: r.get("metadata"),
        created_at: r.get("created_at"),
        updated_at: r.get("updated_at"),
        replaced_at: r.get("replaced_at"),
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


fn parse_attachment_sizes_count(meta: Option<&str>) -> i64 {
    let Some(m) = meta else { return 1; };
    // crude php serialized count of sizes array entries: look for s:5:"sizes";a:N:
    if let Some(idx) = m.find("s:5:\"sizes\";a:") {
        let rest = &m[idx + "s:5:\"sizes\";a:".len()..];
        let mut n = String::new();
        for ch in rest.chars() { if ch.is_ascii_digit() { n.push(ch); } else { break; } }
        if let Ok(v) = n.parse::<i64>() { return v.max(0) + 1; }
    }
    1
}
