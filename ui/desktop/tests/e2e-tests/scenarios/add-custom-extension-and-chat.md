# Add Custom Extension and Chat

1. Navigate to the Extensions page
2. Click "Add custom extension" to open the modal
3. Enter extension name "Running Quotes" and command "node /tmp/goose-e2e/basic-mcp.ts"
4. Submit the form and verify the extension appears in Default Extensions (count increases to 8)
5. Navigate to a new chat session
6. Send "Give me a random quote" and verify the runningQuote tool is invoked
7. Verify the response contains a quote from one of the known authors in the extension
