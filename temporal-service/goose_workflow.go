package main

import (
	"context"
	"encoding/base64"
	"encoding/json"
	"fmt"
	"log"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"strings"
	"time"

	"go.temporal.io/sdk/activity"
	"go.temporal.io/sdk/workflow"
	"go.temporal.io/sdk/temporal"
	"gopkg.in/yaml.v2"
)

// Recipe represents the structure we need from recipe files
type Recipe struct {
	Title        string  `json:"title" yaml:"title"`
	Description  string  `json:"description" yaml:"description"`
	Instructions *string `json:"instructions" yaml:"instructions"`
	Prompt       *string `json:"prompt" yaml:"prompt"`
}

// Workflow definition for executing Goose recipes
func GooseJobWorkflow(ctx workflow.Context, jobID, recipePath string) (string, error) {
	logger := workflow.GetLogger(ctx)
	logger.Info("Starting Goose job workflow", "jobID", jobID, "recipePath", recipePath)

	ao := workflow.ActivityOptions{
		StartToCloseTimeout: 2 * time.Hour, // Allow up to 2 hours for job execution
		RetryPolicy: &temporal.RetryPolicy{
			InitialInterval:        time.Second,
			BackoffCoefficient:     2.0,
			MaximumInterval:        time.Minute,
			MaximumAttempts:        3,
			NonRetryableErrorTypes: []string{"InvalidRecipeError"},
		},
	}
	ctx = workflow.WithActivityOptions(ctx, ao)

	var sessionID string
	err := workflow.ExecuteActivity(ctx, ExecuteGooseRecipe, jobID, recipePath).Get(ctx, &sessionID)
	if err != nil {
		logger.Error("Goose job workflow failed", "jobID", jobID, "error", err)
		return "", err
	}

	logger.Info("Goose job workflow completed", "jobID", jobID, "sessionID", sessionID)
	return sessionID, nil
}

// Activity definition for executing Goose recipes with proper cancellation handling
func ExecuteGooseRecipe(ctx context.Context, jobID, recipePath string) (string, error) {
	logger := activity.GetLogger(ctx)
	logger.Info("Executing Goose recipe", "jobID", jobID, "recipePath", recipePath)

	// Mark job as running at the start
	if globalService != nil {
		globalService.markJobAsRunning(jobID)
		// Ensure we mark it as not running when we're done
		defer globalService.markJobAsNotRunning(jobID)
	}

	// Resolve the actual recipe path (might be embedded in metadata)
	actualRecipePath, err := resolveRecipePath(jobID, recipePath)
	if err != nil {
		return "", temporal.NewNonRetryableApplicationError(
			fmt.Sprintf("failed to resolve recipe: %v", err),
			"InvalidRecipeError",
			err,
		)
	}

	// Check if recipe file exists
	if _, err := os.Stat(actualRecipePath); os.IsNotExist(err) {
		return "", temporal.NewNonRetryableApplicationError(
			fmt.Sprintf("recipe file not found: %s", actualRecipePath),
			"InvalidRecipeError",
			err,
		)
	}

	// Create a cancellable context for the subprocess
	subCtx, cancel := context.WithCancel(ctx)
	defer cancel()

	// Monitor for activity cancellation
	go func() {
		select {
		case <-ctx.Done():
			logger.Info("Activity cancelled, killing process for job", "jobID", jobID)
			globalProcessManager.KillProcess(jobID)
		case <-subCtx.Done():
			// Normal completion
		}
	}()

	// Check if this is a foreground job
	if isForegroundJob(actualRecipePath) {
		logger.Info("Executing foreground job with cancellation support", "jobID", jobID)
		return executeForegroundJobWithCancellation(subCtx, jobID, actualRecipePath)
	}

	// For background jobs, execute with cancellation support
	logger.Info("Executing background job with cancellation support", "jobID", jobID)
	return executeBackgroundJobWithCancellation(subCtx, jobID, actualRecipePath)
}

