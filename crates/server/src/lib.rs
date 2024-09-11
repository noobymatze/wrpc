use std::{path::PathBuf, sync::Arc};

use axum::{extract::State, response::Html, routing::get, Router};
use compiler::{
    ast::canonical::Module,
    docs::{self, render},
};
use dotenvy::dotenv;
use tokio::net::TcpListener;
use tower_http::{compression::CompressionLayer, trace::TraceLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Debug, Clone)]
struct AppState {
    module: Arc<Module>,
}

pub async fn run(module: &Module) {
    //dotenv().expect("There should be a .env file");

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                // axum logs rejections from built-in extractors with the `axum::rejection`
                // target, at `TRACE` level. `axum::rejection=trace` enables showing those events
                "wrpc=debug,tower_http=debug,axum::rejection=trace".into()
            }),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let state = AppState {
        module: Arc::new(module.clone()),
    };

    // build our application with a route
    let app = Router::new()
        .route("/", get(index))
        .layer(TraceLayer::new_for_http())
        .layer(CompressionLayer::new())
        .with_state(state);

    // run it
    let listener = TcpListener::bind("127.0.0.1:3000").await.unwrap();
    tracing::debug!(
        "✅ Server started, listening on http://localhost:{}",
        listener.local_addr().unwrap()
    );
    axum::serve(listener, app).await.unwrap();
}

async fn index(State(state): State<AppState>) -> Html<String> {
    let result = render(&state.module);
    Html(result)
}
