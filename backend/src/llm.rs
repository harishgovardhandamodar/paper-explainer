//! Optional AI enhancement layer.
//!
//! The deterministic extractor is the source of truth. This module asks an
//! Ollama model (through the fox-services LLM gateway) for a friendlier,
//! narrative explanation and stores it alongside the paper. Everything here
//! is fail-soft: if the gateway is down, the rest of the app keeps working.

use crate::error::{AppError, AppResult};
use crate::models::{LlmStatus, Paper, PaperEnhancement};
use chrono::Utc;
use serde::{Deserialize, Serialize};

/// fox-services publishes the Ollama-compatible gateway on host port 8210.
/// In a bridge-network deployment use `http://fox-services:8210` instead.
const DEFAULT_BASE_URL: &str = "http://localhost:8210";
const DEFAULT_MODEL: &str = "qwen3.8:27b";
const DEFAULT_TIMEOUT_SECS: u64 = 300;

#[derive(Debug, Clone)]
pub struct LlmConfig {
    pub enabled: bool,
    pub base_url: String,
    pub model: String,
    pub timeout_secs: u64,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            base_url: DEFAULT_BASE_URL.into(),
            model: DEFAULT_MODEL.into(),
            timeout_secs: DEFAULT_TIMEOUT_SECS,
        }
    }
}

impl LlmConfig {
    pub fn from_env() -> Self {
        let mut c = Self::default();
        if let Ok(v) = std::env::var("PAPER_EXPLAINER_OLLAMA_ENABLED") {
            c.enabled = !matches!(v.trim().to_lowercase().as_str(), "0" | "false" | "no" | "off");
        }
        if let Ok(v) = std::env::var("PAPER_EXPLAINER_OLLAMA_URL") {
            let v = v.trim().trim_end_matches('/').to_string();
            if !v.is_empty() { c.base_url = v; }
        }
        if let Ok(v) = std::env::var("PAPER_EXPLAINER_OLLAMA_MODEL") {
            let v = v.trim().to_string();
            if !v.is_empty() { c.model = v; }
        }
        if let Ok(v) = std::env::var("PAPER_EXPLAINER_OLLAMA_TIMEOUT") {
            if let Ok(n) = v.trim().parse::<u64>() {
                if n >= 10 { c.timeout_secs = n; }
            }
        }
        c
    }
}

#[derive(Debug, Clone)]
pub struct LlmClient {
    pub config: LlmConfig,
    http: reqwest::Client,
    probe_http: reqwest::Client,
}