// resolveRecipePath resolves the actual recipe path, handling embedded recipes
func resolveRecipePath(jobID, recipePath string) (string, error) {
	// If the recipe path exists as-is, use it
	if _, err := os.Stat(recipePath); err == nil {
		return recipePath, nil
	}

	// Try to get embedded recipe content from schedule metadata
	if globalService != nil {
		if recipeContent, err := globalService.getEmbeddedRecipeContent(jobID); err == nil && recipeContent != "" {
			// Create a temporary file with the embedded content
			tempPath := filepath.Join(globalService.recipesDir, fmt.Sprintf("%s-temp.yaml", jobID))
			if err := os.WriteFile(tempPath, []byte(recipeContent), 0644); err != nil {
				return "", fmt.Errorf("failed to write temporary recipe file: %w", err)
			}
			log.Printf("Created temporary recipe file for job %s: %s", jobID, tempPath)
			return tempPath, nil
		}
	}

	// If no embedded content and original path doesn't exist, return error
	return "", fmt.Errorf("recipe not found: %s (and no embedded content available)", recipePath)
}

// executeBackgroundJobWithCancellation handles background job execution with proper process management
func executeBackgroundJobWithCancellation(ctx context.Context, jobID, recipePath string) (string, error) {
	log.Printf("Executing background job %s using recipe file: %s", jobID, recipePath)

	// Find the goose CLI binary
	goosePath, err := findGooseBinary()
	if err != nil {
		return "", fmt.Errorf("failed to find goose CLI binary: %w", err)
	}

	// Generate session name for this scheduled job
	sessionName := fmt.Sprintf("scheduled-%s", jobID)

	// Create command with context for cancellation
	cmd := exec.CommandContext(ctx, goosePath, "run",
		"--recipe", recipePath,
		"--name", sessionName,
		"--scheduled-job-id", jobID,
	)

	// Set up process group for proper cleanup
	configureSysProcAttr(cmd)

	// Set up environment
	cmd.Env = append(os.Environ(),
		fmt.Sprintf("GOOSE_JOB_ID=%s", jobID),
	)

	log.Printf("Starting background CLI job %s with session %s", jobID, sessionName)

	// Start the process
	if err := cmd.Start(); err != nil {
		return "", fmt.Errorf("failed to start background CLI execution: %w", err)
	}

	// Register the process with the process manager
	_, cancel := context.WithCancel(ctx)
	globalProcessManager.AddProcess(jobID, cmd.Process, cancel)

	// Ensure cleanup
	defer func() {
		globalProcessManager.RemoveProcess(jobID)
		cancel()
	}()

	// Wait for completion or cancellation
	done := make(chan error, 1)
	go func() {
		done <- cmd.Wait()
	}()

	select {
	case <-ctx.Done():
		// Context cancelled - kill the process
		log.Printf("Background job %s cancelled, killing process", jobID)
		globalProcessManager.KillProcess(jobID)
		return "", ctx.Err()
	case err := <-done:
		if err != nil {
			log.Printf("Background CLI job %s failed: %v", jobID, err)
			return "", fmt.Errorf("background CLI execution failed: %w", err)
		}
		log.Printf("Background CLI job %s completed successfully with session %s", jobID, sessionName)
		return sessionName, nil
	}
}

// executeForegroundJobWithCancellation handles foreground job execution with proper process management
func executeForegroundJobWithCancellation(ctx context.Context, jobID, recipePath string) (string, error) {
	log.Printf("Executing foreground job %s with recipe %s", jobID, recipePath)

	// Parse the recipe file first
	recipe, err := parseRecipeFile(recipePath)
	if err != nil {
		return "", fmt.Errorf("failed to parse recipe file: %w", err)
	}

	// Check if desktop app is running
	if isDesktopAppRunning() {
		log.Printf("Desktop app is running, using GUI mode for job %s", jobID)
		return executeForegroundJobGUIWithCancellation(ctx, jobID, recipe)
	}

	// Desktop app not running, fall back to CLI
	log.Printf("Desktop app not running, falling back to CLI mode for job %s", jobID)
	return executeForegroundJobCLIWithCancellation(ctx, jobID, recipe, recipePath)
}

