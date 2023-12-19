use crate::structs::{ChatMessageBuildError, History, Message, Role};
use crate::{history, COMPLETION_TOKENS};
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
pub async fn prompt(window: Window, markdown: &str, low_cost: bool) -> Result<String, String> {
    // Acquire lock
    let _lock = PROMPT_LOCK.lock().await;

    let mut history = history::read_history().map_err(|e| e.to_string())?;

    history.history.push(Message::new(Role::USER, markdown));

    // Start the conversation as a stream that emits events
    let stream = start_conversation_stream(&history, low_cost)
        .await
        .map_err(|e| format!("Failed to start conversation: {}", e))?;
    pin_mut!(stream);
    let mut temp_history = String::new();
    // Shared state for communication between the event listener and the streaming loop
    let canceled = Arc::new(AtomicBool::new(false));
    let cancel_flag = canceled.clone(); // Clone the Arc for use in the event listener

    // Listen for the cancel-stream event
    let cancel_listener_id = window.listen("cancel-stream", move |_| {
        cancel_flag.store(true, Ordering::SeqCst); // Set the cancellation flag
    });
    while let Some(result) = stream.next().await {
        // Exit loop if stream has been canceled
        if canceled.load(Ordering::SeqCst) {
            break;
        }
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

    // Unlisten to the cancel event since we are done with the stream
    window.unlisten(cancel_listener_id);

    // Update the history with the response (or partial response)
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
