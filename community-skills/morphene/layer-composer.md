# Skill: layer-composer

## Description
Manages the global rendering order and conditional visibility logic for multi-layered avatar systems.

## Instructions
1. Read Hierarchy: Analyze src/js/core/schema.js and CONFIG.LAYERS in config.js.
2. Compose: When asked to "move a layer," calculate the new priority values to ensure it sits correctly between existing layers without collisions.
3. Logic Rules: Update OverrideRule set in schema.js (e.g., "If 'outerwear' is equipped, hide 'tops'").
4. Sync: Automatically run tools/update_layers.py (or equivalent) after logic changes to ensure the data layer matches the code.