use entropa_core::Probe;
use std::time::{SystemTime, UNIX_EPOCH};

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

#[tokio::main]
async fn main() {
    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".into());
    let origin = std::env::var("CORS_ORIGIN").ok().filter(|s| !s.is_empty());
    let router = clinic_api::app(Probe::spawn(), now, origin);
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await.expect("bind");
    println!("clinic-api listening on {port}");
    axum::serve(listener, router).await.expect("serve");
}
