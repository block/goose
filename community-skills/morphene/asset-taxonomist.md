# Skill: asset-taxonomist

## Description
Scans asset directories, identifies visual traits using vision tools, and synchronizes them with project metadata/registry files.

## Instructions
1. Scan: Use Developer.shell to list files in asset directories (e.g., ./BASE AVATARS).
2. Analyze: For new or unmapped files, use Developer.imageProcessor to identify colors, species, and "vibe".
3. Map: Update src/js/core/registry.js or assets/data/items.json to include the new assets with appropriate categories and layer priorities based on the naming convention (e.g., _skin_ -> SKIN layer).
4. Consistency Check: Ensure all registered assets actually exist on disk and vice-versa.