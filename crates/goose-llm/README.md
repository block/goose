### goose-llm 

This crate is meant to be used for foreign function interface (FFI). It's meant to be 
stateless and contain logic related to providers and prompts:
- chat completion with model providers
- detecting read-only tools for smart approval
- methods for summarization / truncation


Run:
```
cargo run -p goose-llm --example simple
```

