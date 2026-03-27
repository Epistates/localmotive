# Localmotive

A desktop application for generating high-quality LLM fine-tuning training data from source code repositories. Built with Tauri v2, SvelteKit 5, and Rust.

## What It Does

Localmotive scans codebases, performs AST analysis via tree-sitter, generates training samples through templates, and exports to JSONL in multiple model-specific formats:

- **OpenAI** — Chat completions format
- **ChatML / Hermes** — ChatML-tagged format
- **Llama 4** — Meta's conversation format
- **Mistral** — Mistral instruction format
- **ShareGPT** — Multi-turn conversation format
- **Alpaca** — Stanford Alpaca instruction format

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Desktop framework | Tauri v2 |
| Frontend | SvelteKit 5, Tailwind CSS v4, shadcn-svelte, Bits UI |
| Backend | Rust (tokio, rayon) |
| AST parsing | tree-sitter (Rust, TypeScript, JavaScript, Python, Go, Java, C, C++) |
| Token counting | tiktoken-rs |
| Package manager | bun |

## Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (stable)
- [Bun](https://bun.sh/)
- System dependencies for Tauri v2 — see [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/)

## Development

```bash
# Install dependencies
bun install

# Run in development mode
bun run tauri dev

# Type-check the frontend
bun run check

# Build for production
bun run tauri build
```

## License

MIT
