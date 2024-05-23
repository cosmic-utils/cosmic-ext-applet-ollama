use chrono::Local;
use ron::{
    from_str,
    ser::{to_string_pretty, PrettyConfig},
};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File},
    io::Write,
};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Text {
    User(String),
    Bot(String),
}

#[derive(Serialize, Deserialize)]
pub struct Conversation {
    pub messages: Vec<Text>,
}

impl Conversation {
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
        }
    }

    pub fn push(&mut self, message: Text) -> &mut Self {
        self.messages.push(message);
        self
    }

    pub fn save_to_file(&self) -> anyhow::Result<()> {
        let data_path = dirs::data_dir()
            .expect("xdg-data not found")
            .join("cosmic-applet-ollama");

        fs::create_dir_all(&data_path)?;

        let now = Local::now();
        let formatted = now.format("%Y-%m-%d %H:%M:%S").to_string();
        let filename = format!("{}.ron", formatted);

        let pretty = PrettyConfig::default();
        let ron_string = to_string_pretty(self, pretty).unwrap();

        let mut file = File::create(data_path.join(filename))?;
        file.write_all(ron_string.as_bytes())?;

        Ok(())
    }
}

pub fn read_conversation_files() -> anyhow::Result<Vec<String>> {
    let data_path = dirs::data_dir()
        .expect("xdg-data not found")
        .join("cosmic-applet-ollama");

    let mut conversations = Vec::new();

    if let Ok(entries) = fs::read_dir(data_path) {
        for entry in entries.flatten() {
            conversations.push(
                entry
                    .path()
                    .file_stem()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .to_owned(),
            );
        }
    }

    Ok(conversations)
}

pub fn load_conversation(filename: String) -> Conversation {
    let data_path = dirs::data_dir()
        .expect("xdg-data not found")
        .join("cosmic-applet-ollama")
        .join(format!("{}.ron", filename));

    let contents = fs::read_to_string(data_path).unwrap();
    from_str(&contents).unwrap()
}
