use super::{EngineAdapter, EngineMetrics, EngineStatus, EngineType, ModelInfo};
use async_trait::async_trait;
use serde::Deserialize;
use std::time::Duration;

/// Adapter for the Ollama runtime (default `http://localhost:11434`).
///
/// Ollama exposes a small REST surface — `/api/version` (liveness), `/api/ps`
/// (models currently loaded into memory), and `/api/tags` (on-disk catalog) —
/// but no Prometheus `/metrics` endpoint. Only liveness and the loaded-model
/// identity are therefore available; `get_metrics` is intentionally a no-op.
pub struct OllamaAdapter {
    client: reqwest::Client,
    endpoint: String,
}

impl OllamaAdapter {
    pub fn new(client: reqwest::Client, endpoint: String) -> Self {
        Self { client, endpoint }
    }
}

/// Subset of the `GET /api/ps` response we surface. Ollama returns the models
/// currently resident in memory; we display the first as the active model.
#[derive(Deserialize)]
struct PsResponse {
    #[serde(default)]
    models: Vec<PsModel>,
}

#[derive(Deserialize)]
struct PsModel {
    name: String,
    #[serde(default)]
    details: PsDetails,
}

#[derive(Deserialize, Default)]
struct PsDetails {
    /// Human-readable parameter count, e.g. "36.0B".
    parameter_size: Option<String>,
    /// Quantization label, e.g. "Q8_0".
    quantization_level: Option<String>,
    /// Model architecture family, e.g. "qwen35moe".
    family: Option<String>,
}

/// Map the first loaded model from an `/api/ps` body onto `ModelInfo`. Returns
/// `None` when nothing is loaded or the body does not parse — the dashboard
/// then shows the engine as running with no active model. Ollama already hands
/// back human-readable strings, so (unlike vLLM) no HuggingFace enrichment or
/// numeric formatting is needed.
fn model_info_from_ps(body: &str) -> Option<ModelInfo> {
    let resp: PsResponse = serde_json::from_str(body).ok()?;
    let m = resp.models.into_iter().next()?;
    Some(ModelInfo {
        name: m.name,
        parameter_size: m.details.parameter_size,
        quantization: m.details.quantization_level,
        precision: None,
        tensor_type: None,
        model_type: m.details.family,
        pipeline_tag: None,
    })
}

#[async_trait]
impl EngineAdapter for OllamaAdapter {
    fn engine_type(&self) -> EngineType {
        EngineType::Ollama
    }

    fn endpoint(&self) -> &str {
        &self.endpoint
    }

    async fn health_check(&self) -> EngineStatus {
        match self
            .client
            .get(format!("{}/api/version", self.endpoint))
            .timeout(Duration::from_secs(2))
            .send()
            .await
        {
            Ok(r) if r.status().is_success() => EngineStatus::Running,
            Ok(r) => EngineStatus::Error(format!("HTTP {}", r.status())),
            Err(e) => EngineStatus::Error(e.to_string()),
        }
    }

    async fn get_model_info(&self) -> Option<ModelInfo> {
        let body = self
            .client
            .get(format!("{}/api/ps", self.endpoint))
            .timeout(Duration::from_secs(2))
            .send()
            .await
            .ok()?
            .text()
            .await
            .ok()?;
        model_info_from_ps(&body)
    }

    /// Ollama exposes no Prometheus metrics endpoint, so there are no live
    /// throughput/latency signals to report. The frontend renders Ollama with
    /// a dedicated card that shows model identity instead of metric tiles.
    async fn get_metrics(&self) -> Option<EngineMetrics> {
        None
    }

    /// Ollama's loaded model changes at runtime as users switch models, and
    /// `/api/ps` is a cheap local call — so re-resolve every couple of poll
    /// ticks instead of caching for the default 10 minutes, so a model switch
    /// shows up on the dashboard within a few seconds.
    fn model_refresh_interval(&self) -> Duration {
        Duration::from_secs(2)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A representative `/api/ps` body (captured from Ollama 0.30.10) maps onto
    /// `ModelInfo` with Ollama's native strings passed through unchanged.
    #[test]
    fn ps_body_maps_to_model_info() {
        let body = r#"{"models":[{"name":"qwen3.5:35b-a3b-q8_0","model":"qwen3.5:35b-a3b-q8_0","size":41997691780,"digest":"655d","details":{"parent_model":"","format":"gguf","family":"qwen35moe","families":["qwen35moe"],"parameter_size":"36.0B","quantization_level":"Q8_0"},"expires_at":"2026-06-30T18:11:27+02:00","size_vram":41997691780,"context_length":262144}]}"#;
        let info = model_info_from_ps(body).expect("model info");
        assert_eq!(info.name, "qwen3.5:35b-a3b-q8_0");
        assert_eq!(info.parameter_size.as_deref(), Some("36.0B"));
        assert_eq!(info.quantization.as_deref(), Some("Q8_0"));
        assert_eq!(info.model_type.as_deref(), Some("qwen35moe"));
        // Ollama reports no safetensors-derived precision / tensor dtype.
        assert!(info.precision.is_none());
        assert!(info.tensor_type.is_none());
        assert!(info.pipeline_tag.is_none());
    }

    /// No model loaded (idle Ollama) yields `None`, so the engine shows running
    /// with no active model rather than a phantom entry.
    #[test]
    fn empty_ps_body_yields_no_model() {
        assert!(model_info_from_ps(r#"{"models":[]}"#).is_none());
    }

    /// Missing `details` fields are optional — the model still resolves with the
    /// absent attributes left as `None`.
    #[test]
    fn first_loaded_model_wins_and_details_are_optional() {
        let body =
            r#"{"models":[{"name":"llama3:8b","details":{}},{"name":"gemma:2b","details":{}}]}"#;
        let info = model_info_from_ps(body).expect("model info");
        assert_eq!(info.name, "llama3:8b");
        assert!(info.parameter_size.is_none());
        assert!(info.quantization.is_none());
        assert!(info.model_type.is_none());
    }

    /// Malformed or unexpected bodies never panic — they collapse to `None`.
    #[test]
    fn malformed_body_yields_no_model() {
        assert!(model_info_from_ps("not json").is_none());
        assert!(model_info_from_ps("{}").is_none());
    }

    /// Ollama's model identity is dynamic, so its refresh interval must be far
    /// shorter than the collector's default so a model switch surfaces quickly.
    #[test]
    fn model_refresh_interval_is_short() {
        let adapter = OllamaAdapter::new(reqwest::Client::new(), "http://localhost:11434".into());
        assert!(adapter.model_refresh_interval() <= Duration::from_secs(5));
    }
}
