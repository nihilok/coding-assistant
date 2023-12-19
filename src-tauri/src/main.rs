// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod history;
mod structs;

// Standard Library
use std::error;
use std::fs::File;
use std::io::Read;

// External Crates
use async_openai::types::{
    ChatCompletionRequestAssistantMessageArgs, ChatCompletionRequestMessage,
    ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestUserMessageArgs,
    CreateChatCompletionRequestArgs,
};

use dirs;
use dirs::home_dir;

use async_openai::Client as OpenAiClient;
use async_stream::stream;
use futures::Stream;
use futures::StreamExt;
use pin_utils::pin_mut;
use structs::{ChatMessageBuildError, History, Message, Role};
use tauri::Window;

static COMPLETION_TOKENS: u16 = 1024;
static MAX_HISTORY_LENGTH: usize = 10;
static GPT_4_MODEL: &'static str = "gpt-4-1106-preview";
static GPT_3_MODEL: &'static str = "gpt-3.5-turbo-1106";
const SYSTEM_MESSAGE: &'static str = include_str!("system-message.txt");

fn select_model(low_cost: bool) -> &'static str {
    if low_cost {
        return GPT_3_MODEL;
    }
    GPT_4_MODEL
}

fn build_request_message(
    role: &Role,
    content: &str,
) -> Result<ChatCompletionRequestMessage, ChatMessageBuildError> {
    match role {
        Role::SYSTEM => Ok(ChatCompletionRequestSystemMessageArgs::default()
            .content(content)
            .build()
            .map_err(|_| "Failed to build chat completion system message")?
            .into()),
        Role::USER => Ok(ChatCompletionRequestUserMessageArgs::default()
            .content(content)
            .build()
            .map_err(|_| "Failed to build chat completion user message")?
            .into()),
        Role::ASSISTANT => Ok(ChatCompletionRequestAssistantMessageArgs::default()
            .content(content)
            .build()
            .map_err(|_| "Failed to build chat completion assistant message")?
            .into()),
    }
}

async fn start_conversation_stream(
    history: &History,
    low_cost: bool,
) -> Result<
    impl Stream<Item = Result<String, Box<dyn error::Error + Send + Sync>>>,
    Box<dyn error::Error>,
> {
    let model = select_model(low_cost);

    // Create a client with your API key
    let api_key = match home_dir() {
        Some(mut path) => {
            path.push(".openai_api_key");
            let mut file = File::open(path)
                .map_err(|_| "Failed to open .openai_api_key file in home directory.")?;
            let mut api_key = String::new();
            file.read_to_string(&mut api_key)
                .map_err(|_| "Failed to read .openai_api_key file in home directory.")?;
            api_key.trim().to_string() // Here, trim is used to remove leading and trailing spaces.
        }
        None => return Err("Failed to get home directory.".into()),
    };
    let config = async_openai::config::OpenAIConfig::new().with_api_key(&api_key);
    let client = OpenAiClient::with_config(config);

    // Convert history messages into request messages
    let messages: Vec<ChatCompletionRequestMessage> = history
        .history
        .iter()
        .map(|message| build_request_message(&message.role, message.content.as_str()))
        .collect::<Result<Vec<ChatCompletionRequestMessage>, _>>()?;

    let request = CreateChatCompletionRequestArgs::default()
        .max_tokens(COMPLETION_TOKENS)
        .model(model)
        .messages(messages)
        .build()?;

    // Create the stream using the client
    let request_stream = stream! {
        let mut chat_stream = client.chat().create_stream(request).await?;

        while let Some(result) = chat_stream.next().await {
            match result {
                Ok(response) => {
                    for chat_choice in response.choices.iter() {
                        if let Some(ref content) = chat_choice.delta.content {
                            // Yield the chat message to the stream
                            yield Ok(content.clone());
                        }
                    }
                }
                Err(err) => {
                    // Handle error by sending it to the frontend or logging
                    yield Err(err.into());
                    break;
                }
            }
        }
    };

    Ok(request_stream)
}
#[tauri::command]
async fn prompt(window: Window, markdown: &str, low_cost: bool) -> Result<String, String> {
    let mut history = history::read_history().map_err(|e| e.to_string())?;

    history.history.push(Message::new(Role::USER, markdown));

    // Start the conversation as a stream that emits events
    let stream = start_conversation_stream(&history, low_cost)
        .await
        .map_err(|e| format!("Failed to start conversation: {}", e))?;
    pin_mut!(stream);
    let mut temp_history = String::new();
    while let Some(result) = stream.next().await {
        match result {
            Ok(chat_choice) => {
                // Emit the chat message to the front-end
                window
                    .emit("chat-message", &chat_choice)
                    .map_err(|e| format!("Failed to emit chat message: {}", e))?;

                temp_history.push_str(&chat_choice);
            }
            Err(_) => break, // On errors, you could emit another event to notify the front end
        }
    }

    // Update the history with the response
    history
        .history
        .push(Message::new(Role::ASSISTANT, &temp_history));

    // Save updated history
    if let Err(e) = history::write_history(&history) {
        return Err(format!("Failed to write history: {}", e));
    }
    // Return an empty string as this function will stream the response rather than return it
    Ok("".to_string())
}

#[tauri::command]
async fn get() -> Result<History, ()> {
    Ok(history::read_history().unwrap_or(History::new()))
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            prompt,
            get,
            history::clear_history
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
