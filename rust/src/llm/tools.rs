use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

macro_rules! define_tool {
    ($name:ident, $description:expr) => {
        impl Tool for $name {
            fn make_tool() -> async_openai::types::chat::ChatCompletionTools {
                let function = async_openai::types::chat::FunctionObjectArgs::default()
                    .parameters(schemars::schema_for!(Self))
                    .name(stringify!($name))
                    .description($description)
                    .strict(true)
                    .build()
                    .unwrap();
                async_openai::types::chat::ChatCompletionTools::Function(
                    async_openai::types::chat::ChatCompletionTool { function },
                )
            }
        }
    };
}

pub trait Tool {
    fn make_tool() -> async_openai::types::chat::ChatCompletionTools;
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct Abstain;

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct Whisper {
    pub to: u8,
    pub message: String,
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct TagPlayerForComment {
    pub id: u8,
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct ProvideID {
    pub id: u8,
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct Talk {
    pub message: String,
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct MultiCall {
    pub actions: Vec<ToolInvocation>,
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct ToolInvocation {
    pub tool: String,
    pub arguments: serde_json::Value,
}

define_tool!(Abstain, "Abstain from the current action");
define_tool!(Whisper, "Whisper a message to another player");
define_tool!(TagPlayerForComment, "Tag a player for comment");
define_tool!(ProvideID, "Provide the ID of another player");
define_tool!(Talk, "Talk to another player");
define_tool!(MultiCall, "Invoke multiple tools in sequence");
