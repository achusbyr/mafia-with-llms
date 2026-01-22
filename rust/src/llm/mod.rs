use serde::Deserialize;

pub mod ai_interface;
pub mod tools;

#[derive(Deserialize, Debug)]
struct OpenRouterResponse {
    choices: Vec<OpenRouterChoice>,
}

#[derive(Deserialize, Debug)]
struct OpenRouterChoice {
    message: OpenRouterMessage,
}

#[derive(Deserialize, Debug)]
struct OpenRouterMessage {
    /// Optional, as tool calls might replace it
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    reasoning: Option<String>,
    #[serde(default)]
    tool_calls: Option<Vec<OpenRouterToolCall>>,
}

#[derive(Deserialize, Debug)]
struct OpenRouterToolCall {
    function: OpenRouterFunction,
}

#[derive(Deserialize, Debug)]
struct OpenRouterFunction {
    name: String,
    arguments: String,
}
