use reqwest::multipart;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_store::StoreExt;
use base64::{Engine as _, engine::general_purpose::{STANDARD as BASE64, URL_SAFE_NO_PAD}};
use sha2::{Sha256, Digest};

use crate::state::AppState;

// OAuth client_id loaded from environment at build time, with XOR obfuscation at rest.
// To set: export OAUTH_CLIENT_ID=... before `cargo build`.
// Falls back to built-in (obfuscated) default for development.
// Note: client_secret is no longer needed — this is a public PKCE client.
fn oauth_client_id() -> String {
    if let Ok(val) = std::env::var("OAUTH_CLIENT_ID") {
        return val;
    }
    deobfuscate(&[
        0xa7, 0xf4, 0xfb, 0xf1, 0xf0, 0xf5, 0xf5, 0xf2, 0xf7, 0xf5, 0xf4, 0xa1, 0xa7, 0xa5, 0xa7, 0xf5,
        0xf3, 0xfb, 0xf1, 0xf4, 0xf0, 0xf2, 0xf6, 0xa7, 0xfb, 0xf1, 0xf5, 0xf0, 0xf7, 0xf7, 0xf4, 0xf7,
        0xa5, 0xa6, 0xfa, 0xa0, 0xf5, 0xa2, 0xa1, 0xf1, 0xa1, 0xa0, 0xf4, 0xf1, 0xfa, 0xf7, 0xf7, 0xf1,
        0xf3, 0xf5, 0xa1, 0xf4, 0xfa, 0xf1, 0xf3, 0xf3, 0xf4, 0xf1, 0xa6, 0xf1, 0xa0, 0xf5, 0xa1, 0xa7,
    ])
}

/// XOR-deobfuscate a byte slice back to the original string.
/// Not encryption — just prevents plaintext secrets in binary/source grep.
const OBFUSCATION_KEY: u8 = 0xC3;
fn deobfuscate(data: &[u8]) -> String {
    data.iter().map(|b| (b ^ OBFUSCATION_KEY) as char).collect()
}

const REDIRECT_URI: &str = "download-screenshoter://auth/callback";
const STORE_FILE: &str = "auth.json";

/// Generate a PKCE code_verifier (43 base64url chars from 32 random bytes).
fn generate_code_verifier() -> String {
    let bytes: Vec<u8> = [
        uuid::Uuid::new_v4().as_bytes().as_slice(),
        uuid::Uuid::new_v4().as_bytes().as_slice(),
    ].concat();
    URL_SAFE_NO_PAD.encode(&bytes)
}

/// Compute code_challenge = base64url(sha256(code_verifier)) for PKCE S256.
fn compute_code_challenge(verifier: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(verifier.as_bytes());
    URL_SAFE_NO_PAD.encode(hasher.finalize())
}

#[derive(serde::Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    #[allow(dead_code)]
    token_type: Option<String>,
    #[allow(dead_code)]
    expires_in: Option<u64>,
}

