# Recipe from Session: Create, Configure Parameters, and Run

1. Send "Hello" and wait for the assistant to respond
2. Click the recipe action button (chef hat icon) in the bottom chat bar
3. Wait for the recipe form to appear after session analysis completes
4. Verify title, description, and instructions are pre-filled (not empty)
5. Update instructions to include a template parameter `{{greeting_style}}`
6. Update the prompt to reference `{{greeting_style}}`
7. Open Advanced Options and verify `greeting_style` was auto-detected as a parameter
8. Add a manual parameter `extra_param` and verify it shows "Unused"
9. Add an activity referencing `{{extra_param}}` and verify the unused indicator disappears
10. Save the recipe
11. Navigate to Recipes and verify the recipe is listed
12. Run the recipe from the library and trust-execute it
13. Fill parameter values (`greeting_style` = "formal", `extra_param` = "test_value") and start
14. Verify the recipe runs with substituted parameters in the prompt and activity
