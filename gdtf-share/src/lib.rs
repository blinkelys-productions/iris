//! Iris GDTF Share: a small HTTP service where users share GDTF fixture profiles
//! and Iris maintains an official, moderated catalogue.

pub mod gdtf;
pub mod store;

use std::sync::Arc;

use axum::{
    Json, Router,
    body::Bytes,
    extract::{DefaultBodyLimit, Path, Query, State},
    http::{HeaderMap, StatusCode, header},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
};
use serde::Deserialize;
use serde_json::json;

use store::{PublicRecord, Source, Store, StoreError, Uploaded};

#[derive(Clone)]
pub struct AppState {
    pub store: Arc<Store>,
    /// Administrator bearer token; admin endpoints are disabled when `None`.
    pub admin_token: Option<Arc<str>>,
}

const ADMIN_PAGE: &str = include_str!("admin.html");

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/", get(|| async { Html(ADMIN_PAGE) }))
        .route("/api/v1/health", get(health))
        .route("/api/v1/profiles", get(list).post(upload_community))
        .route("/api/v1/profiles/{id}", get(show).delete(delete_community))
        .route("/api/v1/profiles/{id}/download", get(download))
        .route("/api/v1/admin/profiles", post(upload_official))
        .route("/api/v1/admin/profiles/{id}", axum::routing::delete(delete_admin))
        .route("/api/v1/admin/profiles/{id}/promote", post(promote))
        .route("/api/v1/admin/profiles/{id}/demote", post(demote))
        .layer(DefaultBodyLimit::max(gdtf::MAX_ARCHIVE_BYTES + 1024))
        .with_state(state)
}

type ApiResult<T> = Result<T, Response>;

fn error(status: StatusCode, message: impl Into<String>) -> Response {
    (status, Json(json!({ "error": message.into() }))).into_response()
}

