use std::{path::PathBuf, sync::Arc};

use axum::{
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse},
    routing::get,
    Router,
};
use compiler::docs::render;
use compiler::print_errors;
use tokio::net::TcpListener;
use tower_http::{compression::CompressionLayer, trace::TraceLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Debug, Clone)]
struct AppState {
    file: Arc<PathBuf>,
}

#[derive(Debug)]
enum Error {
    File(tokio::io::Error),
    BadSyntax(),
}

impl IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        match self {
            Error::File(error) => {
                (StatusCode::INTERNAL_SERVER_ERROR, format!("{error:?}")).into_response()
            }
            Error::BadSyntax() => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Bad Syntax, check your server output"),
            )
                .into_response(),
        }
    }
}

pub async fn run(file: PathBuf) {
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
        file: Arc::new(file.clone()),
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

async fn index(State(state): State<AppState>) -> Result<Html<String>, Error> {
    let file = &*state.file;
    let result = tokio::fs::read_to_string(file).await.map_err(Error::File)?;
    let str = result.as_str();
    match compiler::compile(Some(file.clone()), str) {
        Ok(module) => {
            let result = render(&module);
            Ok(Html(result))
        }

        Err(error) => {
            print_errors(file, str, error);
            Err(Error::BadSyntax())
        }
    }
}
