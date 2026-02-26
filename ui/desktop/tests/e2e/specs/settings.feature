@release @settings
Feature: Settings
  Release checklist: verify settings page loads and dark mode works.

  Background:
    Given the app is loaded and the chat input is visible

  Scenario: Settings page loads and all tabs are accessible
    When I navigate to "Settings" in the sidebar
    Then I take a screenshot to verify "Settings" heading is visible
    # Verify all 7 tabs via data-testid query (single evaluate call)
    And all 7 settings tabs should be present in the DOM

  Scenario: Can change dark mode setting
    When I navigate to "Settings" in the sidebar
    # App tab may be off-screen in narrow windows — scrollIntoView first
    And I click the "App" settings tab via evaluate with scrollIntoView
    # Theme buttons are below the fold — scroll to dark-mode-button directly
    And I scroll the dark-mode-button into view
    When I click the "Dark" theme button
    Then document.documentElement.className should be "dark"
    And I take a screenshot to verify dark background
    When I click the "Light" theme button
    Then document.documentElement.className should be "light"

  Scenario: Models tab shows current provider info
    When I navigate to "Settings" in the sidebar
    Then I take a screenshot to verify:
      - The current model name is displayed (e.g., "goose-claude-4-6-opus")
      - The provider name is shown (e.g., "Databricks")
      - "Switch models" button is visible
      - "Configure providers" button is visible
