# Real-Time Log Anomaly Detection with Rust, Axum, Qdrant, and Ollama

This project demonstrates a powerful, real-time log anomaly detection system built entirely with Rust. It leverages a modern stack to identify unusual log entries by comparing them against a baseline of "normal" behavior using semantic vector search.

This implementation is based on the concepts and architecture detailed in the article: [Real-Time Anomaly Detection with Rust, Axum, Qdrant, and Ollama](https://automatex.win/articles/real-time-anomaly-detection-with-rust-axum-qdrant-and-ollama).

## ✨ Features

- **Real-Time Detection**: Analyzes log entries as they arrive via a simple API endpoint.
- **Semantic Understanding**: Uses language models (via Ollama) to understand the _meaning_ of logs, not just keywords.
- **High Performance**: Built with Rust and the Tokio runtime for asynchronous, non-blocking I/O, ensuring high throughput and low latency.
- **Efficient Vector Search**: Utilizes Qdrant, a high-performance vector database, for fast similarity searches.
- **Memory Safe**: Inherits Rust's compile-time guarantees against data races and common memory bugs.
- **Modular Architecture**: Clear separation of concerns between the web server (Axum), the embedding model (Ollama), and the vector store (Qdrant).

## 🏛️ Architecture

The system operates in two main phases:

1.  **Initialization (Building a Baseline)**:

    - On startup, the server defines a set of "normal" log entries.
    - It sends each normal log to **Ollama** to be converted into a numerical vector embedding using a sentence-transformer model (like `bge-m3`).
    - These vector embeddings are then stored and indexed in a **Qdrant** collection, establishing a baseline of what normal behavior looks like.

2.  **Inference (Checking a New Log)**:
    - The **Axum** web server exposes a `/check_log` API endpoint that accepts a new log entry.
    - This new log is also sent to **Ollama** to get its vector embedding.
    - The server then queries **Qdrant**, searching for the most similar vector(s) from the "normal" baseline.
    - Qdrant returns a similarity score (e.g., Cosine similarity). If this score is below a predefined threshold, the log is flagged as an **anomaly**.

## 🚀 Getting Started

Follow these instructions to get the project up and running on your local machine.

### Prerequisites

1.  **Rust Toolchain**: Install Rust and Cargo.

    ```bash
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
    ```

2.  **Docker & Docker Compose**: Required to run Qdrant and Ollama.

    - [Install Docker](https://docs.docker.com/engine/install/)
    - [Install Docker Compose](https://docs.docker.com/compose/install/)

3.  **OpenSSL Development Libraries**: The Rust compiler needs this to build dependencies that use TLS/SSL.
    ```bash
    # On Debian/Ubuntu
    sudo apt-get update && sudo apt-get install -y libssl-dev pkg-config
    ```

### Installation & Setup

1.  **Clone the repository:**

    ```bash
    git clone https://github.com/openmymai/anomalydetection.git
    cd your-repo-name
    ```

2.  **Start Qdrant and Ollama services:**
    Use the provided `docker-compose.yml` to start the necessary background services.

    ```bash
    docker-compose up -d
    ```

    This will:

    - Start a **Qdrant** instance, accessible at `localhost:6333`.
    - Start an **Ollama** instance, accessible at `localhost:11434`.

3.  **Pull the embedding model into Ollama:**
    We need to download the language model that will generate the vector embeddings. This project uses `bge-m3`.

    ```bash
    docker-compose exec ollama ollama pull bge-m3
    ```

    Wait for the download to complete.

4.  **Run the Rust application:**
    With the background services running, you can now start the anomaly detection server.
    ```bash
    cargo run --release
    ```
    The server will start, initialize the Qdrant collection with the baseline logs, and listen for requests on `127.0.0.1:8080`.

## Usage

Once the server is running, you can send `POST` requests to the `/check_log` endpoint to check if a log entry is anomalous.

### Example: Checking a "Normal" Log

This log is similar to the baseline data. The `is_anomalous` flag should be `false` and the score should be high.

```bash
curl -X POST http://127.0.0.1:8080/check_log \
-H "Content-Type: application/json" \
-d '{
  "log_entry": "INFO: User ''guest'' logged in from IP 192.168.1.50"
}'
```

**Expected Response:**

```json
{
  "is_anomalous": false,
  "score": 0.9215,
  "log_entry": "INFO: User 'guest' logged in from IP 192.168.1.50"
}
```

### Example: Checking an "Anomalous" Log

This log is semantically different from the baseline. The `is_anomalous` flag should be `true` and the score should be low.

```bash
curl -X POST http://127.0.0.1:8080/check_log \
-H "Content-Type: application/json" \
-d '{
  "log_entry": "CRITICAL: Failed to connect to primary database after 5 retries."
}'
```

**Expected Response:**

```json
{
  "is_anomalous": true,
  "score": 0.5831,
  "log_entry": "CRITICAL: Failed to connect to primary database after 5 retries."
}
```

## 🔧 Configuration

You can configure the application by modifying the constants at the top of `src/main.rs`:

- `COLLECTION_NAME`: The name of the collection in Qdrant.
- `EMBEDDING_MODEL`: The model to use from Ollama.
- `VECTOR_SIZE`: The dimensionality of the vectors produced by the model (e.g., `1024` for `bge-m3`).
- `ANOMALY_THRESHOLD`: The similarity score threshold (0.0 to 1.0). Scores below this value are considered anomalies.

## 📜 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
