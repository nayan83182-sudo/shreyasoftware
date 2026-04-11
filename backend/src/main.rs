use axum::{
    routing::{get, post},
    Router,
    response::IntoResponse,
    Json,
};
use serde_json::json;

#[tokio::main]
async fn main() {
    // Build our application with routes
    let app = Router::new()
        .route("/api/v1/products", get(get_products))
        .route("/api/v1/products/:id", get(get_product_details))
        .route("/api/v1/inventory/:product_id", get(get_inventory))
        .route("/api/v1/checkout", post(checkout))
        .route("/api/v1/plugin/import", post(plugin_import))
        .route("/api/v1/webhooks/vendor", post(vendor_webhook));

    // Run it with hyper on localhost:3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("Server running on http://0.0.0.0:3000");
    axum::serve(listener, app).await.unwrap();
}

async fn get_products() -> impl IntoResponse {
    Json(json!({ "message": "List of products" }))
}

async fn get_product_details() -> impl IntoResponse {
    Json(json!({ "message": "Product details" }))
}

async fn get_inventory() -> impl IntoResponse {
    Json(json!({ "message": "Inventory level" }))
}

async fn checkout() -> impl IntoResponse {
    Json(json!({ "message": "Checkout processed" }))
}

async fn plugin_import() -> impl IntoResponse {
    Json(json!({ "message": "Product imported" }))
}

async fn vendor_webhook() -> impl IntoResponse {
    Json(json!({ "message": "Webhook received" }))
}
