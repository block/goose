# RMCP Servers

* We're converting MCP servers from using an internal mcp crate to using rmcp
* Work with full files and full diffs - don't show ranges - just read the whole things into your context window to get full context
* Look at 6807b6d0a31ecc194b0d8018d12f2284e22dc010 for an example of the DeveloperServer that was converted
* Look at b5749d645736546c8f7106e9fe510c0bf70eec3b for how I brought back the original tests for the DeveloperServer
* Do a full git show on each of those and keep it in context
* Look at the converted versions of the following servers in the following files
    * crates/goose-mcp/src/autovisualiser/rmcp_autovisualiser.rs
    * crates/goose-mcp/src/computercontroller/rmcp_computercontroller.rs
    * crates/goose-mcp/src/memory/rmcp_memory.rs
    * crates/goose-mcp/src/tutorial/rmcp_tutorial.rs
* Look at the original versions of the same in the following files
    * /Users/alexhancock/Desktop/old-mcp-servers/autovisualiser/mod.rs
    * /Users/alexhancock/Desktop/old-mcp-servers/computercontroller/mod.rs
    * /Users/alexhancock/Desktop/old-mcp-servers/memory/mod.rs
    * /Users/alexhancock/Desktop/old-mcp-servers/tutorial/mod.rs
* Then finish the migration for each of these servers and do two things differently than the current state
    * I want to preserve all tests from the original mod.rs files, and follow the example of b5749d645736546c8f7106e9fe510c0bf70eec3b for how to test rmcp based servers
    * I want to keep each server in the mod.rs file instead of a separate file prefixed with rmcp_
