@release @extensions
Feature: Extensions
  Release checklist: verify adding custom extensions and using them in chat.

  Background:
    Given the app is loaded and the chat input is visible

  Scenario: Navigate to Extensions page
    When I navigate to "Extensions" in the sidebar
    Then I take a screenshot to verify:
      - "Extensions" heading is visible
      - "Add custom extension" button is visible
      - "Browse extensions" button is visible
      - "Default Extensions" section shows enabled extensions with toggle switches

  Scenario: Add the Running Quotes custom extension
    When I navigate to "Extensions" in the sidebar
    # Remove if exists: check innerText, scroll to card, click gear (coordinate),
    # click "Remove extension" (may need scrollIntoView), click "Confirm removal"
    And I remove the "Running Quotes" extension if it already exists
    And I click the "Add custom extension" button
    Then a modal dialog should appear with form fields
    When I fill the extension name with "Running Quotes"
    And I fill the extension description with "Inspirational running quotes MCP server"
    And I fill the extension command with "node /Users/zane/Development/goose-main/ui/desktop/tests/e2e/basic-mcp.ts"
    # IMPORTANT: Use scrollIntoView on the submit button first, then click at
    # exact coordinates. Clicking near modal edges triggers "Unsaved Changes" dialog.
    # If that happens: click "No" to stay, then retry with exact coordinates.
    And I scroll the submit button into view and click it
    Then I take a screenshot to verify:
      - "Running Quotes" appears in the Default Extensions section
      - The toggle is enabled (blue)
      - Default Extensions count increased by 1

  Scenario: Use the Running Quotes extension in chat
    Given the "Running Quotes" extension is enabled
    # NOTE: Extension may not be immediately available in a new chat session.
    # The agent may use "Search Available Extensions" instead of the tool directly.
    When I click "Start New Chat" in the sidebar
    # DUAL CHAT-INPUT: use visible textarea + React native setter
    And I type "Give me an inspirational running quote using the runningQuote tool" into the visible chat input and press Enter
    Then the agent should respond within 30 seconds
    And I take a screenshot to verify:
      - A response is visible (may include tool invocation or direct quote)
      - The response contains a running/inspirational quote
