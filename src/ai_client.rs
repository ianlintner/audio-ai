/// AI client module for OpenAI integration with support for mocking/stubbing
use crate::audio_analysis::AnalysisResult;
use crate::comparison::ComparisonMetrics;
use anyhow::Result;
use serde_json::json;

/// Default OpenAI model - can be overridden with OPENAI_MODEL env var
pub const DEFAULT_OPENAI_MODEL: &str = "gpt-4o-mini";

/// Response from AI analysis
#[derive(Debug, Clone)]
pub struct AIFeedback {
    pub content: String,
}

/// Trait for AI client to enable testing with mocks
pub trait AIClient: Send + Sync {
    /// Send comparison results to AI for feedback
    fn send_comparison(
        &self,
        metrics: &ComparisonMetrics,
        reference_path: &str,
        player_path: &str,
    ) -> impl std::future::Future<Output = Result<AIFeedback>> + Send;

    /// Send single file analysis to AI
    fn send_single_analysis(
        &self,
        analysis: &AnalysisResult,
        file_path: &str,
    ) -> impl std::future::Future<Output = Result<AIFeedback>> + Send;
}

/// Production OpenAI client implementation
pub struct OpenAIClient {
    api_key: String,
    model: String,
    client: reqwest::Client,
}

impl OpenAIClient {
    pub fn new() -> Result<Self> {
        let api_key = std::env::var("OPENAI_API_KEY")
            .map_err(|_| anyhow::anyhow!("OPENAI_API_KEY environment variable not set"))?;
        let model = std::env::var("OPENAI_MODEL").unwrap_or_else(|_| DEFAULT_OPENAI_MODEL.to_string());
        let client = reqwest::Client::new();

        Ok(Self {
            api_key,
            model,
            client,
        })
    }

    async fn call_openai(&self, system_prompt: &str, user_prompt: &str) -> Result<AIFeedback> {
        let body = json!({
            "model": self.model,
            "messages": [
                {"role": "system", "content": system_prompt},
                {"role": "user", "content": user_prompt}
            ]
        });

        let res = self
            .client
            .post("https://api.openai.com/v1/chat/completions")
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await?;

        let json: serde_json::Value = res.json().await?;
        let content = json["choices"][0]["message"]["content"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Failed to extract AI response"))?
            .to_string();

        Ok(AIFeedback { content })
    }
}

impl AIClient for OpenAIClient {
    async fn send_comparison(
        &self,
        metrics: &ComparisonMetrics,
        reference_path: &str,
        player_path: &str,
    ) -> Result<AIFeedback> {
        let prompt = format!(
            "I'm comparing a student's guitar performance to a reference recording.\n\n\
            Reference: {}\n\
            Student: {}\n\n\
            Performance Metrics:\n\
            - Overall Similarity: {:.1}%\n\
            - Note Accuracy: {:.1}%\n\
            - Pitch Accuracy: {:.1}%\n\
            - Timing Accuracy: {:.1}%\n\
            - Rhythm Accuracy: {:.1}%\n\n\
            Errors Found:\n\
            - Missed Notes: {}\n\
            - Extra Notes: {}\n\
            - Pitch Errors: {} instances\n\
            - Timing Errors: {} instances\n\n\
            Please provide constructive feedback focusing on:\n\
            1. What the student did well\n\
            2. Specific areas for improvement\n\
            3. Practice suggestions\n\
            4. Overall assessment",
            reference_path,
            player_path,
            metrics.overall_similarity * 100.0,
            metrics.note_accuracy * 100.0,
            metrics.pitch_accuracy * 100.0,
            metrics.timing_accuracy * 100.0,
            metrics.rhythm_accuracy * 100.0,
            metrics.missed_notes.len(),
            metrics.extra_notes.len(),
            metrics.pitch_errors.len(),
            metrics.timing_errors.len()
        );

        let system_prompt = "You are an expert guitar teacher providing constructive feedback to students. Be specific, encouraging, and helpful.";

        self.call_openai(system_prompt, &prompt).await
    }

