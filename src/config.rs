use once_cell::sync::Lazy;
use std::env;

// Struct to hold our application settings
#[derive(Debug)]
pub struct Settings {
    pub openai_api_key: String,
}

impl Settings {
    // Load settings from environment variables
    fn load() -> Self {
        // Load .env file if it exists
        dotenvy::dotenv().ok();

        let openai_api_key = env::var("OPENAI_API_KEY")
            .expect("OPENAI_API_KEY must be set in .env or environment");

        Self { openai_api_key }
    }
}

// Create a static, lazy-loaded instance of the settings
// This is the Rust equivalent of the module-level `settings = Settings()` in Python
pub static SETTINGS: Lazy<Settings> = Lazy::new(Settings::load);