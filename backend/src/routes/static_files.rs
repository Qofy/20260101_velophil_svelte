// Static file serving with WASM support and SPA fallback
use actix_files::NamedFile;
use actix_web::{get, web, HttpRequest, HttpResponse, Result};
use actix_web::http::header::{self, HeaderValue};
use std::path::PathBuf;

/// Serve WASM files with correct MIME type
#[get("/assets/{file:.+\\.wasm}")]
pub async fn serve_wasm(path: web::Path<String>, req: HttpRequest) -> Result<HttpResponse> {
    let file_rel = path.into_inner();
    let full_path = format!("./static/assets/{}", file_rel);
    let file = NamedFile::open_async(full_path).await?;
    let mut res = file.into_response(&req);
    res.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/wasm"),
    );
    Ok(res)
}

/// SPA fallback - serve index.html for non-API routes
/// This enables HTML5 history mode routing in the frontend
pub async fn spa_fallback(req: HttpRequest) -> Result<NamedFile> {
    let path: PathBuf = req.match_info().query("tail").parse()?;

    // Don't fallback for API routes or static assets
    let path_str = path.to_str().unwrap_or("");
    if path_str.starts_with("api/") || path_str.starts_with("assets/") {
        return Err(actix_web::error::ErrorNotFound("Not found"));
    }

    // Serve index.html for all other routes (SPA routing)
    let index_path = PathBuf::from("./static/index.html");
    NamedFile::open_async(index_path)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e))
}