impl LlmClient {
    pub fn new(config: LlmConfig) -> AppResult<Self> {
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_secs))
            .build()
            .map_err(|e| AppError::Llm(format!("cannot build HTTP client: {e}")))?;
        let probe_http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(8))
            .build()
            .map_err(|e| AppError::Llm(format!("cannot build probe client: {e}")))?;
        Ok(Self { config, http, probe_http })
    }

    pub fn from_env() -> AppResult<Self> { Self::new(LlmConfig::from_env()) }

    /// Probe the gateway's `/api/tags` endpoint. Never fails hard: an
    /// unreachable gateway is reported as a status, not an HTTP error.
    pub async fn status(&self) -> LlmStatus {
        let base = self.config.base_url.clone();
        let model = self.config.model.clone();
        let mut s = LlmStatus {
            enabled: self.config.enabled,
            reachable: false,
            base_url: base.clone(),
            model: model.clone(),
            model_available: false,
            models: Vec::new(),
            message: String::new(),
        };

        if !self.config.enabled {
            s.message = "AI enhancement is disabled (PAPER_EXPLAINER_OLLAMA_ENABLED=0)".into();
            return s;
        }

        match self.probe_http.get(format!("{base}/api/tags")).send().await {
            Err(e) => {
                s.message = format!("gateway unreachable at {base}: {e}");
                s
            }
            Ok(resp) if !resp.status().is_success() => {
                s.message = format!("gateway returned HTTP {}", resp.status());
                s
            }
            Ok(resp) => match resp.json::<OllamaTags>().await {
                Err(e) => {
                    s.message = format!("gateway returned invalid /api/tags: {e}");
                    s
                }
                Ok(tags) => {
                    let mut names: Vec<String> = tags.models.into_iter().map(|m| m.name).collect();
                    names.sort();
                    names.dedup();
                    s.model_available = names.iter().any(|n| n == &model);
                    s.reachable = true;
                    s.models = names;
                    s.message = if s.model_available {
                        format!("Ready — {model} is available through the gateway")
                    } else {
                        format!("Gateway reachable, but {model} is not installed")
                    };
                    s
                }
            },
        }
    }

    /// Generate an enhancement for a paper. The model is asked for strict JSON
    /// so the result can be stored as structured data and rendered reliably.
    pub async fn enhance(&self, paper: &Paper) -> AppResult<PaperEnhancement> {
        if !self.config.enabled {
            return Err(AppError::Llm("AI enhancement is disabled".into()));
        }

        let (system, user) = build_prompt(paper);
        let req = ChatRequest {
            model: &self.config.model,
            stream: false,
            messages: vec![
                ChatMessage { role: "system", content: &system },
                ChatMessage { role: "user", content: &user },
            ],
            options: serde_json::json!({ "temperature": 0.2 }),
        };

        let url = format!("{}/api/chat", self.config.base_url);
        let resp = self.http.post(url).json(&req).send().await.map_err(|e| {
            AppError::Llm(format!(
                "could not reach Ollama gateway {} (model {}): {}",
                self.config.base_url, self.config.model, e
            ))
        })?;

        let status = resp.status();
        let body = resp.text().await.map_err(|e| AppError::Llm(format!("gateway response read failed: {e}")))?;
        if !status.is_success() {
            return Err(AppError::Llm(format!("gateway returned HTTP {status}: {}", truncate(&body, 300))));
        }

        let chat: ChatResponse = serde_json::from_str(&body)
            .map_err(|e| AppError::Llm(format!("invalid gateway response: {e}: {}", truncate(&body, 300))))?;
        let raw = match parse_enhancement_json(&chat.message.content) {
            Ok(raw) => raw,
            Err(e) => {
                tracing::warn!(
                    paper_id = %paper.id,
                    raw = %truncate(&chat.message.content, 12_000),
                    "enhancement JSON parse failed"
                );
                return Err(e);
            }
        };

        tracing::info!(
            paper_id = %paper.id,
            requested_model = %self.config.model,
            served_model = %chat.model.clone().unwrap_or_default(),
            "AI enhancement generated"
        );

        Ok(PaperEnhancement {
            tldr: clean(raw.tldr.unwrap_or_default()),
            explanation: clean(raw.explanation.unwrap_or_default()),
            how_it_works: clean(raw.how_it_works.unwrap_or_default()),
            key_insights: raw.key_insights.unwrap_or_default().iter().map(|s| clean(s.clone())).filter(|s| !s.is_empty()).collect(),
            caveats: raw.caveats.unwrap_or_default().iter().map(|s| clean(s.clone())).filter(|s| !s.is_empty()).collect(),
            requested_model: self.config.model.clone(),
            served_model: chat.model.unwrap_or_else(|| self.config.model.clone()),
            gateway: self.config.base_url.clone(),
            prompt_tokens: chat.prompt_eval_count,
            completion_tokens: chat.eval_count,
            generated_at: Utc::now(),
        })
    }
}

// ---------------------------------------------------------------------------
// Wire types
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct OllamaTags {
    #[serde(default)]
    models: Vec<OllamaTag>,
}

#[derive(Debug, Deserialize)]
struct OllamaTag {
    name: String,
}

#[derive(Debug, Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    stream: bool,
    messages: Vec<ChatMessage<'a>>,
    options: serde_json::Value,
}

#[derive(Debug, Serialize)]
struct ChatMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    model: Option<String>,
    message: ChatMessageOut,
    #[serde(default)]
    prompt_eval_count: Option<u64>,
    #[serde(default)]
    eval_count: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct ChatMessageOut {
    #[serde(default)]
    content: String,
}

#[derive(Debug, Deserialize, Default)]
struct RawEnhancement {
    tldr: Option<String>,
    explanation: Option<String>,
    how_it_works: Option<String>,
    key_insights: Option<Vec<String>>,
    caveats: Option<Vec<String>>,
}

// ---------------------------------------------------------------------------
// Prompt + response parsing
// ---------------------------------------------------------------------------

const SYSTEM_PROMPT: &str = "You are an expert research-paper explainer writing for a curious engineer.\n\
Be faithful to the supplied paper: never invent numbers, results, or claims.\n\
Explain in plain language, define jargon the first time you use it, and keep the reader's caveats honest.\n\
Return ONLY a single JSON object with exactly these string/list fields:\n\
{\"tldr\": \"one sentence\", \"explanation\": \"two to four short paragraphs\\n\\nseparated by blank lines\", \
\"how_it_works\": \"a short numbered or step-by-step explanation\", \
\"key_insights\": [\"3 to 6 concise takeaways\"], \"caveats\": [\"limitations or open questions\"]}\n\
Do not wrap the JSON in markdown fences. Do not use markdown headings. Keep each field concise.\n\
Use double-quoted strings with escaped characters only; never put a literal line break inside a string.\n\
Use \\n for paragraph breaks inside the explanation field.";

