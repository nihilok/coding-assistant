use crate::history;
use crate::structs::{ChatMessageBuildError, History, Message, Role};
use async_openai::types::{
    ChatCompletionRequestAssistantMessageArgs, ChatCompletionRequestMessage,
    ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestUserMessageArgs,
    CreateChatCompletionRequestArgs,
};
use async_openai::Client as OpenAiClient;
use async_stream::stream;
use dirs::home_dir;
use futures::{Stream, StreamExt};
use lazy_static::lazy_static;
use pin_utils::pin_mut;
use std::error;
use std::fs::File;
use std::io::Read;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::Window;
use tokio::sync::Mutex;

pub const SYSTEM_MESSAGE: &'static str = include_str!("system-message.txt");

static MAX_HISTORY_LENGTH: usize = 12;
static COMPLETION_TOKENS: u16 = 1024;
static GPT_4_MODEL: &'static str = "gpt-4-1106-preview";
static GPT_3_MODEL: &'static str = "gpt-3.5-turbo-1106";

lazy_static! {
    static ref PROMPT_LOCK: Arc<Mutex<()>> = Arc::new(Mutex::new(()));
}

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

/// Start a conversation stream with OpenAI.
///
/// This function creates a stream that communicates with the OpenAI Chat API to generate
/// responses for a conversation history. It takes a reference to a `History` object, which
/// contains the conversation history, and a `low_cost` boolean indicating whether to use
/// the low-cost model for generating responses.
///
/// The function returns a `Result` containing the conversation stream. The stream yields
/// `Result` objects, where each item of the stream is either a generated response as a
/// `String`, or an error as a boxed `dyn error::Error` trait object.
///
/// # Arguments
///
/// * `history` - A reference to a `History` object containing the conversation history.
/// * `low_cost` - A boolean indicating whether to use the low-cost model for generating responses.
///
/// # Examples
///
/// ```rust
/// use crate::start_conversation_stream;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let history = // create a History object
/// let low_cost = true;
/// let conversation_stream = start_conversation_stream(&history, low_cost).await?;
/// // process the conversation stream
/// # Ok(())
/// # }
/// ```
async fn start_conversation_stream(
    history: &History,
    low_cost: bool,
    api_key: String,
) -> Result<
    impl Stream<Item = Result<String, Box<dyn error::Error + Send + Sync>>>,
    Box<dyn error::Error>,
> {
    let model = select_model(low_cost);

    // Create a client with your API key
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

fn get_api_key() -> Result<String, String> {
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
    Ok(api_key)
}

/// Starts a response stream from a markdown prompt provided by the user. Thread safety when reading and writing
/// history to file is preserved by means of a global lock.
///
/// # Arguments
///
/// * `window` - The Tauri window.
/// * `markdown` - The markdown message provided as a prompt to the LLM.
/// * `low_cost` - A flag indicating whether to enable low cost mode (uses gpt-3.5-turbo model instead of gpt4).
///
/// # Returns
///
/// Returns a `Result` with the response string, or an error message if an error occurs.
///
#[tauri::command]
pub async fn prompt(window: Window, markdown: &str, low_cost: bool) -> Result<String, String> {
    // Acquire lock
    let _lock = PROMPT_LOCK.lock().await;

    let mut history = history::read_history().map_err(|e| e.to_string())?;
    history.history.push(Message::new(Role::USER, markdown));
    let api_key = get_api_key().map_err(|e| format!("Error: {}", e))?;

    // Start the conversation as a stream that emits events
    let truncated_history = history::truncate_history(&mut history, MAX_HISTORY_LENGTH); // truncated to max length currently set by constant in history.rs
    let stream = start_conversation_stream(truncated_history, low_cost, api_key)
        .await
        .map_err(|e| format!("Failed to start conversation: {}", e))?;
    pin_mut!(stream);
    let mut response_buffer = String::new();

    // Shared state for communication between the event listener and the streaming loop
    let canceled = Arc::new(AtomicBool::new(false));
    let cancel_flag = canceled.clone(); // Clone the Arc for use in the event listener

    let cancel_listener_id = window.listen("cancel-stream", move |_| {
        cancel_flag.store(true, Ordering::SeqCst); // Set the cancellation flag
    });
    while let Some(result) = stream.next().await {
        if canceled.load(Ordering::SeqCst) {
            break;
        }
        match result {
            Ok(chat_choice) => {
                // Emit the chat message to the front-end
                window
                    .emit("chat-message", &chat_choice)
                    .map_err(|e| format!("Failed to emit chat message: {}", e))?;

                response_buffer.push_str(&chat_choice);
            }
            Err(err) => window
                .emit("stream-error", &err.to_string())
                .map_err(|e| format!("Failed to emit chat message: {}", e))?,
        }
    }

    // Unlisten to the cancel event since we are done with the stream
    window.unlisten(cancel_listener_id);

    // Update the history with the response (or partial response)
    history
        .history
        .push(Message::new(Role::ASSISTANT, &response_buffer));

    // Save updated history
    if let Err(e) = history::write_history(&history) {
        return Err(format!("Failed to write history: {}", e));
    }
    // Return full response in case of slow connection
    Ok(response_buffer)
}
