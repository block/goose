# Install Playwright Extension via Registry Deep Link and Search Google

1. Wait for the app to load
2. Trigger the Playwright extension install via the registry deep link (`goose://extension?...`), simulating clicking "Install" on the extensions registry page
3. Confirm the installation in the confirmation dialog
4. Verify the Playwright extension appears in Default Extensions (count increases to 8)
5. Navigate to a new chat session
6. Send a message asking Goose to use Playwright to navigate to Google and search for cats
7. Verify that the `browser_navigate` tool is invoked and Google is referenced in the response
8. Verify that the Google search results for "cat" are rendered back in the chat — confirms the Playwright browser actually loaded the page and returned content containing "cat"
