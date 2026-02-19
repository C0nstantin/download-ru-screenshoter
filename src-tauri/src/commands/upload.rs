use reqwest::multipart;
use sha1::{Sha1, Digest};
use tauri::State;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

use crate::state::AppState;

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

/// Upload screenshot to download.ru
#[tauri::command]
pub async fn upload_to_download(
    filename: String,
    image_data: Option<String>,
    state: State<'_, AppState>,
) -> Result<UploadResponse, String> {
    // Get image bytes
    let png_bytes = if let Some(data) = image_data {
        // Decode base64 (remove data URL prefix if present)
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
        token.clone().ok_or("No access token set. Please configure in settings.")?
    };

    // Calculate SHA1 and CRC32
    let mut hasher = Sha1::new();
    hasher.update(&png_bytes);
    let sha1_result = hasher.finalize();
    let sha1_hex = format!("{:x}", sha1_result);

    let crc32_value = crc32fast::hash(&png_bytes);

    let file_size = png_bytes.len();

    // Create multipart form
    let file_part = multipart::Part::bytes(png_bytes)
        .file_name(filename.clone())
        .mime_str("image/png")
        .map_err(|e| format!("Failed to create form part: {}", e))?;

    let form = multipart::Form::new()
        .part("file[data]", file_part)
        .text("file[data][original_filename]", filename.clone())
        .text("file[data][sha1]", sha1_hex)
        .text("file[data][size]", file_size.to_string())
        .text("file[data][crc32]", crc32_value.to_string())
        .text("file[shared]", "true");

    // Send request
    let client = reqwest::Client::new();
    let response = client
        .post("https://download.ru/fast_upload")
        .header("Authorization", format!("Bearer {}", token))
        .header("X-Content-Type", "image/png")
        .header("User-Agent", "DownloadScreenshoter/1.0")
        .multipart(form)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("Upload failed with status {}: {}", status, body));
    }

    let api_response: ApiResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

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
