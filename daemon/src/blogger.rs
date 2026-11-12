use crate::chrome::Cdp;
use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{debug, info, warn};

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
    ensure_editor_ready(cdp).await?;
    set_title(cdp, post_title).await?;

    let mut uploaded = Vec::new();
    let mut failed_count = 0;
    for path in local_paths {
        if !std::path::Path::new(path).exists() {
            warn!("skip missing file {}", path);
            failed_count += 1;
            continue;
        }
        match upload_one_image(cdp, path).await {
            Ok(u) => uploaded.push(u),
            Err(e) => {
                info!("upload failed {}: {}", path, e);
                failed_count += 1;
            }
        }
    }

    if failed_count > 0 && uploaded.is_empty() {
        return Err(Error::BloggerError(format!("all {} images failed to upload", failed_count)));
    }

    let (post_id, post_url) = publish_post(cdp).await?;
    wait_blogger_settled(cdp, 60).await?;

    Ok((BloggerPostLink { post_id, post_url }, uploaded))
}

async fn open_composer(cdp: &Cdp, blog_id: &str, existing_post_id: Option<&str>) -> Result<()> {
    // If we have an existing post ID, try to open that specific editor URL first
    if let Some(pid) = existing_post_id {
        if !pid.is_empty() && !blog_id.is_empty() {
            for host in ["draft.blogger.com", "www.blogger.com"] {
                let url = format!("https://{host}/blog/post/edit/{blog_id}/{pid}");
                info!("trying existing post editor: {}", url);
                cdp.goto(&url, 3500).await?;
                ensure_signed_in(cdp, &url).await?;
                if wait_for_editor(cdp, Some(pid), 25).await {
                    return Ok(());
                }
                warn!("existing post editor did not load at {}", url);
            }
        }
    }

    // Fresh post: open the posts list for the blog, then click "New post"
    for host in ["draft.blogger.com", "www.blogger.com"] {
        let list_url = if blog_id.is_empty() {
            format!("https://{host}/blog/posts")
        } else {
            format!("https://{host}/blog/posts/{blog_id}")
        };
        info!("trying posts list: {}", list_url);
        cdp.goto(&list_url, 3000).await?;
        ensure_signed_in(cdp, &list_url).await?;

        // Try clicking "New post" button (Closure UI needs trusted clicks)
        let clicked = try_click_new_post(cdp).await?;
        if clicked {
            if wait_for_editor(cdp, None, 20).await {
                return Ok(());
            }
            warn!("'New post' clicked but editor did not open");
        }

        // Fallback: direct new-post URL
        if !blog_id.is_empty() {
            let new_url = format!("https://{host}/blog/post/new/{blog_id}");
            info!("fallback to direct new-post URL: {}", new_url);
            cdp.goto(&new_url, 3500).await?;
            ensure_signed_in(cdp, &new_url).await?;
            if wait_for_editor(cdp, None, 20).await {
                return Ok(());
            }
        }
    }

    if quota_notice(cdp).await {
        return Err(Error::BloggerError("Blogger quota exceeded: cannot create new post".into()));
    }
    Err(Error::BloggerError("failed to open Blogger composer (check Chrome profile login)".into()))
}

async fn try_click_new_post(cdp: &Cdp) -> Result<bool> {
    // Blogger's Closure buttons ignore synthetic clicks; use trusted (real) clicks
    for sel in [
        "[aria-label='Tạo bài đăng mới']",
        "[aria-label='New post']",
        "[data-g-label='New post']",
    ] {
        if cdp.real_click_selector(sel).await.unwrap_or(false) {
            info!("clicked New post via selector {}", sel);
            return Ok(true);
        }
    }
    for text in ["Tạo bài đăng mới", "New post", "Bài đăng mới"] {
        if cdp.real_click_text(text).await.unwrap_or(false) {
            info!("clicked New post via text '{}'", text);
            return Ok(true);
        }
    }
    // Try header blog dropdown first (sometimes needed to select correct blog)
    for label in ["SinfuSlay Store", "SinfuSlay"] {
        let _ = cdp.real_click_text(label).await;
        cdp.wait_ms(3000).await;
    }
    for sel in [
        "[aria-label='Tạo bài đăng mới']",
        "[aria-label='New post']",
        "[data-g-label='New post']",
    ] {
        if cdp.real_click_selector(sel).await.unwrap_or(false) {
            return Ok(true);
        }
    }
    for text in ["Tạo bài đăng mới", "New post", "Bài đăng mới"] {
        if cdp.real_click_text(text).await.unwrap_or(false) {
            return Ok(true);
        }
    }
    Ok(false)
}

