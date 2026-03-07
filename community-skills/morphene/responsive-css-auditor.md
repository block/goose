# Skill: responsive-css-auditor

## Description
Audits UI components for responsive breakage and theme consistency using headless browser snapshots.

## Instructions
1. Emulate: Use Chromedevtools.emulate to test the UI at different resolutions (Desktop vs Mobile).
2. Snapshot: Use Chromedevtools.takeScreenshot on specific components (like .race-preview or .social-grid).
3. Audit: Compare the computed styles against config.js constants. Check for image-rendering: pixelated consistency on all avatar previews.
4. Report: List CSS classes that are redundant or breaking the "MORPHENE_OS" aesthetic.