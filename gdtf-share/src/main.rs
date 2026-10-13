use std::sync::Arc;

use gdtf_share::{AppState, router, store::Store};

#[tokio::main]
async fn main() {
    let data_dir = std::env::var("GDTF_SHARE_DATA").unwrap_or_else(|_| "data".to_owned());
    let addr = std::env::var("GDTF_SHARE_ADDR").unwrap_or_else(|_| "127.0.0.1:8787".to_owned());
    let admin_token = std::env::var("GDTF_SHARE_ADMIN_TOKEN")
        .ok()
        .filter(|token| token.len() >= 12)
        .map(Arc::<str>::from);
    if admin_token.is_none() {
        eprintln!("GDTF_SHARE_ADMIN_TOKEN is unset or shorter than 12 characters: administration is disabled");
    }

    let store = Store::open(&data_dir).unwrap_or_else(|e| panic!("cannot open {data_dir}: {e:?}"));
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .unwrap_or_else(|e| panic!("cannot bind {addr}: {e}"));
    println!("Iris GDTF Share listening on http://{addr} (data: {data_dir})");
    axum::serve(listener, router(AppState { store: Arc::new(store), admin_token }))
        .await
        .expect("server failed");
}