// executeForegroundJobGUIWithCancellation handles GUI execution with cancellation
func executeForegroundJobGUIWithCancellation(ctx context.Context, jobID string, recipe *Recipe) (string, error) {
	// Generate session name for this scheduled job
	sessionName := fmt.Sprintf("scheduled-%s", jobID)

	// Generate deep link with session name
	deepLink, err := generateDeepLink(recipe, jobID, sessionName)
	if err != nil {
		return "", fmt.Errorf("failed to generate deep link: %w", err)
	}

	// Open the deep link
	if err := openDeepLink(deepLink); err != nil {
		return "", fmt.Errorf("failed to open deep link: %w", err)
	}
	
	log.Printf("Foreground GUI job %s initiated with session %s, waiting for completion...", jobID, sessionName)

	// Wait for session completion with cancellation support
	err = waitForSessionCompletionWithCancellation(ctx, sessionName, 2*time.Hour)
	if err != nil {
		if ctx.Err() != nil {
			log.Printf("GUI session %s cancelled", sessionName)
			return "", ctx.Err()
		}
		return "", fmt.Errorf("GUI session failed or timed out: %w", err)
	}
	
	log.Printf("Foreground GUI job %s completed successfully with session %s", jobID, sessionName)
	return sessionName, nil
}

// executeForegroundJobCLIWithCancellation handles CLI execution with cancellation
func executeForegroundJobCLIWithCancellation(ctx context.Context, jobID string, recipe *Recipe, recipePath string) (string, error) {
	log.Printf("Executing job %s via CLI fallback using recipe file: %s", jobID, recipePath)
	// Find the goose CLI binary
	goosePath, err := findGooseBinary()
	if err != nil {
		return "", fmt.Errorf("failed to find goose CLI binary: %w", err)
	}

	// Generate session name for this scheduled job
	sessionName := fmt.Sprintf("scheduled-%s", jobID)
	// Create command with context for cancellation
	cmd := exec.CommandContext(ctx, goosePath, "run",
		"--recipe", recipePath,
		"--name", sessionName,
		"--scheduled-job-id", jobID,
	)

	// Set up process group for proper cleanup
	configureSysProcAttr(cmd)

	// Set up environment
	cmd.Env = append(os.Environ(),
		fmt.Sprintf("GOOSE_JOB_ID=%s", jobID),
	)
	
	log.Printf("Starting foreground CLI job %s with session %s", jobID, sessionName)

	// Start the process
	if err := cmd.Start(); err != nil {
		return "", fmt.Errorf("failed to start foreground CLI execution: %w", err)
	}

	// Register the process with the process manager
	_, cancel := context.WithCancel(ctx)
	globalProcessManager.AddProcess(jobID, cmd.Process, cancel)

	// Ensure cleanup
	defer func() {
		globalProcessManager.RemoveProcess(jobID)
		cancel()
	}()

	// Wait for completion or cancellation
	done := make(chan error, 1)
	go func() {
		done <- cmd.Wait()
	}()
	
	select {
	case <-ctx.Done():
		// Context cancelled - kill the process
		log.Printf("Foreground CLI job %s cancelled, killing process", jobID)
		globalProcessManager.KillProcess(jobID)
		return "", ctx.Err()
	case err := <-done:
		if err != nil {
			log.Printf("Foreground CLI job %s failed: %v", jobID, err)
			return "", fmt.Errorf("foreground CLI execution failed: %w", err)
		}
		log.Printf("Foreground CLI job %s completed successfully with session %s", jobID, sessionName)
		return sessionName, nil
	}
}