/// Open internal webview for OAuth with PKCE — intercepts redirect to custom URI scheme.
/// Flow: webview → login → Doorkeeper redirects to download-screenshoter://auth/callback?code=...
/// → on_navigation catches it → exchange code with code_verifier → token saved.
#[tauri::command]
pub fn open_oauth_browser(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let code_verifier = generate_code_verifier();
    let code_challenge = compute_code_challenge(&code_verifier);

    let encoded_redirect = REDIRECT_URI
        .replace(':', "%3A")
        .replace('/', "%2F");
    let base = crate::config::api_base_url();
    let url = format!(
        "{}/oauth/authorize?client_id={}&redirect_uri={}&response_type=code&scope=&code_challenge={}&code_challenge_method=S256",
        base, oauth_client_id(), encoded_redirect, code_challenge
    );

    let app_clone = app.clone();
    let token_arc = state.access_token.clone();
    let code_captured = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let code_captured_close = std::sync::Arc::clone(&code_captured);

    let win = tauri::WebviewWindowBuilder::new(
        &app,
        "oauth",
        tauri::WebviewUrl::External(url.parse().map_err(|e| format!("URL error: {}", e))?),
    )
    .title(crate::i18n::current(&app).oauth_title)
    .inner_size(900.0, 650.0)
    .visible(true)
    .on_navigation(move |nav_url| {
        let url_str = nav_url.as_str();
        #[cfg(debug_assertions)]
        println!("OAuth nav: {}", url_str);

        // Catch the redirect to our custom URI scheme (download-screenshoter://auth/callback?code=...)
        if url_str.starts_with(REDIRECT_URI) || url_str.starts_with("download-screenshoter://") {
            if code_captured.swap(true, std::sync::atomic::Ordering::SeqCst) {
                return false;
            }

            let code = url_str
                .split('?').nth(1)
                .and_then(|q| q.split('&').find_map(|p| {
                    let mut kv = p.splitn(2, '=');
                    if kv.next()? == "code" {
                        kv.next().map(|v| urlencoding_decode(v))
                    } else { None }
                }));

            if let Some(code) = code {
                let app2 = app_clone.clone();
                let token_arc2 = token_arc.clone();
                let verifier = code_verifier.clone();
                tauri::async_runtime::spawn(async move {
                    match exchange_code_internal(code, verifier, &app2, token_arc2).await {
                        Ok(_) => { let _ = app2.emit("oauth-complete", true); }
                        Err(e) => {
                            eprintln!("OAuth error: {}", e);
                            let _ = app2.emit("oauth-complete", false);
                        }
                    }
                    if let Some(w) = app2.get_webview_window("oauth") {
                        let _ = w.close().ok();
                    }
                });
            }
            return false;
        }

        true
    })
    .build()
    .map_err(|e| format!("Failed to open auth window: {}", e))?;

    // Only emit false if user closed manually WITHOUT completing auth
    let app_for_close = app.clone();
    win.on_window_event(move |event| {
        if let tauri::WindowEvent::CloseRequested { .. } = event {
            if !code_captured_close.load(std::sync::atomic::Ordering::SeqCst) {
                let _ = app_for_close.emit("oauth-complete", false);
            }
        }
    });

    Ok(())
}

fn urlencoding_decode(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.bytes();
    while let Some(b) = chars.next() {
        if b == b'%' {
            let h1 = chars.next().unwrap_or(b'0');
            let h2 = chars.next().unwrap_or(b'0');
            let hex = format!("{}{}", h1 as char, h2 as char);
            if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                result.push(byte as char);
            }
        } else if b == b'+' {
            result.push(' ');
        } else {
            result.push(b as char);
        }
    }
    result
}

async fn exchange_code_internal(
    code: String,
    code_verifier: String,
    app: &AppHandle,
    token_arc: std::sync::Arc<std::sync::Mutex<Option<String>>>,
) -> Result<(), String> {
    let client = reqwest::Client::new();
    let response = client
        .post(format!("{}/oauth/token", crate::config::api_base_url()))
        .form(&[
            ("client_id", oauth_client_id()),
            ("code", code),
            ("code_verifier", code_verifier),
            ("redirect_uri", REDIRECT_URI.to_string()),
            ("grant_type", "authorization_code".to_string()),
        ])
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("Token exchange failed {}: {}", status, body));
    }

    let token_resp: TokenResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse token response: {}", e))?;

    let store = app.store(STORE_FILE)
        .map_err(|e| format!("Failed to open store: {}", e))?;
    store.set("access_token", serde_json::json!(token_resp.access_token.clone()));
    if let Some(ref rt) = token_resp.refresh_token {
        store.set("refresh_token", serde_json::json!(rt));
    }
    store.save().map_err(|e| format!("Failed to save store: {}", e))?;

    let mut access_token = token_arc.lock().unwrap();
    *access_token = Some(token_resp.access_token);

    #[cfg(debug_assertions)]
    println!("OAuth: token saved successfully");
    Ok(())
}

