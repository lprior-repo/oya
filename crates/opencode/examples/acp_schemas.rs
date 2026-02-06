//!
//! # ACP Schema Examples
//!
//! Examples of using the ACP (Agent Client Protocol) schemas in oya-opencode.
//!
//! Note: These are OpenCode's internal message format types (MessageV2),
//! not the Anthropic Context Protocol.

use oya_opencode::acp_schemas::*;

fn main() {
    // Create a text message
    let text_msg = AcpMessage::text("Hello, world!", 1);
    println!("Created text message: {:?}", text_msg);

    // Create a final chunk
    let final_msg = AcpMessage::final_chunk("Execution complete", 10);
    println!("Created final chunk: {:?}", final_msg);

    // Create an error message
    let error_msg = AcpMessage::error("Something went wrong", 5);
    println!("Created error message: {:?}", error_msg);

    // Create a tool state
    let tool_state = ToolState::Completed {
        input: Default::default(),
        output: "Success!".to_string(),
        title: "Test Tool".to_string(),
        metadata: None,
        time: ToolTimeRange {
            start: 1234567890,
            end: 1234567891,
            compacted: None,
        },
        attachments: None,
    };
    println!("Created tool state: {:?}", tool_state);

    // Create token usage
    let tokens = TokenUsage {
        input: 1000,
        output: 500,
        reasoning: 100,
        cache: CacheStats {
            read: 8000,
            write: 2000,
        },
    };
    println!("Token usage: {:?}", tokens);

    // Serialize to JSON
    if let Ok(json) = serde_json::to_string_pretty(&text_msg) {
        println!("\nSerialized message:");
        println!("{}", json);
    }
}
