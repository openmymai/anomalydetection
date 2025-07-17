use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::post,
    Router,
};

use qdrant_client::qdrant::{
    CreateCollection, Distance, PointStruct, SearchPoints, UpsertPoints, VectorParams, VectorsConfig,
};
use qdrant_client::Qdrant;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use qdrant_client::qdrant::vectors_config::Config;
use serde_json::{Map, Value};

const COLLECTION_NAME: &str = "normal_server_logs_axum";
const EMBEDDING_MODEL: &str = "bge-m3";
const VECTOR_SIZE: u64 = 1024; 

#[derive(Clone)]
struct AppState {
    qdrant_client: Arc<Qdrant>,
    http_client: reqwest::Client,
}

#[derive(Serialize)]
struct OllamaEmbeddingRequest<'a> {
    model: &'a str,
    prompt: &'a str,
}

#[derive(Deserialize)]
struct OllamaEmbeddingResponse {
    embedding: Vec<f32>,
}

#[derive(Deserialize)]
struct CheckLogRequest {
    log_entry: String,
}

#[derive(Serialize)]
struct AnomalyResponse {
    is_anomalous: bool,
    score: f32,
    log_entry: String,
}

async fn get_embedding(http_client: &reqwest::Client, log_entry: &str) -> anyhow::Result<Vec<f32>> {
    let response = http_client
        .post("http://localhost:11434/api/embeddings")
        .json(&OllamaEmbeddingRequest {
            model: EMBEDDING_MODEL,
            prompt: log_entry,
        })
        .send()
        .await?
        .json::<OllamaEmbeddingResponse>()
        .await?;
    Ok(response.embedding)
}


async fn initialize_qdrant_baseline(state: &AppState) -> anyhow::Result<()> {
    let _ = state.qdrant_client.delete_collection(COLLECTION_NAME).await;

    state
        .qdrant_client
        .create_collection(CreateCollection {
            collection_name: COLLECTION_NAME.to_string(),
            vectors_config: Some(VectorsConfig {
                config: Some(Config::Params(VectorParams {
                    size: VECTOR_SIZE,
                    distance: Distance::Cosine.into(),
                    ..Default::default()
                })),
            }),
            ..Default::default()
        })
        .await?;

    let normal_logs = vec![
        "INFO: User 'admin' logged in successfully from IP 192.168.1.10",
        "INFO: Service 'database-connector' started successfully on port 5432",
        "DEBUG: Cache cleared for user session 'user123'",
        "INFO: GET /api/v1/users request processed in 25ms",
        "INFO: Scheduled backup job 'daily-backup' completed successfully.",
    ];

    let mut points = Vec::new();
    for (i, log) in normal_logs.iter().enumerate() {
        let vector = get_embedding(&state.http_client, log).await?;
        let payload: Map<String, Value> = serde_json::from_str(&format!(r#"{{"log": "{}"}}"#, log))?;
        points.push(PointStruct::new(i as u64, vector, payload));
    }

    state
        .qdrant_client
        .upsert_points(UpsertPoints {
            collection_name: COLLECTION_NAME.to_string(),
            points,
            wait: Some(true),
            ..Default::default()
        })
        .await?;

    tracing::info!("Successfully indexed {} normal log entries.", normal_logs.len());
    Ok(())
}

async fn check_log_handler(
    State(state): State<AppState>,
    Json(payload): Json<CheckLogRequest>,
) -> impl IntoResponse {
    const ANOMALY_THRESHOLD: f32 = 0.70; 

    let vector = match get_embedding(&state.http_client, &payload.log_entry).await {
        Ok(v) => v,
        Err(e) => {
            tracing::error!("Failed to get embedding: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to get embedding").into_response();
        }
    };

    let search_result = match state
        .qdrant_client
        .search_points(SearchPoints {
            collection_name: COLLECTION_NAME.to_string(),
            vector,
            limit: 1,
            with_payload: Some(true.into()),
            ..Default::default()
        })
        .await
    {
        Ok(res) => res,
        Err(e) => {
            tracing::error!("Qdrant search failed: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, "Qdrant search failed").into_response();
        }
    };

    let mut score = 0.0;
    let mut is_anomalous = true;

    if let Some(closest_point) = search_result.result.into_iter().next() {
        score = closest_point.score;
        if score >= ANOMALY_THRESHOLD {
            is_anomalous = false;
        }
    }

    Json(AnomalyResponse {
        is_anomalous,
        score,
        log_entry: payload.log_entry,
    })
    .into_response()
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::new("info"))
        .init();

    let app_state = AppState {
        qdrant_client: Arc::new(Qdrant::from_url("http://localhost:6334").build()?),
        http_client: reqwest::Client::new(),
    };

    initialize_qdrant_baseline(&app_state).await?;

    let app = Router::new()
        .route("/check_log", post(check_log_handler))
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080").await?;
    tracing::info!("listening on {}", listener.local_addr()?);
    axum::serve(listener, app).await?;

    Ok(())
}