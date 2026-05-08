use std::sync::Arc;

use axum::{
    http::StatusCode,
    middleware,
    routing::{get, post},
    Router,
};
use tower_http::trace::TraceLayer;

use crate::{
    config::AppConfig,
    handlers::{
        auth::{handle_cpsess_get, handle_login, handle_root},
        dashboard::handle_dashboard,
        fallback::{handle_fallback, handle_file_serve, handle_raw_post},
        terminal::handle_terminal,
        upload::handle_upload,
    },
};

/// Constructs the full Axum router for a single honeypot port instance.
pub fn make_router(config: Arc<AppConfig>) -> Router {
    Router::new()
        // ── Favicon (reduce 404 noise) ──────────────────────────────────
        .route("/favicon.ico", get(|| async { StatusCode::NO_CONTENT }))
        // ── Authentication flow (scanner compatibility) ──────────────────
        .route("/login", get(handle_login))
        .route("/", get(handle_root))
        .route("/cpsess{token}", get(handle_cpsess_get))
        .route("/cpsess{token}/", get(handle_cpsess_get))
        // WHM proxy-subdomain variants
        .route("/___proxy_subdomain_whm/login", get(handle_login))
        .route("/___proxy_subdomain_whm/", get(handle_root))
        .route(
            "/___proxy_subdomain_whm/cpsess{token}",
            get(handle_cpsess_get),
        )
        .route(
            "/___proxy_subdomain_whm/cpsess{token}/",
            get(handle_cpsess_get),
        )
        // cPanel proxy-subdomain variants
        .route("/___proxy_subdomain_cpanel/login", get(handle_login))
        .route("/___proxy_subdomain_cpanel/", get(handle_root))
        .route(
            "/___proxy_subdomain_cpanel/cpsess{token}",
            get(handle_cpsess_get),
        )
        .route(
            "/___proxy_subdomain_cpanel/cpsess{token}/",
            get(handle_cpsess_get),
        )
        // ── Fake File Manager dashboard ──────────────────────────────────
        .route("/cpsess{token}/fm", get(handle_dashboard))
        .route("/cpsess{token}/fm/", get(handle_dashboard))
        .route("/cpsess{token}/filemanager", get(handle_dashboard))
        .route("/cpsess{token}/filemanager/", get(handle_dashboard))
        // ── Upload capture ───────────────────────────────────────────────
        .route("/cpsess{token}/upload", post(handle_upload))
        .route("/cpsess{token}/upload_files", post(handle_upload))
        .route("/execute/Fileman/upload_files", post(handle_upload))
        .route(
            "/frontend/paper_lantern/filemanager/upload-ajax.html",
            post(handle_upload),
        )
        // ── Terminal / command capture ───────────────────────────────────
        .route("/cpsess{token}/term", post(handle_terminal))
        .route("/cpsess{token}/terminal", post(handle_terminal))
        .route("/execute/Terminal/get_session", post(handle_terminal))
        // ── File serving (previously uploaded payloads) ──────────────────
        .route("/cpsess{token}/{*subpath}", get(handle_file_serve))
        // ── Generic POST catch-all ───────────────────────────────────────
        .route("/{*path}", post(handle_raw_post))
        // ── Fallback for unmatched GET requests ──────────────────────────
        .fallback(handle_fallback)
        .layer(middleware::map_response(crate::handlers::headers::inject_headers))
        .layer(TraceLayer::new_for_http())
        .with_state(config)
}
