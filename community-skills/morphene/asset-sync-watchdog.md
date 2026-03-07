# Skill: asset-sync-watchdog

## Description
An automated workflow agent that monitors raw folders and executes processing pipelines.

## Instructions
1. Monitor: Check the ./uncutr folder for new files using Developer.shell.
2. Process: Trigger tools/sync_assets.py or tools/ultimate_sync.py when changes are detected.
3. Verify: After sync, check src/js/core/config.js to ensure the VERSION is updated or the new asset count is reflected.
4. Alert: Notify the user if a sync fails due to malformed file names or missing categories.