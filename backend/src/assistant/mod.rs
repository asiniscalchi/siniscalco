mod handlers;
mod mock;
mod model_registry;
mod openai_client;
mod tool_executor;
mod types;

// ── Public API surface (unchanged from the old assistant.rs) ──────────────────

pub use handlers::{
    chat, delete_system_prompt, get_system_prompt, models, select_model, set_reasoning_effort,
    update_system_prompt,
};

pub use model_registry::{
    AssistantModelRegistry, SETTING_SELECTED_MODEL, SharedAssistantChatSemaphore,
    SharedAssistantModelRegistry, load_reasoning_effort_setting, load_selected_model_setting,
    new_assistant_chat_semaphore, new_shared_assistant_model_registry, openai_models_url,
    refresh_assistant_model_registry, spawn_assistant_model_refresh_task,
};

pub use openai_client::openai_responses_url;

pub use types::{
    AssistantChatErrorResponse, AssistantChatMessageRequest, AssistantChatRequest,
    AssistantChatResponse, AssistantModelRefreshError, AssistantModelSelectionRequest,
    AssistantModelsResponse, ReasoningEffort, SystemPromptResponse, UpdateSystemPromptRequest,
};
