use axum::extract::FromRequestParts;
use axum::http::{header::AUTHORIZATION, request::Parts};
use std::convert::Infallible;

#[derive(Debug, Clone, Default)]
pub struct AuthContext {
    pub access_token: Option<String>,
    pub secure_key: Option<String>,
    pub user_ns: Option<String>,
}

impl AuthContext {
    pub fn access_token(&self) -> Option<&str> {
        self.access_token.as_deref()
    }

    pub fn secure_key(&self) -> Option<&str> {
        self.secure_key.as_deref()
    }

    pub fn user_ns(&self) -> Option<&str> {
        self.user_ns.as_deref()
    }
}

#[axum::async_trait]
impl<S> FromRequestParts<S> for AuthContext
where
    S: Send + Sync,
{
    type Rejection = Infallible;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let headers = &parts.headers;
        let mut access_token = headers
            .get(AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(parse_authorization_token);
        if access_token.is_none() {
            access_token = cookie_value(headers, "reader_session");
        }
        let secure_key = headers
            .get("X-Secure-Key")
            .and_then(|v| v.to_str().ok())
            .map(|v| v.to_string());
        let user_ns = headers
            .get("X-User-NS")
            .and_then(|v| v.to_str().ok())
            .map(|v| v.to_string());

        Ok(Self {
            access_token,
            secure_key,
            user_ns,
        })
    }
}

fn cookie_value(
    headers: &axum::http::HeaderMap,
    name: &str,
) -> Option<String> {
    headers
        .get(axum::http::header::COOKIE)?
        .to_str()
        .ok()?
        .split(';')
        .filter_map(|part| {
            let (key, value) = part.trim().split_once('=')?;
            (key == name).then(|| value.trim().to_string())
        })
        .next()
}

fn parse_authorization_token(raw: &str) -> Option<String> {
    let value = raw.trim();
    if value.is_empty() {
        return None;
    }
    if let Some(token) = value
        .strip_prefix("Bearer ")
        .or_else(|| value.strip_prefix("bearer "))
    {
        let token = token.trim();
        return (!token.is_empty()).then(|| token.to_string());
    }
    Some(value.to_string())
}
