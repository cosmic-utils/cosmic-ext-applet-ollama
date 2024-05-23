use chrono::Local;
use ron::ser::{to_string_pretty, PrettyConfig};
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
