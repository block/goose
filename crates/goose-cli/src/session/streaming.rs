use goose::conversation::message::{Message, MessageContent};
use rmcp::model::Role;
use std::io::{self, IsTerminal, Write};

/// Manages streaming output for the CLI by accumulating messages with the same ID
pub struct StreamingRenderer {
    /// ID of the last message we processed
    last_message_id: Option<String>,
    /// Message we're currently accumulating
    pub accumulating_message: Option<Message>,
    /// Whether terminal supports streaming
    supports_streaming: bool,
}

impl StreamingRenderer {
    pub fn new() -> Self {
        Self {
            last_message_id: None,
            accumulating_message: None,
            supports_streaming: io::stdout().is_terminal(),
        }
    }

    /// Process a message - returns Some(completed_message) if a message was finalized
    pub fn process_message(&mut self, message: &Message) -> io::Result<Option<Message>> {
        if !self.supports_streaming {
            return Ok(None);
        }

        // Only handle text-only assistant messages
        if message.role != Role::Assistant || message.content.len() != 1 {
            // Non-streaming message - finalize any pending message
            return Ok(self.finalize_internal());
        }

        if let MessageContent::Text(text) = &message.content[0] {
            if let Some(ref msg_id) = message.id {
                if self.last_message_id.as_ref() == Some(msg_id) {
                    // Continuation of previous message - accumulate
                    if !text.text.is_empty() {
                        print!("{}", text.text);
                        io::stdout().flush()?;
                        
                        if let Some(ref mut acc_msg) = self.accumulating_message {
                            if let Some(MessageContent::Text(acc_text)) = acc_msg.content.get_mut(0) {
                                acc_text.text.push_str(&text.text);
                            }
                        }
                    }
                    return Ok(None); // Still accumulating
                } else {
                    // New message ID - finalize previous and start new
                    let completed = self.finalize_internal();
                    
                    // Start new accumulation
                    println!(); // New line for streaming
                    if !text.text.is_empty() {
                        print!("{}", text.text);
                        io::stdout().flush()?;
                    }
                    
                    self.last_message_id = Some(msg_id.clone());
                    self.accumulating_message = Some(message.clone());
                    return Ok(completed);
                }
            }
        }

        // Message without ID or non-text - finalize any pending
        Ok(self.finalize_internal())
    }

    /// Internal finalization that doesn't print newline if nothing was accumulated
    fn finalize_internal(&mut self) -> Option<Message> {
        let msg = self.accumulating_message.take();
        if msg.is_some() {
            println!(); // End streaming line
        }
        self.last_message_id = None;
        msg
    }

    /// Call when stream ends to get any pending message
    pub fn finalize(&mut self) -> Option<Message> {
        self.finalize_internal()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_accumulation() {
        let mut renderer = StreamingRenderer {
            last_message_id: None,
            accumulating_message: None,
            supports_streaming: true, // Force enable for tests
        };
        
        // First chunk
        let msg1 = Message::assistant().with_id("msg-1").with_text("Hello ");
        let completed = renderer.process_message(&msg1).unwrap();
        assert!(completed.is_none());
        assert!(renderer.accumulating_message.is_some());
        
        // Second chunk (same ID) - should accumulate
        let msg2 = Message::assistant().with_id("msg-1").with_text("world");
        let completed = renderer.process_message(&msg2).unwrap();
        assert!(completed.is_none());
        
        // Verify accumulation
        let acc_msg = renderer.accumulating_message.as_ref().unwrap();
        assert_eq!(acc_msg.as_concat_text(), "Hello world");
        
        // New message (different ID) - should finalize previous
        let msg3 = Message::assistant().with_id("msg-2").with_text("New");
        let completed = renderer.process_message(&msg3).unwrap();
        assert!(completed.is_some());
        assert_eq!(completed.unwrap().as_concat_text(), "Hello world");
        
        // Finalize last message
        let final_msg = renderer.finalize();
        assert!(final_msg.is_some());
        assert_eq!(final_msg.unwrap().as_concat_text(), "New");
    }

    #[test]
    fn test_non_streaming_messages() {
        let mut renderer = StreamingRenderer {
            last_message_id: None,
            accumulating_message: None,
            supports_streaming: true, // Force enable for tests
        };
        
        // Start streaming
        let msg1 = Message::assistant().with_id("msg-1").with_text("Streaming");
        renderer.process_message(&msg1).unwrap();
        
        // Tool message should finalize streaming
        let tool_msg = Message::user().with_text("Tool response");
        let completed = renderer.process_message(&tool_msg).unwrap();
        assert!(completed.is_some());
        assert_eq!(completed.unwrap().as_concat_text(), "Streaming");
    }

    #[test]
    fn test_messages_without_ids() {
        let mut renderer = StreamingRenderer::new();
        
        // Message without ID should not be accumulated
        let msg = Message::assistant().with_text("No ID");
        let completed = renderer.process_message(&msg).unwrap();
        assert!(completed.is_none());
        assert!(renderer.accumulating_message.is_none());
    }
}