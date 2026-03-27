use crate::generator::types::TrainingSample;
use crate::pipeline::config::QualityConfig;

use super::token_counter;

/// Result of filtering a sample.
#[derive(Debug)]
pub enum FilterResult {
    Pass,
    Reject(String),
}

/// Run all quality filters on a sample. Returns Pass or Reject with reason.
pub fn filter_sample(sample: &TrainingSample, config: &QualityConfig) -> FilterResult {
    // 1. Check token bounds
    let token_count = token_counter::count_sample_tokens(&sample.conversation);
    if token_count < config.min_tokens_per_sample {
        return FilterResult::Reject(format!(
            "Too few tokens: {} < {}",
            token_count, config.min_tokens_per_sample
        ));
    }
    if token_count > config.max_tokens_per_sample {
        return FilterResult::Reject(format!(
            "Too many tokens: {} > {}",
            token_count, config.max_tokens_per_sample
        ));
    }

    // 2. Check for empty turns
    for turn in &sample.conversation {
        if turn.content.trim().is_empty() && turn.tool_calls.is_none() {
            return FilterResult::Reject("Contains empty conversation turn".to_string());
        }
    }

    // 3. Check assistant turn content quality
    for turn in &sample.conversation {
        if turn.role == crate::generator::types::Role::Assistant {
            let content = &turn.content;

            // Check average line length
            let lines: Vec<&str> = content.lines().collect();
            if !lines.is_empty() {
                let avg_line_len = content.len() / lines.len();
                if avg_line_len > config.max_avg_line_length {
                    return FilterResult::Reject(format!(
                        "Average line length too high: {} > {}",
                        avg_line_len, config.max_avg_line_length
                    ));
                }
            }

            // Check alphanumeric fraction (skip code blocks)
            if !content.contains("```") {
                let alnum_count = content.chars().filter(|c| c.is_alphanumeric()).count();
                let total_chars = content.chars().count().max(1);
                let fraction = alnum_count as f64 / total_chars as f64;
                if fraction < config.min_alphanumeric_fraction {
                    return FilterResult::Reject(format!(
                        "Low alphanumeric fraction: {:.2} < {:.2}",
                        fraction, config.min_alphanumeric_fraction
                    ));
                }
            }
        }
    }

    // 4. Check source code in user turns isn't too short
    for turn in &sample.conversation {
        if turn.role == crate::generator::types::Role::User && turn.content.contains("```") {
            let code_content: String = turn
                .content
                .lines()
                .skip_while(|l| !l.starts_with("```"))
                .skip(1)
                .take_while(|l| !l.starts_with("```"))
                .collect::<Vec<&str>>()
                .join("\n");

            let code_lines = code_content.lines().count();
            if code_lines > 0 && code_lines < config.min_line_count {
                return FilterResult::Reject(format!(
                    "Code too short: {} lines < {} minimum",
                    code_lines, config.min_line_count
                ));
            }
        }
    }

    FilterResult::Pass
}