/// Refresh access token using refresh token
#[tauri::command]
pub async fn refresh_oauth_token(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let store = app.store(STORE_FILE)
        .map_err(|e| format!("Failed to open store: {}", e))?;

    let refresh_token = store.get("refresh_token")
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .ok_or("No refresh token stored")?;

    let client = reqwest::Client::new();
    let response = client
        .post(format!("{}/oauth/token", crate::config::api_base_url()))
        .form(&[
            ("client_id", oauth_client_id()),
            ("refresh_token", refresh_token),
            ("grant_type", "refresh_token".to_string()),
        ])
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("Token refresh failed {}: {}", status, body));
    }

    let token_resp: TokenResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse token response: {}", e))?;

    store.set("access_token", serde_json::json!(token_resp.access_token.clone()));
    if let Some(ref rt) = token_resp.refresh_token {
        store.set("refresh_token", serde_json::json!(rt));
    }
    store.save().map_err(|e| format!("Failed to save store: {}", e))?;

    let mut access_token = state.access_token.lock().unwrap();
    *access_token = Some(token_resp.access_token);

    Ok(())
}

/// Load saved token from store into state on startup
#[tauri::command]
pub fn load_saved_token(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    let store = app.store(STORE_FILE)
        .map_err(|e| format!("Failed to open store: {}", e))?;

    if let Some(token) = store.get("access_token").and_then(|v| v.as_str().map(|s| s.to_string())) {
        let mut access_token = state.access_token.lock().unwrap();
        *access_token = Some(token);
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Logout - open sign_out page in webview, then clear local tokens
#[tauri::command]
pub fn logout(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    // Open sign_out in a temporary webview window
    let app_clone = app.clone();
    let win = tauri::WebviewWindowBuilder::new(
        &app,
        "signout",
        tauri::WebviewUrl::External(
            format!("{}/users/sign_out", crate::config::api_base_url())
                .parse().map_err(|e| format!("URL error: {}", e))?
        ),
    )
    .title(crate::i18n::current(&app).signout_title)
    .inner_size(400.0, 300.0)
    .build()
    .map_err(|e| format!("Failed to open signout window: {}", e))?;

    // Close the window after a short delay (enough for sign_out to complete)
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        if let Some(w) = app_clone.get_webview_window("signout") {
            let _ = w.close().ok();
        }
    });

    // Clear local tokens
    let store = app.store(STORE_FILE)
        .map_err(|e| format!("Failed to open store: {}", e))?;
    store.delete("access_token");
    store.delete("refresh_token");
    store.save().map_err(|e| format!("Failed to save store: {}", e))?;

    let mut access_token = state.access_token.lock().unwrap();
    *access_token = None;
    Ok(())
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct UploadResponse {
    pub id: String,
    pub name: String,
    pub secure_url: String,
    pub shared: bool,
}

#[derive(serde::Deserialize)]
struct ApiResponse {
    object: FileObject,
}

#[derive(serde::Deserialize)]
struct FileObject {
    id: String,
    name: String,
    secure_url: String,
    shared: bool,
}

#[derive(serde::Deserialize)]
struct FolderListResponse {
    contents: Vec<ContentItem>,
}

#[derive(serde::Deserialize)]
struct ContentItem {
    id: String,
    name: String,
    is_dir: bool,
}

#[derive(serde::Deserialize)]
struct CreateFolderResponse {
    object: FolderObject,
}

#[derive(serde::Deserialize)]
struct FolderObject {
    id: String,
}

/// Get or create Screenshots folder, return its id (public alias for use from recording.rs)
pub async fn get_or_create_screenshots_folder_pub(client: &reqwest::Client, token: &str) -> Result<String, String> {
    get_or_create_screenshots_folder(client, token).await
}

/// Get or create Screenshots folder, return its id
async fn get_or_create_screenshots_folder(client: &reqwest::Client, token: &str) -> Result<String, String> {
    // List root folder contents
    let resp = client
        .get(format!("{}/folders.json", crate::config::api_base_url()))
        .header("Authorization", format!("Bearer {}", token))
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| format!("Failed to list folders: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("Failed to list folders: {}", resp.status()));
    }

    let folder_list: FolderListResponse = resp.json().await
        .map_err(|e| format!("Failed to parse folders: {}", e))?;

    // Look for Screenshots folder in contents
    if let Some(f) = folder_list.contents.iter().find(|f| f.is_dir && f.name == "Screenshots") {
        #[cfg(debug_assertions)]
        println!("Found Screenshots folder: {}", f.id);
        return Ok(f.id.clone());
    }

    // Create Screenshots folder
    #[cfg(debug_assertions)]
    println!("Creating Screenshots folder...");
    let resp = client
        .post(format!("{}/folders.json", crate::config::api_base_url()))
        .header("Authorization", format!("Bearer {}", token))
        .header("Accept", "application/json")
        .form(&[("folder[name]", "Screenshots")])
        .send()
        .await
        .map_err(|e| format!("Failed to create folder: {}", e))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("Failed to create folder {}: {}", status, body));
    }

    let created: CreateFolderResponse = resp.json().await
        .map_err(|e| format!("Failed to parse create folder response: {}", e))?;

    #[cfg(debug_assertions)]
    println!("Created Screenshots folder: {}", created.object.id);
    Ok(created.object.id)
}

