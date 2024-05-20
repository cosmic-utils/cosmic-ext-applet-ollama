use std::sync::Arc;

#[derive(Debug, Clone, PartialEq)]
pub enum Models {
    NoModel,
    Llama3,
    Llama370b,
    Phi3,
    Mistral,
    NeuralChat,
    Starling,
    CodeLlama,
    Llama2Uncensored,
    LlaVa,
    Gemma,
    Gemma7b,
    Solar,
}

impl AsRef<str> for Models {
    fn as_ref(&self) -> &str {
        match self {
            Models::NoModel => "None",
            Models::Llama3 => "Llama3",
            Models::Llama370b => "Llam3 70B",
            Models::Phi3 => "Phi 3",
            Models::Mistral => "Mistral",
            Models::NeuralChat => "Neural Chat",
            Models::Starling => "Starling",
            Models::CodeLlama => "Code Llama",
            Models::Llama2Uncensored => "Llama2 Uncensored",
            Models::LlaVa => "LlaVa",
            Models::Gemma => "Gemma",
            Models::Gemma7b => "Gemma 7B",
            Models::Solar => "Solar",
        }
    }
}

impl std::fmt::Display for Models {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Models::NoModel => write!(f, "none"),
            Models::Llama3 => write!(f, "llama3"),
            Models::Llama370b => write!(f, "llama3:70b"),
            Models::Phi3 => write!(f, "phi3"),
            Models::Mistral => write!(f, "mistral"),
            Models::NeuralChat => write!(f, "neural-chat"),
            Models::Starling => write!(f, "starling-lm"),
            Models::CodeLlama => write!(f, "codellama"),
            Models::Llama2Uncensored => write!(f, "llama2-uncensored"),
            Models::LlaVa => write!(f, "llava"),
            Models::Gemma => write!(f, "gemma:2b"),
            Models::Gemma7b => write!(f, "gemma:7b"),
            Models::Solar => write!(f, "solar"),
        }
    }
}

pub fn is_installed(model: &Arc<Models>) -> bool {
    if let Ok(output) = std::process::Command::new("ollama").arg("list").output() {
        let output = String::from_utf8_lossy(&output.stdout);

        return output.to_lowercase().contains(&model.to_string());
    }

    false
}