fn store_error(error_value: StoreError) -> Response {
    match error_value {
        StoreError::Invalid(message) => error(StatusCode::UNPROCESSABLE_ENTITY, message),
        StoreError::NotFound => error(StatusCode::NOT_FOUND, "profile not found"),
        StoreError::Forbidden => error(StatusCode::FORBIDDEN, "not allowed"),
        StoreError::Io(message) => {
            eprintln!("storage error: {message}");
            error(StatusCode::INTERNAL_SERVER_ERROR, "storage error")
        }
    }
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

fn require_admin(state: &AppState, headers: &HeaderMap) -> ApiResult<()> {
    let Some(expected) = &state.admin_token else {
        return Err(error(StatusCode::FORBIDDEN, "administration is disabled on this server"));
    };
    let supplied = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .unwrap_or("");
    if constant_time_eq(supplied.as_bytes(), expected.as_bytes()) {
        Ok(())
    } else {
        Err(error(StatusCode::UNAUTHORIZED, "invalid administrator token"))
    }
}

async fn health(State(state): State<AppState>) -> Json<serde_json::Value> {
    let (official, community) = state.store.stats();
    Json(json!({ "ok": true, "official": official, "community": community }))
}

#[derive(Deserialize)]
struct ListQuery {
    q: Option<String>,
    source: Option<String>,
}

async fn list(State(state): State<AppState>, Query(query): Query<ListQuery>) -> ApiResult<Json<Vec<PublicRecord>>> {
    let source = match query.source.as_deref() {
        None | Some("") | Some("all") => None,
        Some("official") => Some(Source::Official),
        Some("community") => Some(Source::Community),
        Some(_) => return Err(error(StatusCode::BAD_REQUEST, "source must be official, community or all")),
    };
    Ok(Json(state.store.list(query.q.as_deref().unwrap_or(""), source)))
}

async fn show(State(state): State<AppState>, Path(id): Path<String>) -> ApiResult<Json<PublicRecord>> {
    state.store.get(&id).map(Json).ok_or_else(|| store_error(StoreError::NotFound))
}

async fn download(State(state): State<AppState>, Path(id): Path<String>) -> ApiResult<Response> {
    let (record, bytes) = state.store.download(&id).map_err(store_error)?;
    let file: String = format!("{} {}", record.manufacturer, record.name)
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect();
    Ok((
        [
            (header::CONTENT_TYPE, "application/octet-stream".to_owned()),
            (header::CONTENT_DISPOSITION, format!("attachment; filename=\"{file}.gdtf\"")),
        ],
        bytes,
    )
        .into_response())
}

fn uploaded_response(uploaded: Uploaded) -> Response {
    match uploaded {
        Uploaded::Created { record, delete_token } => (
            StatusCode::CREATED,
            Json(json!({ "profile": record, "delete_token": delete_token, "duplicate": false })),
        )
            .into_response(),
        Uploaded::Existing(record) => (
            StatusCode::OK,
            Json(json!({ "profile": record, "delete_token": null, "duplicate": true })),
        )
            .into_response(),
    }
}

fn uploader(headers: &HeaderMap) -> String {
    headers
        .get("x-uploader")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_owned()
}

async fn upload_community(State(state): State<AppState>, headers: HeaderMap, body: Bytes) -> ApiResult<Response> {
    let uploaded = state.store.upload(&body, Source::Community, &uploader(&headers)).map_err(store_error)?;
    Ok(uploaded_response(uploaded))
}

async fn upload_official(State(state): State<AppState>, headers: HeaderMap, body: Bytes) -> ApiResult<Response> {
    require_admin(&state, &headers)?;
    let name = match uploader(&headers) {
        name if name.is_empty() => "Iris".to_owned(),
        name => name,
    };
    let uploaded = state.store.upload(&body, Source::Official, &name).map_err(store_error)?;
    Ok(uploaded_response(uploaded))
}

async fn delete_community(State(state): State<AppState>, headers: HeaderMap, Path(id): Path<String>) -> ApiResult<StatusCode> {
    let token = headers.get("x-delete-token").and_then(|v| v.to_str().ok()).unwrap_or("");
    state.store.delete(&id, Some(token)).map_err(store_error)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn delete_admin(State(state): State<AppState>, headers: HeaderMap, Path(id): Path<String>) -> ApiResult<StatusCode> {
    require_admin(&state, &headers)?;
    state.store.delete(&id, None).map_err(store_error)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn promote(State(state): State<AppState>, headers: HeaderMap, Path(id): Path<String>) -> ApiResult<Json<PublicRecord>> {
    require_admin(&state, &headers)?;
    state.store.set_source(&id, Source::Official).map(Json).map_err(store_error)
}

async fn demote(State(state): State<AppState>, headers: HeaderMap, Path(id): Path<String>) -> ApiResult<Json<PublicRecord>> {
    require_admin(&state, &headers)?;
    state.store.set_source(&id, Source::Community).map(Json).map_err(store_error)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::Request};
    use tower::ServiceExt;

    async fn call(app: &Router, request: Request<Body>) -> (StatusCode, Bytes) {
        let response = app.clone().oneshot(request).await.unwrap();
        let status = response.status();
        (status, axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap())
    }

    #[tokio::test]
    async fn community_and_admin_flow() {
        let dir = std::env::temp_dir().join(format!("gdtf-share-http-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let app = router(AppState {
            store: Arc::new(Store::open(&dir).unwrap()),
            admin_token: Some("secret".into()),
        });
        let archive = gdtf::sample_archive("Spot");

        let post = |uri: &str, auth: Option<&str>, body: Vec<u8>| {
            let mut b = Request::post(uri.to_owned());
            if let Some(a) = auth {
                b = b.header("authorization", format!("Bearer {a}"));
            }
            b.body(Body::from(body)).unwrap()
        };

        let (status, _) = call(&app, post("/api/v1/profiles", None, b"junk".to_vec())).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);

        let (status, body) = call(&app, post("/api/v1/profiles", None, archive.clone())).await;
        assert_eq!(status, StatusCode::CREATED);
        let created: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let id = created["profile"]["id"].as_str().unwrap().to_owned();
        assert!(created["delete_token"].is_string());
        assert!(created["profile"].get("delete_token_hash").is_none());

        let (status, _) = call(&app, post(&format!("/api/v1/admin/profiles/{id}/promote"), None, vec![])).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        let (status, _) = call(&app, post(&format!("/api/v1/admin/profiles/{id}/promote"), Some("secret"), vec![])).await;
        assert_eq!(status, StatusCode::OK);

        let (_, body) = call(&app, Request::get("/api/v1/profiles?source=official&q=spot").body(Body::empty()).unwrap()).await;
        assert_eq!(serde_json::from_slice::<serde_json::Value>(&body).unwrap().as_array().unwrap().len(), 1);

        let (status, body) = call(&app, Request::get(format!("/api/v1/profiles/{id}/download")).body(Body::empty()).unwrap()).await;
        assert_eq!((status, body.to_vec()), (StatusCode::OK, archive));

        let delete = Request::delete(format!("/api/v1/admin/profiles/{id}"))
            .header("authorization", "Bearer secret")
            .body(Body::empty())
            .unwrap();
        assert_eq!(call(&app, delete).await.0, StatusCode::NO_CONTENT);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
