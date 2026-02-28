@release @recipes
Feature: Recipes
  Release checklist: verify recipe page navigation and recipe creation from session.

  Background:
    Given the app is loaded and the chat input is visible

  Scenario: Navigate to Recipes page
    When I navigate to "Recipes" in the sidebar
    Then I take a screenshot to verify:
      - The Recipes page is displayed with "Recipes" heading
      - "Create Recipe" and "Import Recipe" buttons are visible
      - Recipe list shows existing recipes (or empty state)

  Scenario: Create a recipe from a chat session
    Given I have an open conversation with at least one exchange
    # The chef's hat icon is the 2nd button from right in the bottom status bar
    # It has NO title or aria-label — identify by position or 2-path SVG
    # The RIGHTMOST button (11-path SVG) is the bug report icon — NOT the chef's hat
    When I click the chef's hat icon in the bottom bar
    Then the "Create Recipe from Session" modal should appear
    And I take a screenshot to verify:
      - Title is auto-filled from the conversation
      - Description is auto-filled
      - Instructions are auto-filled with behavioral guidance
      - "Create Recipe" and "Create & Run Recipe" buttons are visible
    When I click "Cancel" to close the modal
    # To find the chef's hat programmatically:
    # Get all buttons with y > 560, sort by x descending, pick the 2nd one
    # Or find the button with exactly 2 SVG path elements