    async fn send_single_analysis(
        &self,
        analysis: &AnalysisResult,
        file_path: &str,
    ) -> Result<AIFeedback> {
        use crate::comparison::extract_note_sequence;

        let note_seq = extract_note_sequence(analysis);
        let detected_pitch = format!("{:.2} Hz", analysis.pitch_hz.first().unwrap_or(&0.0));
        let detected_tempo = analysis
            .tempo_bpm
            .map(|t| format!("{:.1} bpm", t))
            .unwrap_or("N/A".to_string());
        let detected_onsets = analysis.onsets.len();

        let prompt = format!(
            "Analyze this guitar recording. Provide feedback on timing, accuracy, and tone.\n\n\
            Features extracted:\n\
            - First detected pitch: {}\n\
            - Tempo: {}\n\
            - Number of onsets: {}\n\
            - Detected {} distinct notes: {:?}\n\n\
            File: {}",
            detected_pitch,
            detected_tempo,
            detected_onsets,
            note_seq.len(),
            note_seq
                .iter()
                .take(10)
                .map(|n| &n.note_name)
                .collect::<Vec<_>>(),
            file_path
        );

        let system_prompt = "You are a guitar teacher analyzing student recordings.";

        self.call_openai(system_prompt, &prompt).await
    }
}

/// Mock AI client for testing
pub struct MockAIClient {
    pub comparison_responses: Vec<String>,
    pub single_analysis_responses: Vec<String>,
    comparison_call_count: std::sync::Arc<std::sync::Mutex<usize>>,
    single_call_count: std::sync::Arc<std::sync::Mutex<usize>>,
}

impl MockAIClient {
    pub fn new() -> Self {
        Self {
            comparison_responses: vec![
                "Great job! Your performance shows excellent progress. Keep practicing the timing on measures 3-4.".to_string(),
            ],
            single_analysis_responses: vec![
                "Nice playing! The notes are clear and the tempo is consistent. Work on your vibrato technique.".to_string(),
            ],
            comparison_call_count: std::sync::Arc::new(std::sync::Mutex::new(0)),
            single_call_count: std::sync::Arc::new(std::sync::Mutex::new(0)),
        }
    }

    pub fn with_comparison_response(mut self, response: String) -> Self {
        self.comparison_responses = vec![response];
        self
    }

    pub fn with_single_response(mut self, response: String) -> Self {
        self.single_analysis_responses = vec![response];
        self
    }

    pub fn comparison_call_count(&self) -> usize {
        *self.comparison_call_count.lock().unwrap()
    }

    pub fn single_call_count(&self) -> usize {
        *self.single_call_count.lock().unwrap()
    }
}

impl AIClient for MockAIClient {
    async fn send_comparison(
        &self,
        _metrics: &ComparisonMetrics,
        _reference_path: &str,
        _player_path: &str,
    ) -> Result<AIFeedback> {
        let mut count = self.comparison_call_count.lock().unwrap();
        let index = *count % self.comparison_responses.len();
        *count += 1;
        
        Ok(AIFeedback {
            content: self.comparison_responses[index].clone(),
        })
    }

    async fn send_single_analysis(
        &self,
        _analysis: &AnalysisResult,
        _file_path: &str,
    ) -> Result<AIFeedback> {
        let mut count = self.single_call_count.lock().unwrap();
        let index = *count % self.single_analysis_responses.len();
        *count += 1;

        Ok(AIFeedback {
            content: self.single_analysis_responses[index].clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::comparison::ComparisonMetrics;

    #[tokio::test]
    async fn test_mock_client_comparison() {
        let mock = MockAIClient::new()
            .with_comparison_response("Test feedback for comparison".to_string());

        let metrics = ComparisonMetrics {
            overall_similarity: 0.85,
            note_accuracy: 0.90,
            pitch_accuracy: 0.80,
            timing_accuracy: 0.85,
            rhythm_accuracy: 0.88,
            missed_notes: vec![],
            extra_notes: vec![],
            pitch_errors: vec![],
            timing_errors: vec![],
        };

        let result = mock
            .send_comparison(&metrics, "ref.wav", "player.wav")
            .await
            .unwrap();

        assert_eq!(result.content, "Test feedback for comparison");
        assert_eq!(mock.comparison_call_count(), 1);
    }

    #[tokio::test]
    async fn test_mock_client_single_analysis() {
        let mock = MockAIClient::new()
            .with_single_response("Test feedback for single file".to_string());

        let analysis = AnalysisResult {
            pitch_hz: vec![440.0, 440.0],
            tempo_bpm: Some(120.0),
            onsets: vec![0.0, 0.5],
            spectral_centroid: vec![1000.0, 1000.0],
            streaming: None,
        };

        let result = mock
            .send_single_analysis(&analysis, "test.wav")
            .await
            .unwrap();

        assert_eq!(result.content, "Test feedback for single file");
        assert_eq!(mock.single_call_count(), 1);
    }
}
