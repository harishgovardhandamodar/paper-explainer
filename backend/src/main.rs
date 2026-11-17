use paper_explainer::api::ApiState;
use paper_explainer::build_router;
use paper_explainer::llm::LlmClient;
use paper_explainer::store::StoreHandle;
use std::net::SocketAddr;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "paper_explainer=info,tower_http=warn".into()),
        )
        .init();

    let store = StoreHandle::open()?;
    let llm = LlmClient::from_env()?;
    let host = std::env::var("PAPER_EXPLAINER_HOST").unwrap_or_else(|_| "0.0.0.0".into());
    let port: u16 = std::env::var("PAPER_EXPLAINER_PORT")
        .ok().and_then(|v| v.parse().ok()).unwrap_or(8787);
    let dist = PathBuf::from(
        std::env::var("PAPER_EXPLAINER_DIST").unwrap_or_else(|_| "frontend/dist".into()),
    );

    let addr: SocketAddr = format!("{host}:{port}").parse()?;
    let app = build_router(ApiState { store, llm }, &dist);

    tracing::info!("Paper Explainer listening on http://{addr}");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
