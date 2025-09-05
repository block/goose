# RMCP Servers

* We're converting MCP servers from using an internal mcp crate to using rmcp
* Look at 6807b6d0a31ecc194b0d8018d12f2284e22dc010 for an example of another server that was converted
* Look at b5749d645736546c8f7106e9fe510c0bf70eec3b for how I brought back the original tests
* Now convert the following servers within crates/goose-mcp:
    * autovisualizer
    * computercontroller
    * memory
    * tutorial
* I want to do two things differently than the prior conversion in 6807b6d0a31ecc194b0d8018d12f2284e22dc010
    * I want to preserve all tests, and follow how I did it in b5749d645736546c8f7106e9fe510c0bf70eec3b
    * I want to keep the server in the mod.rs file instead of a separate file prefixed with rmcp_
