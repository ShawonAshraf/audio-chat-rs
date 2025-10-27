use serde::{Deserialize, Serialize};

//--- Outgoing Messages (Client -> OpenAI) ---

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SessionUpdate<'a> {
    #[serde(rename = "type")]
    pub event_type: &'a str,
    pub session: SessionConfig<'a>,
}

impl<'a> SessionUpdate<'a> {
    pub fn new() -> Self {
        Self {
            event_type: "session.update",
            session: SessionConfig {
                modalities: &["text", "audio"],
                instructions: "You are a helpful AI assistant. Have a natural conversation with the user in English.",
                voice: "alloy",
                input_audio_format: "pcm16",
                output_audio_format: "pcm16",
                input_audio_transcription: TranscriptionConfig { model: "whisper-1" },
                turn_detection: None,
            },
        }
    }
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SessionConfig<'a> {
    pub modalities: &'a [&'a str],
    pub instructions: &'a str,
    pub voice: &'a str,
    pub input_audio_format: &'a str,
    pub output_audio_format: &'a str,
    pub input_audio_transcription: TranscriptionConfig<'a>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub turn_detection: Option<serde_json::Value>,
}

#[derive(Serialize, Debug)]
pub struct TranscriptionConfig<'a> {
    pub model: &'a str,
}

#[derive(Serialize, Debug)]
pub struct AudioAppend { 
    #[serde(rename = "type")]
    pub event_type: &'static str,
    pub audio: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sequence_id: Option<i32>,
}

// Add a custom constructor for default values
impl AudioAppend {
    pub fn new(audio: String) -> Self {
        Self {
            event_type: "input_audio_buffer.append",
            audio,
            sequence_id: None,
        }
    }
}

#[derive(Serialize, Debug)]
pub struct Commit<'a> {
    #[serde(rename = "type")]
    pub event_type: &'a str,
}

impl<'a> Default for Commit<'a> {
    fn default() -> Self {
        Self {
            event_type: "input_audio_buffer.commit",
        }
    }
}


#[derive(Serialize, Debug)]
pub struct ResponseCreate<'a> {
    #[serde(rename = "type")]
    pub event_type: &'a str,
    pub response: ResponseConfig<'a>,
}

impl<'a> Default for ResponseCreate<'a> {
    fn default() -> Self {
        Self {
            event_type: "response.create",
            response: ResponseConfig {
                modalities: &["text", "audio"],
            },
        }
    }
}

#[derive(Serialize, Debug)]
pub struct ResponseConfig<'a> {
    pub modalities: &'a [&'a str],
}

//--- Incoming Messages (OpenAI -> Client) ---

#[derive(Deserialize, Debug)]
pub struct OpenAIEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    #[serde(default)]
    pub delta: Option<String>,
    // Add other fields you might care about, e.g.
    // pub text: Option<String>,
    // pub sequence_id: Option<i32>,
}