async fn wait_for_editor(cdp: &Cdp, expected_post_id: Option<&str>, secs: u32) -> bool {
    for i in 0..secs {
        let url = cdp.current_url().await.unwrap_or_default();
        if is_editor_url(&url, expected_post_id) {
            debug!("editor ready at {} (attempt {})", url, i + 1);
            return true;
        }
        if is_composer_ready(cdp).await {
            debug!("composer ready (no editor URL yet) at {}", url);
            return true;
        }
        cdp.wait_ms(1000).await;
    }
    false
}

fn is_editor_url(url: &str, expected_post_id: Option<&str>) -> bool {
    let route = url.split('#').next().unwrap_or("").split('?').next().unwrap_or("").trim_end_matches('/');
    if let Some(caps) = regex::Regex::new(r"/post/edit/[^/]+/([^/]+)$").unwrap().captures(route) {
        let post_id = caps.get(1).map(|m| m.as_str()).unwrap_or("");
        if expected_post_id.map(|e| e == post_id).unwrap_or(true) && !post_id.is_empty() {
            return true;
        }
    }
    false
}

async fn is_composer_ready(cdp: &Cdp) -> bool {
    let sel = "input[name='title'], .title input, input[data-g-title], div.editable, div[contenteditable='true']";
    cdp.wait_for_visible(sel, 2000).await.unwrap_or(false)
}

async fn ensure_editor_ready(cdp: &Cdp) -> Result<()> {
    let timeout = Duration::from_secs(25);
    let deadline = std::time::Instant::now() + timeout;
    while std::time::Instant::now() < deadline {
        if is_composer_ready(cdp).await {
            return Ok(());
        }
        cdp.wait_ms(500).await;
    }
    Err(Error::BloggerError("Blogger editor/composer did not become ready in time".into()))
}

async fn quota_notice(cdp: &Cdp) -> bool {
    let js = r#"(function(){const nodes=document.querySelectorAll("div[id^='goog-lr-'],[aria-live],[role='alert'],.toast"); let t=''; nodes.forEach(n=>t+=' '+(n.innerText||n.textContent||'')); return t.toLowerCase();})()"#;
    let v = cdp.evaluate(js, false).await.unwrap_or_default();
    let s = v.as_str().unwrap_or("").to_lowercase();
    ["không thể tạo bài đăng mới", "unable to create new post", "could not create new post", "can't create a new post"]
        .iter()
        .any(|p| s.contains(p))
}

async fn needs_google_sign_in(cdp: &Cdp) -> bool {
    let url = cdp.current_url().await.unwrap_or_default();
    if regex::Regex::new(r"^https?://accounts\.google\.com(?:/|$)").unwrap().is_match(&url) {
        return true;
    }
    for sel in [
        "a[href*='SignInOptions']",
        "a[data-g-label='Sign in']",
        "div[data-g-label='Sign in']",
        "span[data-g-label='Sign in']",
        "button[data-g-label='Sign in']",
        "a[href*='signin']",
        "a[href*='ServiceLogin']",
    ] {
        if cdp.wait_for_visible(sel, 750).await.unwrap_or(false) {
            return true;
        }
    }
    let js = r#"Array.from(document.querySelectorAll('a,button,[role="button"]')).some((el) => ['sign in', 'đăng nhập'].includes((el.innerText || el.getAttribute('aria-label') || '').trim().toLowerCase()))"#;
    cdp.evaluate(js, false).await.unwrap_or_default().as_bool().unwrap_or(false)
}

