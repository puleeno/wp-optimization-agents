use crate::chrome::Cdp;
use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BloggerPostLink {
    pub post_id: String,
    pub post_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadedImage {
    pub origin_path: String,
    pub blogger_url: String,
    pub blogger_image_id: Option<String>,
}

/// Upload images for one WP post as one Blogger post (album-like post).
/// `existing_post_id` if WP post already linked to a Blogger post (reuse editor).
pub async fn upload_images_for_post(
    cdp: &Cdp,
    blog_id: &str,
    post_title: &str,
    existing_post_id: Option<&str>,
    local_paths: &[String],
) -> Result<(BloggerPostLink, Vec<UploadedImage>)> {
    if local_paths.is_empty() {
        return Err(Error::BloggerError("no images to upload".into()));
    }
    open_composer(cdp, blog_id, existing_post_id).await?;
    set_title(cdp, post_title).await?;

    let mut uploaded = Vec::new();
    for path in local_paths {
        if !std::path::Path::new(path).exists() {
            tracing::warn!("skip missing file {}", path);
            continue;
        }
        match upload_one_image(cdp, path).await {
            Ok(u) => uploaded.push(u),
            Err(e) => {
                tracing::error!("upload failed {}: {}", path, e);
                return Err(e);
            }
        }
    }

    let (post_id, post_url) = publish_post(cdp, existing_post_id).await?;
    Ok((BloggerPostLink { post_id, post_url }, uploaded))
}

async fn open_composer(cdp: &Cdp, blog_id: &str, existing_post_id: Option<&str>) -> Result<()> {
    if let Some(pid) = existing_post_id {
        if !pid.is_empty() && !blog_id.is_empty() {
            for host in ["draft.blogger.com", "www.blogger.com"] {
                let url = format!("https://{host}/blog/post/edit/{blog_id}/{pid}");
                cdp.goto(&url, 3500).await?;
                if wait_editor_ready(cdp, 25).await { return Ok(()); }
            }
        }
    }
    // fresh post
    for host in ["draft.blogger.com", "www.blogger.com"] {
        let list_url = if blog_id.is_empty() { format!("https://{host}/blog/posts") } else { format!("https://{host}/blog/posts/{blog_id}") };
        cdp.goto(&list_url, 3000).await?;
        let _ = try_click_new_post(cdp).await;
        if wait_editor_ready(cdp, 20).await { return Ok(()); }
        if !blog_id.is_empty() {
            let new_url = format!("https://{host}/blog/post/new/{blog_id}");
            cdp.goto(&new_url, 3500).await?;
            if wait_editor_ready(cdp, 20).await { return Ok(()); }
        }
    }
    if quota_notice(cdp).await { return Err(Error::BloggerError("Blogger quota: cannot create new post".into())); }
    Err(Error::BloggerError("failed to open Blogger composer (check login/profile)".into()))
}

async fn try_click_new_post(cdp: &Cdp) -> Result<bool> {
    for sel in ["[aria-label='Tạo bài đăng mới']", "[aria-label='New post']", "[data-g-label='New post']"] {
        if cdp.real_click_selector(sel).await.unwrap_or(false) { return Ok(true); }
    }
    for t in ["Tạo bài đăng mới", "New post", "Bài đăng mới"] {
        if cdp.real_click_text(t).await.unwrap_or(false) { return Ok(true); }
    }
    Ok(false)
}

async fn wait_editor_ready(cdp: &Cdp, secs: u32) -> bool {
    for _ in 0..secs {
        let url = cdp.current_url().await.unwrap_or_default();
        if url.contains("/post/edit/") || url.contains("/blog/post/new/") { return true; }
        cdp.wait_ms(1000).await;
    }
    false
}

async fn quota_notice(cdp: &Cdp) -> bool {
    let js = r#"(function(){const nodes=document.querySelectorAll("div[id^='goog-lr-'],[aria-live],[role='alert'],.toast"); let t=''; nodes.forEach(n=>t+=' '+(n.innerText||n.textContent||'')); return t.toLowerCase();})()"#;
    let v = cdp.evaluate(js, false).await.unwrap_or_default();
    let s = v.as_str().unwrap_or("").to_lowercase();
    ["không thể tạo bài đăng mới", "unable to create new post", "could not create new post"].iter().any(|p| s.contains(p))
}

async fn set_title(cdp: &Cdp, title: &str) -> Result<()> {
    let t = serde_json::to_string(title)?;
    let js = format!(r#"(function(){{const t={t}; const qs=["input[name=title]","input[data-g-title]",".title input","input[aria-label='Tiêu đề']","input[aria-label='Title']"]; for(const s of qs){{const el=document.querySelector(s); if(el){{el.focus(); el.value=t; el.dispatchEvent(new Event('input',{{bubbles:true}})); el.dispatchEvent(new Event('change',{{bubbles:true}})); return true;}}}} return false;}})()"#);
    let ok = cdp.evaluate(&js, false).await.unwrap_or_default().as_bool().unwrap_or(false);
    if ok { return Ok(()); }
    Err(Error::BloggerError("could not set post title".into()))
}

async fn upload_one_image(cdp: &Cdp, path: &str) -> Result<UploadedImage> {
    let before: std::collections::HashSet<String> = cdp.body_imgs().await.unwrap_or_default().into_iter().collect();
    open_image_picker(cdp).await?;

    // click "Upload from computer" / "Tải lên từ máy tính"
    let mut clicked = false;
    for _ in 0..3 {
        for sel in ["[role='menuitem'][aria-label='Tải lên từ máy tính']", "[data-command='+insertPhoto'][data-value='upload']"] {
            if cdp.real_click_selector(sel).await.unwrap_or(false) { clicked = true; break; }
        }
        if !clicked { clicked = cdp.real_click_text("Tải lên từ máy tính").await.unwrap_or(false) || cdp.real_click_text("Upload from computer").await.unwrap_or(false); }
        if clicked { break; }
        open_image_picker(cdp).await.ok();
        cdp.wait_ms(600).await;
    }

    cdp.wait_ms(1500).await;
    let mut uploaded = false;
    if clicked {
        // try Google Picker OOPIF
        uploaded = cdp.picker_set_file(&[path.to_string()]).await.unwrap_or(false);
    }
    if !uploaded {
        // fallback file input
        uploaded = cdp.set_file_input_files("input[type=file]", &[path.to_string()]).await.unwrap_or(false);
    }
    if !uploaded {
        return Err(Error::BloggerError(format!("no upload path for image {}", path)));
    }

    // wait for inserted blogger image
    let mut found: Option<String> = None;
    for _ in 0..35 {
        cdp.wait_ms(1200).await;
        let now = cdp.body_imgs().await.unwrap_or_default();
        for src in now {
            if src.contains("blogger.googleusercontent.com") && !before.contains(&src) {
                found = Some(src);
                break;
            }
        }
        if found.is_some() { break; }
    }
    let blogger_url = found.ok_or_else(|| Error::BloggerError(format!("image uploaded but no blogger URL detected: {}", path)))?;
    let blogger_image_id = extract_asset_id(&blogger_url);
    Ok(UploadedImage { origin_path: path.to_string(), blogger_url, blogger_image_id })
}

async fn open_image_picker(cdp: &Cdp) -> Result<()> {
    // focus editor body
    for sel in ["div.editable[contenteditable='true']", "div[contenteditable='true']", ".editable", "iframe.editable"] {
        let _ = cdp.real_click_selector(sel).await;
        cdp.wait_ms(300).await;
    }
    for _ in 0..3 {
        if cdp.real_click_text("Chèn hình ảnh").await.unwrap_or(false) || cdp.real_click_text("Insert image").await.unwrap_or(false) {
            cdp.wait_ms(1200).await; return Ok(());
        }
        cdp.wait_ms(400).await;
    }
    // try common toolbar selectors
    for sel in ["[data-command='+editImage,+toggleCaption'] div[role='button']", "[data-command='+editImage,+toggleCaption']", "[aria-label='Insert image']"] {
        if cdp.real_click_selector(sel).await.unwrap_or(false) { cdp.wait_ms(1200).await; return Ok(()); }
    }
    Ok(())
}

async fn publish_post(cdp: &Cdp, existing_post_id: Option<&str>) -> Result<(String, String)> {
    // click publish
    for label in ["Đăng", "Publish", "Xuất bản"] {
        if cdp.real_click_text(label).await.unwrap_or(false) { break; }
        let _ = cdp.click_selector("span.publish-button").await;
    }
    cdp.wait_ms(2500).await;
    for label in ["Đăng", "Publish", "Xuất bản", "Xác nhận"] {
        let _ = cdp.real_click_text(label).await;
    }
    cdp.wait_ms(4500).await;
    let url = cdp.current_url().await.unwrap_or_default();
    let post_id = extract_post_id(&url).or_else(|| existing_post_id.map(|s| s.to_string())).unwrap_or_default();
    if post_id.is_empty() {
        return Err(Error::BloggerError("could not resolve published Blogger post id".into()));
    }
    // best-effort post url
    let post_url = if url.contains("/post/edit/") { url.clone() } else { url };
    Ok((post_id, post_url))
}

fn extract_post_id(url: &str) -> Option<String> {
    let re = regex::Regex::new(r"/post/edit/\d+/(\d+)").ok()?;
    re.captures(url).and_then(|c| c.get(1)).map(|m| m.as_str().to_string())
}

fn extract_asset_id(src: &str) -> Option<String> {
    let re = regex::Regex::new(r"AVvXs[\w-]+").ok()?;
    re.captures(src).and_then(|c| c.get(0)).map(|m| m.as_str().to_string())
        .or_else(|| src.split('?').next().and_then(|b| b.rsplit('/').next()).map(|s| s.to_string()))
}
