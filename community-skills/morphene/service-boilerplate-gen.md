# Skill: service-boilerplate-gen

## Description
Generates standardized service modules following the MORPHENE singleton/registry pattern.

## Instructions
1. Pattern Match: Read existing services (e.g., src/js/services/outfit.js) to identify the export structure and store.js integration.
2. Generate: Create a new file in src/js/services/ with: Standard JSDoc headers, Initialization logic, Integration with globalRegistry.
3. Register: Add the new service to the main entry point or registry so it's loaded by the engine.js.