fn build_prompt(paper: &Paper) -> (String, String) {
    let clip = |s: &str, n: usize| -> String { truncate(s, n).to_string() };

    let mut facts = String::new();
    facts.push_str(&format!("Title: {}\n", clip(&paper.title, 300)));
    if !paper.authors.is_empty() {
        facts.push_str(&format!("Authors: {}\n", clip(&paper.authors.join(", "), 300)));
    }
    if let Some(v) = paper.venue.as_ref() { facts.push_str(&format!("Venue: {}\n", clip(v, 120))); }
    if let Some(y) = paper.year { facts.push_str(&format!("Year: {y}\n")); }
    facts.push_str(&format!("Abstract: {}\n", clip(&paper.abstract_text, 1400)));
    facts.push_str(&format!("Existing plain-language summary: {}\n", clip(&paper.summary, 1200)));

    if !paper.hypotheses.is_empty() {
        facts.push_str("\nHypotheses:\n");
        for h in &paper.hypotheses {
            facts.push_str(&format!("- {:?}: {} — {}\n", h.status, clip(&h.statement, 400), clip(&h.simplified, 400)));
        }
    }
    if !paper.experiments.is_empty() {
        facts.push_str("\nExperiments:\n");
        for e in &paper.experiments {
            facts.push_str(&format!("- {}: {} Methods: {}\n", clip(&e.name, 200), clip(&e.question, 400), clip(&e.design, 500)));
            for r in e.results.iter().take(3) { facts.push_str(&format!("  Result: {}\n", clip(r, 300))); }
        }
    }
    if !paper.findings.is_empty() {
        facts.push_str("\nFindings:\n");
        for f in &paper.findings {
            facts.push_str(&format!("- [{:?}] {} Evidence: {}", f.importance, clip(&f.claim, 350), clip(&f.evidence, 250)));
            if let Some(c) = f.caveat.as_ref() { facts.push_str(&format!(" Caveat: {}", clip(c, 200))); }
            facts.push('\n');
        }
    }
    if !paper.concepts.is_empty() {
        let names: Vec<String> = paper.concepts.iter().take(20).map(|c| c.name.clone()).collect();
        facts.push_str(&format!("\nKey concepts: {}\n", clip(&names.join(", "), 600)));
    }

    let user = format!(
        "Write an AI-enhanced explanation of this paper using the facts below.\n\n{facts}\n\
         Your answer: one JSON object with tldr, explanation, how_it_works, key_insights, caveats."
    );
    (SYSTEM_PROMPT.to_string(), user)
}

/// Parse a model reply that may be bare JSON or wrapped in ```json fences.
fn parse_enhancement_json(text: &str) -> AppResult<RawEnhancement> {
    let trimmed = text.trim();
    let candidate = if let Some(start) = trimmed.find('{') {
        let end = trimmed.rfind('}').unwrap_or(start);
        if end > start { &trimmed[start..=end] } else { trimmed }
    } else {
        trimmed
    };
    match serde_json::from_str::<RawEnhancement>(candidate) {
        Ok(raw) => Ok(raw),
        Err(strict_err) => {
            // Small models sometimes emit raw newlines inside strings or a
            // trailing comma. Repair those two cases before giving up.
            let repaired = repair_json(candidate);
            match serde_json::from_str::<RawEnhancement>(&repaired) {
                Ok(raw) => Ok(raw),
                Err(e) => Err(AppError::Llm(format!(
                    "model did not return the expected JSON (strict: {strict_err}; repaired: {e}); near: {}",
                    window(&repaired, e.column(), 140)
                ))),
            }
        }
    }
}

