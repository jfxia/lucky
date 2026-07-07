use std::collections::HashMap;

pub mod tls;
pub mod deepseek;
pub mod openai;
pub mod ollama;

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
        self.routes.get(model_name).map(|b| b.as_ref())
    }

    pub fn list_models(&self) -> Vec<String> {
        let mut models: Vec<String> = self.routes.keys().cloned().collect();
        models.sort();
        models
    }

    pub fn has_model(&self, name: &str) -> bool {
        self.routes.contains_key(name)
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

pub fn create_default_router() -> BackendRouter {
    let mut router = BackendRouter::new();

    router.register("deepseek-v4", Box::new(deepseek::DeepSeekBackend::new(None, None)));
    router.register("deepseek-chat", Box::new(deepseek::DeepSeekBackend::new(None, None)));
    router.register("DeepSeek", Box::new(deepseek::DeepSeekBackend::new(None, None)));

    router.register("gpt-4o", Box::new(openai::OpenAiBackend::new(None, None)));
    router.register("gpt-4", Box::new(openai::OpenAiBackend::new(None, None)));
    router.register("gpt-3.5-turbo", Box::new(openai::OpenAiBackend::new(None, None)));
    router.register("GPT", Box::new(openai::OpenAiBackend::new(None, None)));

    router.register("llama3", Box::new(ollama::OllamaBackend::new(None, None)));
    router.register("llama3.1", Box::new(ollama::OllamaBackend::new(None, None)));
    router.register("ollama", Box::new(ollama::OllamaBackend::new(None, None)));
    router.register("Ollama", Box::new(ollama::OllamaBackend::new(None, None)));

    router
}

pub fn load_router_from_manifest(models: &HashMap<String, ModelConfig>) -> BackendRouter {
    let mut router = BackendRouter::new();

    for (model_name, config) in models {
        match config.provider.as_str() {
            "deepseek" => {
                router.register(
                    model_name,
                    Box::new(deepseek::DeepSeekBackend::new(
                        config.endpoint.clone(),
                        config.api_key.clone(),
                    )),
                );
            }
            "openai" => {
                router.register(
                    model_name,
                    Box::new(openai::OpenAiBackend::new(
                        config.endpoint.clone(),
                        config.api_key.clone(),
                    )),
                );
            }
            "ollama" => {
                router.register(
                    model_name,
                    Box::new(ollama::OllamaBackend::new(
                        config.endpoint.clone(),
                        config.api_key.clone(),
                    )),
                );
            }
            other => {
                eprintln!("Warning: unknown provider '{}' for model '{}', skipping", other, model_name);
            }
        }
    }

    if router.routes.is_empty() {
        return create_default_router();
    }

    router
}
