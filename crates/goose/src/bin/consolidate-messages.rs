//! One-time migration script to consolidate fragmented assistant messages
//!
//! This script fixes chat histories that were broken up during streaming before
//! the consolidation fix was implemented.
//!
//! Usage:
//!   cargo run --bin consolidate-messages

use anyhow::Result;
use goose::session::session_manager::SessionManager;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🔧 Consolidating Fragmented Messages");
    println!("=====================================");
    println!();
    println!("This will merge consecutive assistant text messages that were");
    println!("fragmented during streaming. This operation is safe and can be");
    println!("run multiple times.");
    println!();

    print!("Scanning database... ");
    let count = SessionManager::consolidate_fragmented_messages().await?;
    println!("done!");
    println!();

    if count == 0 {
        println!("✅ No fragmented messages found - your database is already clean!");
    } else {
        println!("✅ Successfully consolidated {} message fragments", count);
        println!("   Your chat history should now display correctly!");
    }

    println!();
    println!("🎉 Migration complete!");
    println!();
    Ok(())
}