/// Repair the two mistakes small models make most often: raw control
/// characters inside JSON strings, and imperfect array/object closure
/// (a missing `]` before the final `}` is especially common).
fn repair_json(input: &str) -> String {
    let chars: Vec<char> = input.chars().collect();
    let mut out = String::with_capacity(input.len() + 16);
    let mut stack: Vec<char> = Vec::new();
    let mut in_string = false;
    let mut escaped = false;

    for (i, &c) in chars.iter().enumerate() {
        if in_string {
            if escaped {
                escaped = false;
                out.push(c);
                continue;
            }
            match c {
                '"' => { in_string = false; out.push('"'); }
                '\\' => { escaped = true; out.push('\\'); }
                '\n' => out.push_str("\\n"),
                '\r' => out.push_str("\\r"),
                '\t' => out.push_str("\\t"),
                c if c < '\u{20}' => out.push_str(&format!("\\u{:04x}", c as u32)),
                _ => out.push(c),
            }
            continue;
        }
        match c {
            '"' => { in_string = true; out.push('"'); }
            ',' => {
                let mut j = i + 1;
                while j < chars.len() && chars[j].is_whitespace() { j += 1; }
                if !(j < chars.len() && (chars[j] == '}' || chars[j] == ']')) {
                    out.push(',');
                }
            }
            '{' | '[' => { stack.push(c); out.push(c); }
            '}' | ']' => {
                let matches = |top: char, c: char| (top == '{' && c == '}') || (top == '[' && c == ']');
                if let Some(&top) = stack.last() {
                    if matches(top, c) {
                        stack.pop();
                        out.push(c);
                    } else {
                        // Insert the bracket the model forgot before this closer.
                        out.push(if top == '{' { '}' } else { ']' });
                        stack.pop();
                        out.push(c);
                        if let Some(&next) = stack.last() {
                            if matches(next, c) { stack.pop(); }
                        }
                    }
                } else {
                    // Stray closer (the model already closed this level) — drop it.
                }
            }
            _ => out.push(c),
        }
    }
    while let Some(open) = stack.pop() {
        out.push(if open == '{' { '}' } else { ']' });
    }
    out
}

fn window(s: &str, column: usize, radius: usize) -> String {
    let mut start = column.saturating_sub(radius);
    while start > 0 && !s.is_char_boundary(start) { start -= 1; }
    let mut end = (column + radius).min(s.len());
    while end < s.len() && !s.is_char_boundary(end) { end += 1; }
    let mut out = String::new();
    if start > 0 { out.push('…'); }
    out.push_str(&s[start..end]);
    if end < s.len() { out.push('…'); }
    out
}

fn truncate(s: &str, max: usize) -> &str {
    if s.len() <= max { return s; }
    let mut end = max;
    while end > 0 && !s.is_char_boundary(end) { end -= 1; }
    &s[..end]
}

fn clean(s: String) -> String {
    s.trim().trim_matches('"').trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_bare_json() {
        let raw = parse_enhancement_json(
            r#"{"tldr":"A short answer.","explanation":"Para one.\n\nPara two.","key_insights":["one","two"]}"#,
        ).unwrap();
        assert_eq!(raw.tldr.as_deref(), Some("A short answer."));
        assert_eq!(raw.key_insights.unwrap().len(), 2);
    }

    #[test]
    fn parses_fenced_json() {
        let raw = parse_enhancement_json(
            "```json\n{\"tldr\":\"Fenced.\",\"caveats\":[\"small sample\"]}\n```",
        ).unwrap();
        assert_eq!(raw.tldr.as_deref(), Some("Fenced."));
        assert_eq!(raw.caveats.unwrap()[0], "small sample");
    }

    #[test]
    fn repairs_raw_newlines_and_trailing_commas() {
        let messy = "{\"tldr\":\"line one\nline two\",\"key_insights\":[\"a\",\"b\",],}";
        let raw = parse_enhancement_json(messy).unwrap();
        assert_eq!(raw.tldr.as_deref(), Some("line one\nline two"));
        assert_eq!(raw.key_insights.unwrap().len(), 2);
    }

    #[test]
    fn repairs_missing_array_closer() {
        let messy = "{\"key_insights\":[\"a\",\"b\",\"c\"}}";
        let raw = parse_enhancement_json(messy).unwrap();
        assert_eq!(raw.key_insights.unwrap().len(), 3);
    }

    #[test]
    fn rejects_non_json() {
        assert!(parse_enhancement_json("Sorry, I cannot help with that.").is_err());
    }

    #[test]
    fn prompt_contains_paper_facts() {
        let mut paper = Paper::default();
        paper.title = "Security Boundary of Obfuscation".into();
        paper.summary = "A summary about TEE protection.".into();
        let (system, user) = build_prompt(&paper);
        assert!(system.contains("JSON"));
        assert!(user.contains("Security Boundary of Obfuscation"));
        assert!(user.contains("key_insights"));
    }
}
