use reqwest::multipart;
use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;
use tauri_plugin_store::StoreExt;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

use crate::state::AppState;

const CLIENT_ID: &str = "d7823661467bdfd60827315d82634474fe9c6ab2bc72944206b7920072e2c6bd";
const CLIENT_SECRET: &str = "b5bd3e7d95704ff6c3b38d5f893c0094c39d0023a8fb046d577c2d38c5586823";
const REDIRECT_URI: &str = "urn:ietf:wg:oauth:2.0:oob";
const STORE_FILE: &str = "auth.json";

#[derive(serde::Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    #[allow(dead_code)]
    token_type: Option<String>,
    #[allow(dead_code)]
    expires_in: Option<u64>,
}

/// Open browser to OAuth authorization page
#[tauri::command]
pub fn open_oauth_browser(app: AppHandle) -> Result<(), String> {
    let url = format!(
        "https://download.ru/oauth/authorize?client_id={}&redirect_uri={}&response_type=code&scope=",
        CLIENT_ID, REDIRECT_URI
    );
    app.opener().open_url(&url, None::<&str>)
        .map_err(|e| format!("Failed to open browser: {}", e))
}

/// Exchange OAuth code for access token
#[tauri::command]
pub async fn exchange_oauth_code(
    code: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let client = reqwest::Client::new();
    let response = client
        .post("https://download.ru/oauth/token")
        .form(&[
            ("client_id", CLIENT_ID),
            ("client_secret", CLIENT_SECRET),
            ("code", &code),
            ("redirect_uri", REDIRECT_URI),
            ("grant_type", "authorization_code"),
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

    // Save tokens to store
    let store = app.store(STORE_FILE)
        .map_err(|e| format!("Failed to open store: {}", e))?;
    store.set("access_token", serde_json::json!(token_resp.access_token.clone()));
    if let Some(ref rt) = token_resp.refresh_token {
        store.set("refresh_token", serde_json::json!(rt));
    }
    store.save().map_err(|e| format!("Failed to save store: {}", e))?;

    // Update in-memory state
    let mut access_token = state.access_token.lock().unwrap();
    *access_token = Some(token_resp.access_token);

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
        .post("https://download.ru/oauth/token")
        .form(&[
            ("client_id", CLIENT_ID),
            ("client_secret", CLIENT_SECRET),
            ("refresh_token", &refresh_token),
            ("grant_type", "refresh_token"),
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

/// Logout - clear saved tokens
#[tauri::command]
pub fn logout(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
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
    object: FolderContents,
}

#[derive(serde::Deserialize)]
struct FolderContents {
    folders: Vec<FolderItem>,
}

#[derive(serde::Deserialize)]
struct FolderItem {
    id: String,
    name: String,
}

#[derive(serde::Deserialize)]
struct CreateFolderResponse {
    object: FolderItem,
}

/// Get or create .screenshots folder, return its id
async fn get_or_create_screenshots_folder(client: &reqwest::Client, token: &str) -> Result<String, String> {
    // List root folders
    let resp = client
        .get("https://download.ru/folders.json")
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

    // Look for .screenshots
    if let Some(f) = folder_list.object.folders.iter().find(|f| f.name == ".screenshots") {
        println!("Found .screenshots folder: {}", f.id);
        return Ok(f.id.clone());
    }

    // Create .screenshots folder
    println!("Creating .screenshots folder...");
    let resp = client
        .post("https://download.ru/folders.json")
        .header("Authorization", format!("Bearer {}", token))
        .header("Accept", "application/json")
        .form(&[("folder[name]", ".screenshots")])
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

    println!("Created .screenshots folder: {}", created.object.id);
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

    println!("Uploading: {} ({} bytes)", filename, png_bytes.len());

    let client = reqwest::Client::new();

    // Get or create .screenshots folder
    let parent_id = get_or_create_screenshots_folder(&client, &token).await?;

    // Create multipart form - field name "files[]" as browser does
    let file_part = multipart::Part::bytes(png_bytes)
        .file_name(filename.clone())
        .mime_str("image/png")
        .map_err(|e| format!("Failed to create form part: {}", e))?;

    let form = multipart::Form::new()
        .part("files[]", file_part);

    // POST /fast_upload?parent_id=<id>
    let url = format!("https://download.ru/fast_upload?parent_id={}", parent_id);
    println!("POST {}", url);

    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", token))
        .header("Accept", "application/json")
        .header("X-Requested-With", "XMLHttpRequest")
        .header("User-Agent", "DownloadScreenshoter/1.0")
        .multipart(form)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    println!("Upload response {}: {}", status, &body[..body.len().min(500)]);

    if !status.is_success() {
        return Err(format!("Upload failed with status {}: {}", status, body));
    }

    let api_response: ApiResponse = serde_json::from_str(&body)
        .map_err(|e| format!("Failed to parse response: {} | body: {}", e, &body[..body.len().min(300)]))?;

    Ok(UploadResponse {
        id: api_response.object.id,
        name: api_response.object.name,
        secure_url: api_response.object.secure_url,
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
