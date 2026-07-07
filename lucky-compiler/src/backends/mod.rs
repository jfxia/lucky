use std::collections::HashMap;

pub mod tls;
pub mod http;
pub mod openai_compat;
pub mod anthropic;
pub mod deepseek;
pub mod openai;
pub mod ollama;

/// Internal provider keys (not user-facing) — one per supported provider.
const PROVIDER_DEEPSEEK: &str = "__lucky_provider_deepseek__";
const PROVIDER_OPENAI: &str = "__lucky_provider_openai__";
const PROVIDER_KIMI: &str = "__lucky_provider_kimi__";
const PROVIDER_QWEN: &str = "__lucky_provider_qwen__";
const PROVIDER_DOUBAO: &str = "__lucky_provider_doubao__";
const PROVIDER_GLM: &str = "__lucky_provider_glm__";
const PROVIDER_GOOGLE: &str = "__lucky_provider_google__";
const PROVIDER_ANTHROPIC: &str = "__lucky_provider_anthropic__";
const PROVIDER_OLLAMA: &str = "__lucky_provider_ollama__";

pub struct CompleteOptions {
    pub temperature: f64,
    pub max_tokens: u32,
    pub stop_sequences: Vec<String>,
    pub system_prompt: Option<String>,
}

impl Default for CompleteOptions {
    fn default() -> Self {
        Self {
            temperature: 0.7,
            max_tokens: 4096,
            stop_sequences: Vec::new(),
            system_prompt: None,
        }
    }
}

impl CompleteOptions {
    pub fn with_temperature(mut self, t: f64) -> Self {
        self.temperature = t;
        self
    }

    pub fn with_max_tokens(mut self, n: u32) -> Self {
        self.max_tokens = n;
        self
    }

    pub fn with_system_prompt(mut self, p: impl Into<String>) -> Self {
        self.system_prompt = Some(p.into());
        self
    }
}

pub trait Backend {
    fn name(&self) -> &'static str;
    fn complete(&self, prompt: &str, options: &CompleteOptions) -> Result<String, String>;
    fn complete_stream(
        &self, prompt: &str, options: &CompleteOptions,
    ) -> Result<Box<dyn FnMut() -> Option<String>>, String>;
    fn health_check(&self) -> bool;
    fn cost_per_1k_tokens(&self) -> f64;
}

pub struct BackendRouter {
    routes: HashMap<String, Box<dyn Backend>>,
}

impl BackendRouter {
    pub fn new() -> Self {
        Self { routes: HashMap::new() }
    }

    pub fn register(&mut self, model_name: &str, backend: Box<dyn Backend>) {
        self.routes.insert(model_name.to_string(), backend);
    }

    pub fn route(&self, model_name: &str) -> Option<&dyn Backend> {
        if let Some(backend) = self.routes.get(model_name) {
            return Some(backend.as_ref());
        }
        if let Some(provider_key) = guess_provider(model_name) {
            if let Some(backend) = self.routes.get(provider_key) {
                return Some(backend.as_ref());
            }
        }
        self.routes.values().next().map(|b| b.as_ref())
    }

    pub fn list_models(&self) -> Vec<String> {
        let mut models: Vec<String> = self.routes.keys().cloned().collect();
        models.retain(|k| !k.starts_with("__lucky_provider_"));
        models.sort();
        if models.is_empty() {
            models.push("deepseek (default)".to_string());
            models.push("openai (default)".to_string());
            models.push("kimi (default)".to_string());
            models.push("qwen (default)".to_string());
            models.push("doubao (default)".to_string());
            models.push("glm (default)".to_string());
            models.push("google (default)".to_string());
            models.push("anthropic (default)".to_string());
            models.push("ollama (default)".to_string());
        }
        models
    }

    pub fn has_model(&self, name: &str) -> bool {
        self.routes.contains_key(name) || guess_provider(name).is_some()
    }
}

impl Default for BackendRouter {
    fn default() -> Self {
        Self::new()
    }
}

pub struct ModelConfig {
    pub model_name: String,
    pub provider: String,
    pub api_key: Option<String>,
    pub endpoint: Option<String>,
    pub temperature: f64,
    pub max_tokens: u32,
}

/// Guess the LLM provider from a model name string.
/// No hardcoded model versions — matches by pattern so new model names
/// like "gpt-5" or "claude-4" work without code changes.
fn guess_provider(model_name: &str) -> Option<&'static str> {
    let lower = model_name.to_lowercase();
    if lower.contains("deepseek") {
        Some(PROVIDER_DEEPSEEK)
    } else if lower.contains("claude") || lower.contains("anthropic") {
        Some(PROVIDER_ANTHROPIC)
    } else if lower.contains("gpt") || lower.contains("openai") || lower.contains("o1") || lower.contains("o3") {
        Some(PROVIDER_OPENAI)
    } else if lower.contains("kimi") || lower.contains("moonshot") {
        Some(PROVIDER_KIMI)
    } else if lower.contains("qwen") || lower.contains("dashscope") {
        Some(PROVIDER_QWEN)
    } else if lower.contains("doubao") || lower.contains("volc") || lower.contains("ark") {
        Some(PROVIDER_DOUBAO)
    } else if lower.contains("glm") || lower.contains("zhipu") || lower.contains("chatglm") {
        Some(PROVIDER_GLM)
    } else if lower.contains("gemini") || lower.contains("google") {
        Some(PROVIDER_GOOGLE)
    } else if lower.contains("llama") || lower.contains("ollama") || lower.contains("mistral")
        || lower.contains("qwen") || lower.contains("gemma") || lower.contains("phi")
    {
        Some(PROVIDER_OLLAMA)
    } else {
        Some(PROVIDER_DEEPSEEK)
    }
}

