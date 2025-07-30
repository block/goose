#[tokio::main]
async fn main() {
    use goose::token_counter::{TokenCounter, create_async_token_counter};
    use goose::message::{Message, MessageContent};
    // Test basic token counting
    let sync_counter = TokenCounter::new();
    let async_counter = create_async_token_counter().await.unwrap();
    
    let test_text = "Hello, how are you?";
    println!("Text: '{}'", test_text);
    println!("Sync tokens: {}", sync_counter.count_tokens(test_text));
    println!("Async tokens: {}", async_counter.count_tokens(test_text));
    
    // Test with a longer text
    let long_text = "This is a much longer piece of text that should have significantly more tokens than the short greeting. We want to see how the tokenizer handles different lengths of text and whether there are any discrepancies between the sync and async versions.";
    println!("\nLong text: '{}'", long_text);
    println!("Sync tokens: {}", sync_counter.count_tokens(long_text));
    println!("Async tokens: {}", async_counter.count_tokens(long_text));
    
    // Test with message counting
    let messages = vec![
        Message::new(
            Role::User,
            0,
            vec![MessageContent::text("What's the weather like?")],
        ),
        Message::new(
            Role::Assistant,
            1,
            vec![MessageContent::text("I don't have access to current weather data.")],
        ),
    ];
    
    println!("\nMessage token counting:");
    println!("Sync chat tokens: {}", sync_counter.count_chat_tokens("", &messages, &[]));
    println!("Async chat tokens: {}", async_counter.count_chat_tokens("", &messages, &[]));
    
    // Test with system prompt
    let system_prompt = "You are a helpful assistant.";
    println!("\nWith system prompt:");
    println!("Sync chat tokens: {}", sync_counter.count_chat_tokens(system_prompt, &messages, &[]));
    println!("Async chat tokens: {}", async_counter.count_chat_tokens(system_prompt, &messages, &[]));
    
    // Test individual message token counts
    println!("\nIndividual message tokens:");
    for (i, msg) in messages.iter().enumerate() {
        let sync_tokens = sync_counter.count_chat_tokens("", std::slice::from_ref(msg), &[]);
        let async_tokens = async_counter.count_chat_tokens("", std::slice::from_ref(msg), &[]);
        println!("Message {}: sync={}, async={}", i, sync_tokens, async_tokens);
    }
}