async fn ensure_signed_in(cdp: &Cdp, retry_url: &str) -> Result<()> {
    if !needs_google_sign_in(cdp).await {
        return Ok(());
    }
    warn!("Google sign-in required. Please sign in manually in the Chrome window, then close it to resume.");
    // We can't launch a separate Chrome from here easily; rely on user profile being pre-signed-in.
    // Just wait and re-check once.
    cdp.wait_ms(5000).await;
    if needs_google_sign_in(cdp).await {
        return Err(Error::ChromeError("Google sign-in still required after waiting".into()));
    }
    info!("Google sign-in confirmed; resuming");
    cdp.goto(retry_url, 25000).await?;
    cdp.wait_ms(2500).await;
    Ok(())
}

async fn set_title(cdp: &Cdp, title: &str) -> Result<()> {
    let t = serde_json::to_string(title)?;
    let js = format!(r#"(function(){{const t={t}; const qs=["input[name=title]","input[data-g-title]",".title input","input[aria-label='Tiêu đề']","input[aria-label='Title']","input[placeholder*='Title']"]; for(const s of qs){{const el=document.querySelector(s); if(el){{el.focus(); el.value=t; el.dispatchEvent(new Event('input',{{bubbles:true}})); el.dispatchEvent(new Event('change',{{bubbles:true}})); return true;}}}} return false;}})()"#);
    let ok = cdp.evaluate(&js, false).await.unwrap_or_default().as_bool().unwrap_or(false);
    if ok {
        return Ok(());
    }
    // Also try iframes (older Blogger editor)
    let js_iframe = format!(r#"(function(){{const t={t}; const qs=["input[name=title]","input[data-g-title]",".title input","input[aria-label='Tiêu đề']","input[aria-label='Title']"]; for(const f of document.querySelectorAll('iframe')){{try{{const d=f.contentDocument; if(d){{for(const s of qs){{const el=d.querySelector(s); if(el){{el.focus(); el.value=t; el.dispatchEvent(new Event('input',{{bubbles:true}})); el.dispatchEvent(new Event('change',{{bubbles:true}})); return true;}}}}}}}}catch(e){{}} return false;}})()"#);
    let ok2 = cdp.evaluate(&js_iframe, false).await.unwrap_or_default().as_bool().unwrap_or(false);
    if ok2 {
        return Ok(());
    }
    Err(Error::BloggerError("could not set post title".into()))
}

async fn upload_one_image(cdp: &Cdp, path: &str) -> Result<UploadedImage> {
    let before: std::collections::HashSet<String> = cdp.body_imgs().await.unwrap_or_default().into_iter().collect();

    open_image_picker(cdp).await?;

    // Click "Upload from computer" / "Tải lên từ máy tính" in the image submenu
    let mut clicked = false;
    for _ in 0..3 {
        for sel in [
            "[role='menuitem'][aria-label='Tải lên từ máy tính']",
            "[data-command='+insertPhoto'][data-value='upload']",
        ] {
            if cdp.real_click_selector(sel).await.unwrap_or(false) {
                clicked = true;
                break;
            }
        }
        if !clicked {
            clicked = cdp.real_click_text("Tải lên từ máy tính").await.unwrap_or(false)
                || cdp.real_click_text("Upload from computer").await.unwrap_or(false)
                || cdp.real_click_text("Upload").await.unwrap_or(false);
        }
        if clicked {
            break;
        }
        open_image_picker(cdp).await.ok();
        cdp.wait_ms(600).await;
    }

    cdp.wait_ms(1500).await;
    let mut uploaded = false;
    if clicked {
        // Try Google Picker OOPIF (docs.google.com/picker)
        uploaded = cdp.picker_set_file(&[path.to_string()]).await.unwrap_or(false);
    }
    if !uploaded {
        // Fallback: native file input if picker didn't open
        uploaded = cdp.set_file_input_files("input[type=file]", &[path.to_string()]).await.unwrap_or(false);
    }
    if !uploaded {
        return Err(Error::BloggerError(format!("no upload path for image {}", path)));
    }

    // Wait for inserted blogger image (blogger.googleusercontent.com)
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
        if found.is_some() {
            break;
        }
    }
    let blogger_url = found.ok_or_else(|| Error::BloggerError(format!("image uploaded but no blogger URL detected: {}", path)))?;
    let blogger_image_id = extract_asset_id(&blogger_url);
    Ok(UploadedImage { origin_path: path.to_string(), blogger_url, blogger_image_id })
}

async fn open_image_picker(cdp: &Cdp) -> Result<()> {
    // Focus the editor body so inline toolbar appears
    for sel in [
        "div.editable[contenteditable='true']",
        "div[contenteditable='true']",
        ".editable",
        "iframe.editable",
        "iframe[class*='editable']",
    ] {
        let _ = cdp.real_click_selector(sel).await;
        cdp.wait_ms(300).await;
    }
    // Open the image submenu (toolbar button aria 'Chèn hình ảnh' / 'Insert image')
    for _ in 0..3 {
        if cdp.real_click_text("Chèn hình ảnh").await.unwrap_or(false)
            || cdp.real_click_text("Insert image").await.unwrap_or(false)
        {
            cdp.wait_ms(1200).await;
            return Ok(());
        }
        cdp.wait_ms(400).await;
    }
    // Try common toolbar selectors
    for sel in [
        "[data-command='+editImage,+toggleCaption'] div[role='button']",
        "[data-command='+editImage,+toggleCaption']",
        "[data-g-label='Add image']",
        "[data-g-label='Image']",
        "[aria-label='Insert image']",
    ] {
        if cdp.real_click_selector(sel).await.unwrap_or(false) {
            cdp.wait_ms(1200).await;
            return Ok(());
        }
    }
    Ok(())
}

async fn publish_post(cdp: &Cdp) -> Result<(String, String)> {
    // Click the primary Publish button
    for label in ["Đăng", "Publish", "Xuất bản"] {
        if cdp.real_click_text(label).await.unwrap_or(false) {
            info!("primary publish clicked via '{}'", label);
            break;
        }
        let _ = cdp.click_selector("span.publish-button").await;
    }
    cdp.wait_ms(2500).await;

    // Handle publish confirmation modal (Vietnamese + English)
    for label in ["Đăng", "Publish", "Xuất bản", "Xác nhận"] {
        let _ = cdp.real_click_text(label).await;
    }
    // Try common confirmation button selectors
    for sel in [
        "button[data-g-action='publishMobile']",
        "button[data-g-action='publish']",
        "span.publish-button",
        "span[data-g-label='Publish']",
    ] {
        let _ = cdp.click_selector(sel).await;
    }

    cdp.wait_ms(4500).await;
    let url = cdp.current_url().await.unwrap_or_default();
    let post_id = extract_post_id(&url).unwrap_or_default();
    if post_id.is_empty() {
        return Err(Error::BloggerError("could not resolve published Blogger post id".into()));
    }
    Ok((post_id, url))
}

async fn wait_blogger_settled(cdp: &Cdp, timeout_secs: u64) -> Result<()> {
    let deadline = std::time::Instant::now() + Duration::from_secs(timeout_secs);
    let saving_re = regex::Regex::new(r"(đang lưu|đang xuất bản|saving|publishing)").unwrap();
    let saved_re = regex::Regex::new(r"(đã lưu|đã xuất bản|lưu tất cả|all changes saved|saved|published)").unwrap();

    while std::time::Instant::now() < deadline {
        let text = get_editor_status_text(cdp).await;
        if !text.is_empty() && !saving_re.is_match(&text) && saved_re.is_match(&text) {
            cdp.wait_ms(1500).await;
            return Ok(());
        }
        cdp.wait_ms(1500).await;
    }
    warn!("blogger editor did not report a saved state in time");
    cdp.wait_ms(2000).await;
    Ok(())
}

async fn get_editor_status_text(cdp: &Cdp) -> String {
    let js = r#"(function(){var t=document.body?document.body.innerText:''; return t.length>20000?t.slice(0,20000):t;})()"#;
    cdp.evaluate(js, false).await.unwrap_or_default().as_str().unwrap_or("").to_string()
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