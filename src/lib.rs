mod api;
mod chat;
mod localize;
mod markdown;
mod models;
mod stream;
mod window;

use cosmic::widget;
use cosmic_text::{FontSystem, SwashCache, SyntaxSystem};
use ron::de::from_reader;
use ron::ser::{to_string_pretty, PrettyConfig};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufReader, Write};
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::{fs, io};
use window::Window;

static FONT_SYSTEM: OnceLock<Mutex<FontSystem>> = OnceLock::new();
static SWASH_CACHE: OnceLock<Mutex<SwashCache>> = OnceLock::new();
static SYNTAX_SYSTEM: OnceLock<SyntaxSystem> = OnceLock::new();

pub fn run() -> cosmic::iced::Result {
    localize::localize();

    FONT_SYSTEM.get_or_init(|| Mutex::new(FontSystem::new()));
    SWASH_CACHE.get_or_init(|| Mutex::new(SwashCache::new()));
    SYNTAX_SYSTEM.get_or_init(|| {
        let lazy_theme_set = two_face::theme::LazyThemeSet::from(two_face::theme::extra());
        let mut theme_set = syntect::highlighting::ThemeSet::from(&lazy_theme_set);
        // Hardcoded COSMIC themes
        for (theme_name, theme_data) in &[
            ("COSMIC Dark", cosmic_syntax_theme::COSMIC_DARK_TM_THEME),
            ("COSMIC Light", cosmic_syntax_theme::COSMIC_LIGHT_TM_THEME),
        ] {
            let mut cursor = io::Cursor::new(theme_data);
            match syntect::highlighting::ThemeSet::load_from_reader(&mut cursor) {
                Ok(mut theme) => {
                    // Use libcosmic theme for background and gutter
                    theme.settings.background = Some(syntect::highlighting::Color {
                        r: 0,
                        g: 0,
                        b: 0,
                        a: 0,
                    });
                    theme.settings.gutter = Some(syntect::highlighting::Color {
                        r: 0,
                        g: 0,
                        b: 0,
                        a: 0,
                    });
                    theme_set.themes.insert(theme_name.to_string(), theme);
                }
                Err(err) => {
                    eprintln!("failed to load {:?} syntax theme: {}", theme_name, err);
                }
            }
        }
        SyntaxSystem {
            //TODO: store newlines in buffer
            syntax_set: two_face::syntax::extra_no_newlines(),
            theme_set,
        }
    });

    cosmic::applet::run::<Window>(false, ())
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Settings {
    avatar: PathBuf,
    keep_context: bool,
    model: String,
    ollama_address: String,
}

impl Settings {
    pub fn new() -> Self {
        Self {
            avatar: PathBuf::new(),
            keep_context: true,
            model: String::new(),
            ollama_address: "localhost:11434".to_string(),
        }
    }

    pub fn get_avatar_handle(&self) -> widget::image::Handle {
        if self.avatar.exists() {
            widget::image::Handle::from_path(&self.avatar)
        } else {
            let image: &[u8] = include_bytes!("../data/icons/avatar.png");
            widget::image::Handle::from_memory(image)
        }
    }

    pub fn set_avatar(&mut self, path: PathBuf) -> &mut Self {
        self.avatar = path;
        self
    }

    pub fn change_context(&mut self, context: bool) -> &mut Self {
        self.keep_context = context;
        self
    }

    pub fn set_model(&mut self, model: String) -> &mut Self {
        self.model = model;
        self
    }

    pub fn set_ollama_address(&mut self, ip_port: String) -> &mut Self {
        self.ollama_address = ip_port;
        self
    }

    pub fn load() -> Settings {
        let data_path = dirs::data_dir()
            .expect("xdg-data not found")
            .join("cosmic-ext-applet-ollama")
            .join("settings.ron");

        let settings: Settings = Settings::new();

        if let Ok(opened) = File::open(data_path) {
            let reader = BufReader::new(opened);
            let settings: Settings = from_reader(reader).expect("Cannot parse settings file");
            return settings;
        }
        settings
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let data_path = dirs::data_dir()
            .expect("xdg-data not found")
            .join("cosmic-ext-applet-ollama");

        fs::create_dir_all(&data_path)?;

        let pretty = PrettyConfig::default();
        let ron_string = to_string_pretty(self, pretty).unwrap();

        let mut file = File::create(data_path.join("settings.ron"))?;
        file.write_all(ron_string.as_bytes())?;

        Ok(())
    }
}