// findGooseBinary locates the goose CLI binary
func findGooseBinary() (string, error) {
	// Try different possible locations
	possiblePaths := []string{
		"goose",           // In PATH
		"./goose",         // Current directory
		"../goose",        // Parent directory
	}

	// Also try relative to the current executable
	if exePath, err := os.Executable(); err == nil {
		exeDir := filepath.Dir(exePath)
		possiblePaths = append(possiblePaths,
			filepath.Join(exeDir, "goose"),
			filepath.Join(exeDir, "..", "goose"),
		)
	}

	for _, path := range possiblePaths {
		if _, err := exec.LookPath(path); err == nil {
			return path, nil
		}
		// Also check if file exists directly
		if _, err := os.Stat(path); err == nil {
			return path, nil
		}
	}

	return "", fmt.Errorf("goose CLI binary not found in any of: %v", possiblePaths)
}

// isDesktopAppRunning checks if the Goose desktop app is currently running
func isDesktopAppRunning() bool {
	log.Println("Checking if desktop app is running...")

	var cmd *exec.Cmd
	switch runtime.GOOS {
	case "darwin":
		cmd = exec.Command("pgrep", "-f", "Goose.app")
	case "windows":
		cmd = exec.Command("tasklist", "/FI", "IMAGENAME eq Goose.exe")
	case "linux":
		cmd = exec.Command("pgrep", "-f", "goose")
	default:
		log.Printf("Unsupported OS: %s", runtime.GOOS)
		return false
	}

	output, err := cmd.Output()
	if err != nil {
		log.Printf("Failed to check if desktop app is running: %v", err)
		return false
	}

	var isRunning bool
	switch runtime.GOOS {
	case "darwin", "linux":
		isRunning = len(output) > 0
	case "windows":
		isRunning = strings.Contains(string(output), "Goose.exe")
	}

	log.Printf("Desktop app running: %v", isRunning)
	return isRunning
}

// parseRecipeFile parses a recipe file (YAML or JSON)
func parseRecipeFile(recipePath string) (*Recipe, error) {
	content, err := os.ReadFile(recipePath)
	if err != nil {
		return nil, err
	}

	var recipe Recipe

	// Try YAML first, then JSON
	if err := yaml.Unmarshal(content, &recipe); err != nil {
		if err := json.Unmarshal(content, &recipe); err != nil {
			return nil, fmt.Errorf("failed to parse as YAML or JSON: %w", err)
		}
	}

	return &recipe, nil
}

// generateDeepLink creates a deep link for the recipe with session name
func generateDeepLink(recipe *Recipe, jobID, sessionName string) (string, error) {
	// Create the recipe config for the deep link
	recipeConfig := map[string]interface{}{
		"id":           jobID,
		"title":        recipe.Title,
		"description":  recipe.Description,
		"instructions": recipe.Instructions,
		"activities":   []string{}, // Empty activities array
		"prompt":       recipe.Prompt,
		"sessionName":  sessionName, // Include session name for proper tracking
	}

	// Encode the config as JSON then base64
	configJSON, err := json.Marshal(recipeConfig)
	if err != nil {
		return "", err
	}

	configBase64 := base64.StdEncoding.EncodeToString(configJSON)

	// Create the deep link URL with scheduled job ID parameter
	deepLink := fmt.Sprintf("goose://recipe?config=%s&scheduledJob=%s", configBase64, jobID)

	log.Printf("Generated deep link for job %s with session %s (length: %d)", jobID, sessionName, len(deepLink))
	return deepLink, nil
}

// openDeepLink opens a deep link using the system's default protocol handler
func openDeepLink(deepLink string) error {
	log.Printf("Opening deep link: %s", deepLink)

	var cmd *exec.Cmd
	switch runtime.GOOS {
	case "darwin":
		cmd = exec.Command("open", deepLink)
	case "windows":
		cmd = exec.Command("cmd", "/c", "start", "", deepLink)
	case "linux":
		cmd = exec.Command("xdg-open", deepLink)
	default:
		return fmt.Errorf("unsupported OS: %s", runtime.GOOS)
	}

	if err := cmd.Start(); err != nil {
		return fmt.Errorf("failed to open deep link: %w", err)
	}

	log.Println("Deep link opened successfully")
	return nil
}

