use crate::api::auth::AuthContext;
use crate::api::AppState;
use axum::{
    extract::{Multipart, Query, State},
    http::{header, HeaderMap, HeaderValue},
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use serde_json::Value;
use std::path::PathBuf;
use tokio::fs;

use crate::error::error::{ApiResponse, AppError};
use crate::util::path::{safe_path_component, safe_relative_parts};
const MAX_IMAGE_UPLOAD_BYTES: usize = 10 * 1024 * 1024;

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: Option<String>,
    pub password: Option<String>,
    #[serde(rename = "isLogin")]
    pub is_login: Option<bool>,
    pub code: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct FileTypeQuery {
    #[serde(rename = "type")]
    pub file_type: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AddUserRequest {
    pub username: Option<String>,
    pub password: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ResetPasswordRequest {
    pub username: Option<String>,
    pub password: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ChangePasswordRequest {
    #[serde(rename = "oldPassword")]
    pub old_password: Option<String>,
    #[serde(rename = "newPassword")]
    pub new_password: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateUserRequest {
    pub username: Option<String>,
    #[serde(rename = "enableWebdav")]
    pub enable_webdav: Option<bool>,
    #[serde(rename = "enableLocalStore")]
    pub enable_local_store: Option<bool>,
    #[serde(rename = "enableAiModel")]
    pub enable_ai_model: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct DeleteFileRequest {
    pub url: Option<String>,
}

pub async fn login(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<LoginRequest>,
) -> Result<Response, AppError> {
    let username = req.username.unwrap_or_default();
    let password = req.password.unwrap_or_default();
    let is_login = req.is_login.unwrap_or(false);
    let is_new_user = !is_login && !username.is_empty();
    let mut data = state
        .user_service
        .login(&username, &password, is_login, req.code.as_deref())
        .await?;
    if is_new_user {
        let _ = state
            .book_source_service
            .copy_default_to_user(&username)
            .await;
    }
    let access_token = data
        .get("accessToken")
        .and_then(Value::as_str)
        .map(str::to_owned);
    if let Some(object) = data.as_object_mut() {
        object.remove("accessToken");
    }
    let mut response = Json(ApiResponse::<Value>::ok(data)).into_response();
    if let Some(token) = access_token {
        set_session_cookie(&mut response, &token, request_is_secure(&headers));
    }
    Ok(response)
}

pub async fn logout(
    State(state): State<AppState>,
    headers: HeaderMap,
    auth: AuthContext,
) -> Result<Response, AppError> {
    if !state.user_service.secure_enabled() {
        return Ok(Json(ApiResponse::<Value>::err("不支持的操作")).into_response());
    }
    if let Some(token) = auth.access_token() {
        let _ = state.user_service.logout(token).await;
    }
    let mut response = Json(ApiResponse::err_with_data(
        "请重新登录",
        Value::String("NEED_LOGIN".to_string()),
    ))
    .into_response();
    clear_session_cookie(&mut response, request_is_secure(&headers));
    Ok(response)
}

pub async fn get_user_info(
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<Value>>, AppError> {
    let (user_info, secure, secure_key_required, admin_authorized) = state
        .user_service
        .get_user_info(auth.access_token(), auth.secure_key())
        .await?;
    let data = serde_json::json!({
        "userInfo": user_info,
        "secure": secure,
        "secureKeyRequired": secure_key_required,
        "adminAuthorized": admin_authorized,
    });
    Ok(Json(ApiResponse::ok(data)))
}

pub async fn save_user_config(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(body): Json<Value>,
) -> Result<Json<ApiResponse<Value>>, AppError> {
    let user_ns = match state
        .user_service
        .resolve_user_ns_with_override(auth.access_token(), auth.secure_key(), auth.user_ns())
        .await
    {
        Ok(ns) => ns,
        Err(_) => {
            return Ok(Json(ApiResponse::err_with_data(
                "请登录后使用",
                Value::String("NEED_LOGIN".to_string()),
            )))
        }
    };
    state.user_service.save_user_config(&user_ns, body).await?;
    Ok(Json(ApiResponse::ok(Value::String("".to_string()))))
}

pub async fn get_user_config(
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<Value>>, AppError> {
    let user_ns = match state
        .user_service
        .resolve_user_ns_with_override(auth.access_token(), auth.secure_key(), auth.user_ns())
        .await
    {
        Ok(ns) => ns,
        Err(_) => {
            return Ok(Json(ApiResponse::err_with_data(
                "请登录后使用",
                Value::String("NEED_LOGIN".to_string()),
            )))
        }
    };
    let cfg = state.user_service.get_user_config(&user_ns).await?;
    Ok(Json(ApiResponse::ok(cfg)))
}

pub async fn get_user_list(
    State(state): State<AppState>,
    auth: AuthContext,
) -> Result<Json<ApiResponse<Value>>, AppError> {
    if !state.user_service.secure_enabled() {
        return Ok(Json(ApiResponse::<Value>::err("不支持的操作")));
    }
    // Check if admin (either by is_admin flag or secure key)
    let is_admin = state
        .user_service
        .is_admin(auth.access_token(), auth.secure_key())
        .await?;
    if !is_admin {
        return Ok(Json(ApiResponse::err_with_data(
            "请输入管理密码",
            Value::String("NEED_SECURE_KEY".to_string()),
        )));
    }
    let list = state.user_service.get_user_list().await?;
    Ok(Json(ApiResponse::ok(Value::from(list))))
}

pub async fn add_user(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<AddUserRequest>,
) -> Result<Json<ApiResponse<Value>>, AppError> {
    if !state.user_service.secure_enabled() {
        return Ok(Json(ApiResponse::<Value>::err("不支持的操作")));
    }
    // Check if admin (either by is_admin flag or secure key)
    let is_admin = state
        .user_service
        .is_admin(auth.access_token(), auth.secure_key())
        .await?;
    if !is_admin {
        return Ok(Json(ApiResponse::err_with_data(
            "请输入管理密码",
            Value::String("NEED_SECURE_KEY".to_string()),
        )));
    }
    let username = req.username.unwrap_or_default();
    let password = req.password.unwrap_or_default();
    let list = state.user_service.add_user(&username, &password).await?;
    Ok(Json(ApiResponse::ok(Value::from(list))))
}

pub async fn reset_password(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<ResetPasswordRequest>,
) -> Result<Json<ApiResponse<Value>>, AppError> {
    if !state.user_service.secure_enabled() {
        return Ok(Json(ApiResponse::<Value>::err("不支持的操作")));
    }
    // Check if admin (either by is_admin flag or secure key)
    let is_admin = state
        .user_service
        .is_admin(auth.access_token(), auth.secure_key())
        .await?;
    if !is_admin {
        return Ok(Json(ApiResponse::err_with_data(
            "请输入管理密码",
            Value::String("NEED_SECURE_KEY".to_string()),
        )));
    }
    let username = req.username.unwrap_or_default();
    let password = req.password.unwrap_or_default();
    state
        .user_service
        .reset_password(&username, &password)
        .await?;
    Ok(Json(ApiResponse::ok(Value::String("".to_string()))))
}

pub async fn change_password(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<ChangePasswordRequest>,
) -> Result<Json<ApiResponse<Value>>, AppError> {
    if !state.user_service.secure_enabled() {
        return Ok(Json(ApiResponse::<Value>::err("不支持的操作")));
    }
    let token = auth
        .access_token()
        .ok_or_else(|| AppError::BadRequest("NEED_LOGIN".to_string()))?;
    let old_password = req.old_password.unwrap_or_default();
    let new_password = req.new_password.unwrap_or_default();
    if old_password.is_empty() || new_password.is_empty() {
        return Err(AppError::BadRequest("请填写当前密码和新密码".to_string()));
    }
    state
        .user_service
        .change_password(token, &old_password, &new_password)
        .await?;
    Ok(Json(ApiResponse::ok(Value::String("".to_string()))))
}

pub async fn delete_users(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(list): Json<Vec<String>>,
) -> Result<Json<ApiResponse<Value>>, AppError> {
    if !state.user_service.secure_enabled() {
        return Ok(Json(ApiResponse::<Value>::err("不支持的操作")));
    }
    // Check if admin (either by is_admin flag or secure key)
    let is_admin = state
        .user_service
        .is_admin(auth.access_token(), auth.secure_key())
        .await?;
    if !is_admin {
        return Ok(Json(ApiResponse::err_with_data(
            "请输入管理密码",
            Value::String("NEED_SECURE_KEY".to_string()),
        )));
    }
    let users = state.user_service.delete_users(&list).await?;
    Ok(Json(ApiResponse::ok(Value::from(users))))
}

pub async fn update_user(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<UpdateUserRequest>,
) -> Result<Json<ApiResponse<Value>>, AppError> {
    if !state.user_service.secure_enabled() {
        return Ok(Json(ApiResponse::<Value>::err("不支持的操作")));
    }
    // Check if admin (either by is_admin flag or secure key)
    let is_admin = state
        .user_service
        .is_admin(auth.access_token(), auth.secure_key())
        .await?;
    if !is_admin {
        return Ok(Json(ApiResponse::err_with_data(
            "请输入管理密码",
            Value::String("NEED_SECURE_KEY".to_string()),
        )));
    }
    let username = req.username.unwrap_or_default();
    let list = state
        .user_service
        .update_user(
            &username,
            req.enable_webdav,
            req.enable_local_store,
            req.enable_ai_model,
        )
        .await?;
    Ok(Json(ApiResponse::ok(Value::from(list))))
}

pub async fn upload_file(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(q): Query<FileTypeQuery>,
    mut multipart: Multipart,
) -> Result<Json<ApiResponse<Value>>, AppError> {
    let user_ns = match state
        .user_service
        .resolve_user_ns_with_override(auth.access_token(), auth.secure_key(), auth.user_ns())
        .await
    {
        Ok(ns) => ns,
        Err(_) => {
            return Ok(Json(ApiResponse::err_with_data(
                "请登录后使用",
                Value::String("NEED_LOGIN".to_string()),
            )))
        }
    };
    let mut file_list = Vec::new();
    if q
        .file_type
        .as_deref()
        .is_some_and(|value| !value.is_empty() && value != "images")
    {
        return Err(AppError::BadRequest("不支持的文件类型".to_string()));
    }
    let file_type = "images".to_string();
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::BadRequest(e.to_string()))?
    {
        let name = field
            .file_name()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "file".to_string());
        let name = safe_path_component(&name, "文件名")
            .map_err(|_| AppError::BadRequest("文件名非法".to_string()))?;
        let extension = name.rsplit_once('.').map(|(_, ext)| ext.to_ascii_lowercase());
        if !matches!(
            extension.as_deref(),
            Some("png" | "jpg" | "jpeg" | "webp" | "gif" | "bmp")
        ) {
            return Err(AppError::BadRequest("仅允许图片文件".to_string()));
        }
        let data = field
            .bytes()
            .await
            .map_err(|e| AppError::BadRequest(e.to_string()))?;
        if data.len() > MAX_IMAGE_UPLOAD_BYTES {
            return Err(AppError::BadRequest("图片文件超过大小限制".to_string()));
        }
        let safe_user_ns = safe_path_component(&user_ns, "用户命名空间")
            .map_err(|_| AppError::BadRequest("用户命名空间非法".to_string()))?;
        let dir = PathBuf::from(&state.config.storage_dir)
            .join("assets")
            .join(&safe_user_ns)
            .join(&file_type);
        fs::create_dir_all(&dir)
            .await
            .map_err(|e| AppError::Internal(e.into()))?;
        let path = dir.join(&name);
        fs::write(&path, data)
            .await
            .map_err(|e| AppError::Internal(e.into()))?;
        let url = format!("/assets/{}/{}/{}", safe_user_ns, file_type, name);
        file_list.push(Value::String(url));
    }
    Ok(Json(ApiResponse::ok(Value::from(file_list))))
}

pub async fn delete_file(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<DeleteFileRequest>,
) -> Result<Json<ApiResponse<Value>>, AppError> {
    let user_ns = match state
        .user_service
        .resolve_user_ns_with_override(auth.access_token(), auth.secure_key(), auth.user_ns())
        .await
    {
        Ok(ns) => ns,
        Err(_) => {
            return Ok(Json(ApiResponse::err_with_data(
                "请登录后使用",
                Value::String("NEED_LOGIN".to_string()),
            )))
        }
    };
    let url = req.url.unwrap_or_default();
    let prefix = format!("/assets/{}/images/", user_ns);
    if !url.starts_with(&prefix) {
        return Ok(Json(ApiResponse::err("文件链接错误")));
    }
    let parts = safe_relative_parts(&url[prefix.len()..])
        .map_err(|_| AppError::BadRequest("文件链接错误".to_string()))?;
    if parts.len() != 1 {
        return Ok(Json(ApiResponse::err("文件链接错误")));
    }
    let safe_user_ns = safe_path_component(&user_ns, "用户命名空间")
        .map_err(|_| AppError::BadRequest("用户命名空间非法".to_string()))?;
    let full_path = PathBuf::from(&state.config.storage_dir)
        .join("assets")
        .join(safe_user_ns)
        .join("images")
        .join(&parts[0]);
    let _ = fs::remove_file(full_path).await;
    Ok(Json(ApiResponse::ok(Value::String("".to_string()))))
}

fn request_is_secure(headers: &HeaderMap) -> bool {
    headers
        .get("x-forwarded-proto")
        .and_then(|value| value.to_str().ok())
        .map(|value| value.split(',').next().unwrap_or_default().trim().eq_ignore_ascii_case("https"))
        .unwrap_or(false)
        || headers
            .get("x-forwarded-ssl")
            .and_then(|value| value.to_str().ok())
            .is_some_and(|value| value.eq_ignore_ascii_case("on"))
}

fn set_session_cookie(response: &mut Response, token: &str, secure: bool) {
    let suffix = if secure { "; Secure" } else { "" };
    if let Ok(value) = HeaderValue::from_str(&format!(
        "reader_session={token}; Path=/; HttpOnly; SameSite=Lax; Max-Age=604800{suffix}"
    )) {
        response.headers_mut().append(header::SET_COOKIE, value);
    }
}

fn clear_session_cookie(response: &mut Response, secure: bool) {
    let suffix = if secure { "; Secure" } else { "" };
    if let Ok(value) = HeaderValue::from_str(&format!(
        "reader_session=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0{suffix}"
    )) {
        response.headers_mut().append(header::SET_COOKIE, value);
    }
}
