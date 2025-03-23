mod client;
mod huggingface;
mod ollama;
mod openai;

pub use client::EmbeddingClient;
#[allow(unused_imports)]
pub use huggingface::HuggingFaceEmbeddingClient;
#[allow(unused_imports)]
pub use ollama::OllamaEmbeddingClient;
#[allow(unused_imports)]
pub use openai::OpenAIEmbeddingClient;

pub type Embedding = Vec<f32>;

#[derive(Debug, Clone)]
pub enum EmbeddingClientImpl {
    Ollama(ollama::OllamaEmbeddingClient),
    OpenAI(openai::OpenAIEmbeddingClient),
    HuggingFace(huggingface::HuggingFaceEmbeddingClient),
}
