@release @conversations
Feature: Starting Conversations
  Release checklist: verify all the ways to start and manage conversations.

  Background:
    Given the app is loaded and the chat input is visible

  Scenario: Start a new conversation from home and get a response
    # "Start New Chat" is a <span> not <button> — use TreeWalker
    When I click "Start New Chat" in the sidebar
    # DUAL CHAT-INPUT: find visible textarea (height > 0), use React native setter
    And I type "Hello" into the visible chat input and press Enter
    Then the agent should respond within 30 seconds
    And I take a screenshot to verify:
      - The agent response is visible and not empty
      - A new chat session appears in the sidebar with green dot

  Scenario: New conversation appears in recent chats on home page
    When I navigate to "Home" in the sidebar
    Then I take a screenshot to verify:
      - "Recent chats" section is visible
      - The conversation we just created appears in the list with today's date

  Scenario: Load a conversation from history
    When I navigate to "Home" in the sidebar
    # Recent chat items: find by TreeWalker, filter x > 200, coordinate click
    And I click on a chat in the recent chats list
    Then I take a screenshot to verify:
      - The conversation loaded with original messages visible
      - The chat name is highlighted in the sidebar

  Scenario: Add a follow-up message to an existing conversation
    Given I have an open conversation with at least one exchange
    # DUAL CHAT-INPUT: use visible textarea + React native setter
    When I type "Follow-up: what is 2+2?" into the visible chat input and press Enter
    Then the agent should respond within 30 seconds
    And I take a screenshot to verify:
      - The new exchange appears below the previous ones
      - The token count in the bottom bar has increased

  Scenario: Start a new conversation from the sidebar
    # "Start New Chat" is a <span> not <button> — use TreeWalker
    When I click "Start New Chat" in the sidebar
    Then I take a screenshot to verify:
      - A fresh conversation view is shown with "Popular chat topics"
      - The chat input is visible at the bottom
      - Token count is reset to 0.0000
