use std::sync::OnceLock;

use tiktoken_rs::CoreBPE;

static TOKENIZER: OnceLock<CoreBPE> = OnceLock::new();

fn tokenizer() -> &'static CoreBPE {
    TOKENIZER.get_or_init(|| {
        tiktoken_rs::cl100k_base().expect("failed to initialize cl100k_base tokenizer at startup")
    })
}

/// Count tokens using the cl100k_base tokenizer (GPT-4 / GPT-4o compatible).
pub fn count_tokens(text: &str) -> usize {
    tokenizer().encode_with_special_tokens(text).len()
}

/// Estimate tokens without the full tokenizer (fast approximation).
pub fn estimate_tokens(text: &str) -> usize {
    let words = text.split_whitespace().count();
    let chars = text.len();
    let char_estimate = chars * 10 / 35;
    let word_estimate = words * 13 / 10;
    (char_estimate + word_estimate) / 2
}

/// Count tokens for a full training sample's conversation.
pub fn count_sample_tokens(conversation: &[crate::generator::types::ConversationTurn]) -> usize {
    let mut total = 0;
    for turn in conversation {
        total += 4; // role token overhead
        total += count_tokens(&turn.content);
        if let Some(calls) = &turn.tool_calls {
            for call in calls {
                total += count_tokens(&call.function_name);
                total += count_tokens(&call.arguments.to_string());
                total += 8;
            }
        }
    }
    total
}
