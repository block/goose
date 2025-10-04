#!/usr/bin/env python3
"""
Example of handling tool confirmation requests in non-autonomous mode.

When the Goose server is not in autonomous mode, it will request confirmation
before executing certain tools. This example shows how to handle these
confirmation requests.
"""

import sys
from goose_client import GooseClient

def main():
    # Initialize client
    client = GooseClient(
        api_key="test_secret_8899",  # Replace with your API key
        base_url="http://localhost:8899"  # Replace with your server URL
    )
    
    # Check server health
    if not client.is_healthy:
        print("Error: Server is not healthy")
        sys.exit(1)
    
    print("Connected to Goose server")
    print("=" * 50)
    
    # Example 1: Auto-confirm all tools
    print("\nExample 1: Auto-confirm all tools with 'allow_once'")
    print("-" * 40)
    
    with client.session() as session:
        print(f"Created session: {session.id}")
        
        # This will auto-confirm any tool requests
        for chunk in client.stream_chat_with_confirmations(
            "List the files in the current directory",
            session.id,
            auto_confirm="allow_once"  # Automatically allow tools once
        ):
            if isinstance(chunk, str):
                print(chunk, end="", flush=True)
            else:
                # This won't be reached with auto_confirm, but shown for completeness
                print(f"\n[Tool confirmation auto-handled: {chunk.get('toolName')}]")
        print()
    
    # Example 2: Manual confirmation
    print("\nExample 2: Manual confirmation with user input")
    print("-" * 40)
    
    with client.session() as session:
        print(f"Created session: {session.id}")
        
        # This will prompt for each tool confirmation
        for chunk in client.stream_chat_with_confirmations(
            "Create a file called test.txt with 'Hello World' in it",
            session.id,
            auto_confirm=None  # Don't auto-confirm, handle manually
        ):
            if isinstance(chunk, str):
                # Regular text content
                print(chunk, end="", flush=True)
            else:
                # Tool confirmation request
                print(f"\n\n🔧 Tool Confirmation Request:")
                print(f"   Tool: {chunk.get('toolName')}")
                print(f"   ID: {chunk.get('id')}")
                
                # Show security prompt if present
                prompt = chunk.get('prompt')
                if prompt:
                    print(f"   ⚠️  Security: {prompt}")
                
                # Show arguments
                args = chunk.get('arguments', {})
                if args:
                    print(f"   Arguments: {args}")
                
                # Ask user for confirmation
                print("\n   Actions:")
                print("   1. Allow once")
                print("   2. Always allow")
                print("   3. Deny")
                
                choice = input("   Choose action (1-3): ").strip()
                
                # Map choice to action
                action_map = {
                    "1": "allow_once",
                    "2": "always_allow", 
                    "3": "deny"
                }
                action = action_map.get(choice, "deny")
                
                # Send confirmation
                success = client.confirm_permission(
                    chunk['id'],
                    action,
                    session.id
                )
                
                if success:
                    print(f"   ✅ Confirmed with action: {action}\n")
                else:
                    print(f"   ❌ Failed to confirm\n")
        print()
    
    # Example 3: Mixed auto-confirm and manual
    print("\nExample 3: Conditional auto-confirmation")
    print("-" * 40)
    
    with client.session() as session:
        print(f"Created session: {session.id}")
        
        # Define which tools to auto-confirm
        safe_tools = ["list_files", "read_file", "get_current_directory"]
        
        for chunk in client.stream_chat_with_confirmations(
            "Read the contents of README.md and then create a summary.txt file",
            session.id,
            auto_confirm=None  # Handle manually so we can be selective
        ):
            if isinstance(chunk, str):
                print(chunk, end="", flush=True)
            else:
                tool_name = chunk.get('toolName', '')
                
                # Extract the base tool name (remove extension prefix if present)
                base_tool = tool_name.split('__')[-1] if '__' in tool_name else tool_name
                
                if base_tool in safe_tools:
                    # Auto-confirm safe tools
                    print(f"\n[Auto-confirming safe tool: {tool_name}]")
                    client.confirm_permission(
                        chunk['id'],
                        "allow_once",
                        session.id
                    )
                else:
                    # Manual confirmation for other tools
                    print(f"\n\n⚠️  Manual confirmation needed for: {tool_name}")
                    
                    if chunk.get('prompt'):
                        print(f"   Security warning: {chunk['prompt']}")
                    
                    confirm = input("   Allow this tool? (y/n): ").strip().lower()
                    action = "allow_once" if confirm == 'y' else "deny"
                    
                    client.confirm_permission(
                        chunk['id'],
                        action,
                        session.id
                    )
                    print(f"   {'✅ Allowed' if confirm == 'y' else '❌ Denied'}\n")
        print()
    
    print("\n" + "=" * 50)
    print("Tool confirmation examples complete!")

if __name__ == "__main__":
    main()