/// Upload screenshot to download.ru
#[tauri::command]
pub async fn upload_to_download(
    filename: String,
    image_data: Option<String>,
    state: State<'_, AppState>,
) -> Result<UploadResponse, String> {
    // Get image bytes
    let png_bytes = if let Some(data) = image_data {
        let base64_data = if data.contains(",") {
            data.split(",").last().unwrap_or(&data)
        } else {
            &data
        };
        BASE64.decode(base64_data)
            .map_err(|e| format!("Failed to decode base64: {}", e))?
    } else {
        let current = state.current_screenshot.lock().unwrap();
        current.clone().ok_or("No screenshot to upload")?
    };

    // Get access token
    let token = {
        let token = state.access_token.lock().unwrap();
        token.clone().ok_or("No access token. Please login in settings.")?
    };

    #[cfg(debug_assertions)]
    println!("Uploading: {} ({} bytes)", filename, png_bytes.len());

    let client = reqwest::Client::new();

    // Get or create Screenshots folder
    let parent_id = get_or_create_screenshots_folder(&client, &token).await?;

    // Create multipart form - field name "files[]" as browser does
    let file_part = multipart::Part::bytes(png_bytes)
        .file_name(filename.clone())
        .mime_str("image/png")
        .map_err(|e| format!("Failed to create form part: {}", e))?;

    let form = multipart::Form::new()
        .part("files[]", file_part);

    // POST /fast_upload?parent_id=<id>
    let url = format!("{}/fast_upload?parent_id={}", crate::config::api_base_url(), parent_id);
    #[cfg(debug_assertions)]
    println!("POST {}", url);

    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", token))
        .header("Accept", "application/json")
        .header("X-Requested-With", "XMLHttpRequest")
        .header("User-Agent", "DownloadRuScreenshoter/1.0")
        .multipart(form)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    #[cfg(debug_assertions)]
    println!("Upload response {}: {}", status, &body[..body.len().min(500)]);

    if !status.is_success() {
        return Err(format!("Upload failed with status {}: {}", status, body));
    }

    let api_response: ApiResponse = serde_json::from_str(&body)
        .map_err(|e| format!("Failed to parse response: {} | body: {}", e, &body[..body.len().min(300)]))?;

    // secure_url may be relative like /g/xxx... - prepend domain
    let secure_url = if api_response.object.secure_url.starts_with('/') {
        format!("{}{}", crate::config::api_base_url(), api_response.object.secure_url)
    } else {
        api_response.object.secure_url
    };

    Ok(UploadResponse {
        id: api_response.object.id,
        name: api_response.object.name,
        secure_url,
        shared: api_response.object.shared,
    })
}

/// Set access token
#[tauri::command]
pub fn set_access_token(
    token: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let mut access_token = state.access_token.lock().unwrap();
    *access_token = Some(token);
    Ok(())
}

/// Get access token
#[tauri::command]
pub fn get_access_token(
    state: State<'_, AppState>,
) -> Result<Option<String>, String> {
    let access_token = state.access_token.lock().unwrap();
    Ok(access_token.clone())
}