// waitForSessionCompletionWithCancellation polls for session completion with cancellation support
func waitForSessionCompletionWithCancellation(ctx context.Context, sessionName string, timeout time.Duration) error {
	log.Printf("Waiting for session %s to complete (timeout: %v)", sessionName, timeout)

	start := time.Now()
	ticker := time.NewTicker(10 * time.Second) // Check every 10 seconds
	defer ticker.Stop()

	timeoutCtx, cancel := context.WithTimeout(ctx, timeout)
	defer cancel()

	for {
		select {
		case <-timeoutCtx.Done():
			if timeoutCtx.Err() == context.DeadlineExceeded {
				return fmt.Errorf("session %s timed out after %v", sessionName, timeout)
			}
			return timeoutCtx.Err() // Cancelled
		case <-ticker.C:
			elapsed := time.Since(start)
			log.Printf("Checking session %s status (elapsed: %v)", sessionName, elapsed)

			// Check if session exists and is complete
			complete, err := isSessionComplete(sessionName)
			if err != nil {
				log.Printf("Error checking session %s status: %v", sessionName, err)
				// Continue polling - session might not be created yet
				continue
			}

			if complete {
				log.Printf("Session %s completed after %v", sessionName, elapsed)
				return nil
			}

			log.Printf("Session %s still running (elapsed: %v)", sessionName, elapsed)
		}
	}
}

// isSessionComplete checks if a session is complete by querying the Goose sessions API
func isSessionComplete(sessionName string) (bool, error) {
	// Try to find the goose CLI binary to query session status
	goosePath, err := findGooseBinary()
	if err != nil {
		return false, fmt.Errorf("failed to find goose CLI binary: %w", err)
	}

	// Use goose CLI to list sessions and check if our session exists and is complete
	cmd := exec.Command(goosePath, "sessions", "list", "--format", "json")

	output, err := cmd.Output()
	if err != nil {
		return false, fmt.Errorf("failed to list sessions: %w", err)
	}

	// Parse the JSON output to find our session
	var sessions []map[string]interface{}
	if err := json.Unmarshal(output, &sessions); err != nil {
		return false, fmt.Errorf("failed to parse sessions JSON: %w", err)
	}

	// Look for our session by name
	for _, session := range sessions {
		if name, ok := session["name"].(string); ok && name == sessionName {
			// Session exists, check if it's complete
			// A session is considered complete if it's not currently active
			// We can check this by looking for an "active" field or similar
			if active, ok := session["active"].(bool); ok {
				return !active, nil // Complete if not active
			}

			// If no active field, check for completion indicators
			// This might vary based on the actual Goose CLI output format
			if status, ok := session["status"].(string); ok {
				return status == "completed" || status == "finished" || status == "done", nil
			}

			// If we found the session but can't determine status, assume it's still running
			return false, nil
		}
	}

	// Session not found - it might not be created yet, so not complete
	return false, nil
}

// isForegroundJob checks if a recipe is configured for foreground execution
func isForegroundJob(recipePath string) bool {
	// Simple struct to just check the schedule.foreground field
	type ScheduleConfig struct {
		Foreground bool `json:"foreground" yaml:"foreground"`
	}
	type MinimalRecipe struct {
		Schedule *ScheduleConfig `json:"schedule" yaml:"schedule"`
	}

	content, err := os.ReadFile(recipePath)
	if err != nil {
		return false // Default to background if we can't read
	}

	var recipe MinimalRecipe

	// Try YAML first, then JSON
	if err := yaml.Unmarshal(content, &recipe); err != nil {
		if err := json.Unmarshal(content, &recipe); err != nil {
			return false // Default to background if we can't parse
		}
	}

	return recipe.Schedule != nil && recipe.Schedule.Foreground
}