fn make_openai_compat_backend(
    config: &'static openai_compat::OpenAiCompatConfig,
    endpoint: Option<String>, api_key: Option<String>,
) -> Box<dyn Backend> {
    Box::new(openai_compat::OpenAiCompatBackend::new(config, endpoint, api_key))
}

/// Create a default router with generic backends for all supported providers.
/// No hardcoded model version names — `route()` uses `guess_provider()` to
/// match any model name like "gpt-5" or "claude-4" or "kimi-latest".
pub fn create_default_router() -> BackendRouter {
    let mut router = BackendRouter::new();
    router.register(PROVIDER_DEEPSEEK, make_openai_compat_backend(&openai_compat::DEEPSEEK_CONFIG, None, None));
    router.register(PROVIDER_OPENAI, make_openai_compat_backend(&openai_compat::OPENAI_CONFIG, None, None));
    router.register(PROVIDER_KIMI, make_openai_compat_backend(&openai_compat::KIMI_CONFIG, None, None));
    router.register(PROVIDER_QWEN, make_openai_compat_backend(&openai_compat::QWEN_CONFIG, None, None));
    router.register(PROVIDER_DOUBAO, make_openai_compat_backend(&openai_compat::DOUBAO_CONFIG, None, None));
    router.register(PROVIDER_GLM, make_openai_compat_backend(&openai_compat::GLM_CONFIG, None, None));
    router.register(PROVIDER_GOOGLE, make_openai_compat_backend(&openai_compat::GOOGLE_CONFIG, None, None));
    router.register(PROVIDER_ANTHROPIC, Box::new(anthropic::AnthropicBackend::new(None, None)));
    router.register(PROVIDER_OLLAMA, Box::new(ollama::OllamaBackend::new(None, None)));
    router
}

pub fn load_router_from_manifest(models: &HashMap<String, ModelConfig>) -> BackendRouter {
    let mut has_deepseek = false;
    let mut has_openai = false;
    let mut has_kimi = false;
    let mut has_qwen = false;
    let mut has_doubao = false;
    let mut has_glm = false;
    let mut has_google = false;
    let mut has_anthropic = false;
    let mut has_ollama = false;
    let mut router = BackendRouter::new();

    for (model_name, config) in models {
        match config.provider.as_str() {
            "deepseek" => { has_deepseek = true;
                router.register(model_name, make_openai_compat_backend(&openai_compat::DEEPSEEK_CONFIG, config.endpoint.clone(), config.api_key.clone())); }
            "openai" => { has_openai = true;
                router.register(model_name, make_openai_compat_backend(&openai_compat::OPENAI_CONFIG, config.endpoint.clone(), config.api_key.clone())); }
            "kimi" => { has_kimi = true;
                router.register(model_name, make_openai_compat_backend(&openai_compat::KIMI_CONFIG, config.endpoint.clone(), config.api_key.clone())); }
            "qwen" => { has_qwen = true;
                router.register(model_name, make_openai_compat_backend(&openai_compat::QWEN_CONFIG, config.endpoint.clone(), config.api_key.clone())); }
            "doubao" => { has_doubao = true;
                router.register(model_name, make_openai_compat_backend(&openai_compat::DOUBAO_CONFIG, config.endpoint.clone(), config.api_key.clone())); }
            "glm" => { has_glm = true;
                router.register(model_name, make_openai_compat_backend(&openai_compat::GLM_CONFIG, config.endpoint.clone(), config.api_key.clone())); }
            "google" => { has_google = true;
                router.register(model_name, make_openai_compat_backend(&openai_compat::GOOGLE_CONFIG, config.endpoint.clone(), config.api_key.clone())); }
            "anthropic" => { has_anthropic = true;
                router.register(model_name, Box::new(anthropic::AnthropicBackend::new(config.endpoint.clone(), config.api_key.clone()))); }
            "ollama" => { has_ollama = true;
                router.register(model_name, Box::new(ollama::OllamaBackend::new(config.endpoint.clone(), config.api_key.clone()))); }
            other => {
                eprintln!("Warning: unknown provider '{}' for model '{}', skipping", other, model_name);
            }
        }
    }

    // Register generic provider backends so that route() can match
    // unregistered model names via the heuristic (e.g. "gpt-5" → openai)
    if has_deepseek { router.register(PROVIDER_DEEPSEEK, make_openai_compat_backend(&openai_compat::DEEPSEEK_CONFIG, None, None)); }
    if has_openai { router.register(PROVIDER_OPENAI, make_openai_compat_backend(&openai_compat::OPENAI_CONFIG, None, None)); }
    if has_kimi { router.register(PROVIDER_KIMI, make_openai_compat_backend(&openai_compat::KIMI_CONFIG, None, None)); }
    if has_qwen { router.register(PROVIDER_QWEN, make_openai_compat_backend(&openai_compat::QWEN_CONFIG, None, None)); }
    if has_doubao { router.register(PROVIDER_DOUBAO, make_openai_compat_backend(&openai_compat::DOUBAO_CONFIG, None, None)); }
    if has_glm { router.register(PROVIDER_GLM, make_openai_compat_backend(&openai_compat::GLM_CONFIG, None, None)); }
    if has_google { router.register(PROVIDER_GOOGLE, make_openai_compat_backend(&openai_compat::GOOGLE_CONFIG, None, None)); }
    if has_anthropic { router.register(PROVIDER_ANTHROPIC, Box::new(anthropic::AnthropicBackend::new(None, None))); }
    if has_ollama { router.register(PROVIDER_OLLAMA, Box::new(ollama::OllamaBackend::new(None, None))); }

    if router.routes.is_empty() {
        return create_default_router();
    }

    router
}
