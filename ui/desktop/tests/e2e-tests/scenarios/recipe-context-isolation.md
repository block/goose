# Import Recipe and Run with Parameters

1. Navigate to Recipes
2. Import recipe from the test-recipe.yaml fixture
3. Verify the recipe appears in the list
4. Run the recipe and accept the Trust and Execute dialog
5. Fill in "42" for the test_param parameter
6. Start the recipe and verify the response contains "42"
7. Navigate to Home and start a new chat
8. Ask "what is the value of test_param" and verify the recipe context is not carried over
