use std::collections::HashMap;

use crate::export::statistics::DatasetStatistics;
use crate::pipeline::config::OutputFormat;
use crate::pipeline::orchestrator::PipelineResult;

/// Generate a HF-compatible dataset card (README.md) with YAML frontmatter.
pub fn generate_dataset_card(
    _result: &PipelineResult,
    stats: &DatasetStatistics,
    project_name: &str,
    formats: &[OutputFormat],
    license: &str,
) -> String {
    let safe_name = escape_yaml_string(project_name);
    let size_category = size_category(stats.total_samples);
    let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M UTC").to_string();
    let data_files_yaml = build_data_files_yaml(formats);
    let features_yaml = build_features_yaml(formats);
    let splits_yaml = build_splits_yaml(stats.total_samples);
    let sample_type_rows = build_sample_type_rows(&stats.samples_by_type);
    let language_rows = build_language_rows(&stats.samples_by_language);
    let formats_list = formats
        .iter()
        .map(|f| f.to_string())
        .collect::<Vec<_>>()
        .join(", ");
    let format_description = build_format_description(formats);

    format!(
        r#"---
license: {license}
language:
  - en
pretty_name: "{project_name} Training Data"
size_categories:
  - "{size_category}"
tags:
  - code
  - synthetic
  - localmotive
  - fine-tuning
task_categories:
  - text-generation
configs:
{data_files_yaml}
dataset_info:
  features:
{features_yaml}
  splits:
{splits_yaml}
---

# {project_name} Training Data

LLM fine-tuning training data generated from source code by [Localmotive](https://github.com/localmotive/localmotive).

## Dataset Details

- **Source project:** {project_name}
- **Generated:** {timestamp}
- **Total samples:** {total_samples}
- **Output formats:** {formats_list}

### Sample Distribution

| Type | Count |
|------|-------|
{sample_type_rows}

### Token Statistics

| Metric | Value |
|--------|-------|
| Min tokens | {token_min} |
| Max tokens | {token_max} |
| Mean tokens | {token_mean} |
| Median tokens | {token_median} |
| P90 tokens | {token_p90} |

### Languages

{language_rows}

## Output Format

{format_description}

## Generation

Generated with Localmotive v0.1.0. Training samples are deterministic, template-based (no LLM inference).
"#,
        license = license,
        project_name = safe_name,
        size_category = size_category,
        data_files_yaml = data_files_yaml,
        features_yaml = features_yaml,
        splits_yaml = splits_yaml,
        timestamp = timestamp,
        total_samples = stats.total_samples,
        formats_list = formats_list,
        sample_type_rows = sample_type_rows,
        token_min = stats.token_stats.min,
        token_max = stats.token_stats.max,
        token_mean = stats.token_stats.mean,
        token_median = stats.token_stats.median,
        token_p90 = stats.token_stats.p90,
        language_rows = language_rows,
        format_description = format_description,
    )
}

/// Escape a string for safe use in YAML values (quoted context).
fn escape_yaml_string(s: &str) -> String {
    s.replace('"', "\\\"")
        .replace('\n', " ")
        .replace('\r', "")
        .replace(':', " -")
}

fn size_category(total: usize) -> &'static str {
    match total {
        0..1_000 => "n<1K",
        1_000..10_000 => "1K<n<10K",
        10_000..100_000 => "10K<n<100K",
        100_000..1_000_000 => "100K<n<1M",
        _ => "1M<n<10M",
    }
}

fn build_data_files_yaml(formats: &[OutputFormat]) -> String {
    let mut lines = Vec::new();
    for fmt in formats {
        let config_name = format_config_name(fmt);
        lines.push(format!(
            "  - config_name: {config_name}\n    data_files:\n      - split: train\n        path: \"data/{config_name}/train.jsonl\""
        ));
    }
    lines.join("\n")
}

fn build_features_yaml(formats: &[OutputFormat]) -> String {
    // Use the first format's feature schema as the primary
    let fmt = formats.first().unwrap_or(&OutputFormat::OpenAI);
    match fmt {
        OutputFormat::OpenAI => {
            "    - name: messages\n      list:\n        - name: role\n          dtype: string\n        - name: content\n          dtype: string".to_string()
        }
        OutputFormat::Alpaca => {
            "    - name: instruction\n      dtype: string\n    - name: input\n      dtype: string\n    - name: output\n      dtype: string".to_string()
        }
        OutputFormat::ShareGPT => {
            "    - name: conversations\n      list:\n        - name: from\n          dtype: string\n        - name: value\n          dtype: string".to_string()
        }
        OutputFormat::ChatML | OutputFormat::Llama4 | OutputFormat::Mistral => {
            "    - name: text\n      dtype: string".to_string()
        }
    }
}

fn build_splits_yaml(total: usize) -> String {
    format!(
        "    - name: train\n      num_examples: {total}"
    )
}

fn build_sample_type_rows(by_type: &HashMap<String, usize>) -> String {
    let mut rows: Vec<_> = by_type.iter().collect();
    rows.sort_by(|a, b| b.1.cmp(a.1));
    rows.iter()
        .map(|(t, c)| format!("| {t} | {c} |"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn build_language_rows(by_lang: &HashMap<String, usize>) -> String {
    let mut rows: Vec<_> = by_lang.iter().collect();
    rows.sort_by(|a, b| b.1.cmp(a.1));
    rows.iter()
        .map(|(l, c)| format!("- **{l}:** {c} samples"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn build_format_description(formats: &[OutputFormat]) -> String {
    formats
        .iter()
        .map(|f| match f {
            OutputFormat::OpenAI => {
                "**OpenAI Chat** — `messages` array with `role`/`content` objects."
            }
            OutputFormat::ChatML => {
                "**ChatML/Hermes** — `<|im_start|>`/`<|im_end|>` delimited text."
            }
            OutputFormat::Llama4 => {
                "**Llama 4** — Header token delimited conversation format."
            }
            OutputFormat::Mistral => {
                "**Mistral** — `[INST]`/`[/INST]` control token format."
            }
            OutputFormat::ShareGPT => {
                "**ShareGPT** — `conversations` array with `from`/`value` objects."
            }
            OutputFormat::Alpaca => {
                "**Alpaca** — `instruction`/`input`/`output` triplets."
            }
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

pub fn format_config_name(format: &OutputFormat) -> String {
    match format {
        OutputFormat::OpenAI => "openai".to_string(),
        OutputFormat::ChatML => "chatml".to_string(),
        OutputFormat::Llama4 => "llama4".to_string(),
        OutputFormat::Mistral => "mistral".to_string(),
        OutputFormat::ShareGPT => "sharegpt".to_string(),
        OutputFormat::Alpaca => "alpaca".to_string(),
    }
}
