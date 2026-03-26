# Install Extension from Deep Link with Environment Variables

Tests that a deep link requiring environment variables opens the extension modal
with env var fields pre-populated, allows the user to fill in the values, and
saves the extension successfully.

## Steps

1. Wait for the app to load
2. Trigger the Context7 extension install via deep link (`goose://extension?...&env=TEST_ACCESS_TOKEN`), simulating clicking "Install" on the extensions registry page
3. Confirm the installation in the confirmation dialog
4. Verify the extension modal opens (titled "Add custom extension") with the Environment Variables section visible
5. Verify the env var key `TEST_ACCESS_TOKEN` is pre-populated in the form
6. Fill in the env var value field with a test secret value
7. Verify the "Add Extension" submit button becomes enabled (form is now valid)
8. Click "Add Extension" to save the extension
9. Verify the Context7 extension appears in the extensions list
10. Verify the extension count increases to 8 (confirming it was added to Default Extensions)

## Key Behaviors Tested

- Deep links with `env=` parameters redirect to the extension modal instead of installing directly
- The modal pre-populates the env var key from the deep link
- The submit button is disabled until all env var values are filled in
- Submitting the form stores the secret and adds the extension
