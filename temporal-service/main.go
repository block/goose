package main

import (
	"context"
	"encoding/base64"
	"encoding/json"
	"fmt"
	"log"
	"net"
	"net/http"
	"os"
	"os/exec"
	"os/signal"
	"path/filepath"
	"runtime"
	"strconv"
	"strings"
	"sync"
	"syscall"
	"time"

	"go.temporal.io/api/workflowservice/v1"
	"go.temporal.io/sdk/activity"
	"go.temporal.io/sdk/client"
	"go.temporal.io/sdk/temporal"
	"go.temporal.io/sdk/worker"
	"go.temporal.io/sdk/workflow"
	"gopkg.in/yaml.v2"
)

const (
	TaskQueueName = "goose-task-queue"
	Namespace     = "default"
)

// PortConfig holds the port configuration for Temporal services
type PortConfig struct {
	TemporalPort int // Main Temporal server port
	UIPort       int // Temporal UI port
	HTTPPort     int // HTTP API port
}

// getManagedRecipesDir returns the proper directory for storing managed recipes
func getManagedRecipesDir() (string, error) {
	var baseDir string
	
	switch runtime.GOOS {
	case "darwin":
		// macOS: ~/Library/Application Support/temporal/managed-recipes
		homeDir, err := os.UserHomeDir()
		if err != nil {
			return "", fmt.Errorf("failed to get user home directory: %w", err)
		}
		baseDir = filepath.Join(homeDir, "Library", "Application Support", "temporal", "managed-recipes")
	case "linux":
		// Linux: ~/.local/share/temporal/managed-recipes
		homeDir, err := os.UserHomeDir()
		if err != nil {
			return "", fmt.Errorf("failed to get user home directory: %w", err)
		}
		baseDir = filepath.Join(homeDir, ".local", "share", "temporal", "managed-recipes")
	case "windows":
		// Windows: %APPDATA%\temporal\managed-recipes
		appDataDir := os.Getenv("APPDATA")
		if appDataDir == "" {
			homeDir, err := os.UserHomeDir()
			if err != nil {
				return "", fmt.Errorf("failed to get user home directory: %w", err)
			}
			appDataDir = filepath.Join(homeDir, "AppData", "Roaming")
		}
		baseDir = filepath.Join(appDataDir, "temporal", "managed-recipes")
	default:
		// Fallback for unknown OS
		homeDir, err := os.UserHomeDir()
		if err != nil {
			return "", fmt.Errorf("failed to get user home directory: %w", err)
		}
		baseDir = filepath.Join(homeDir, ".local", "share", "temporal", "managed-recipes")
	}
	
	return baseDir, nil
}

// findAvailablePort finds an available port starting from the given port
func findAvailablePort(startPort int) (int, error) {
	for port := startPort; port < startPort+100; port++ {
		ln, err := net.Listen("tcp", fmt.Sprintf(":%d", port))
		if err == nil {
			ln.Close()
			return port, nil
		}
	}
	return 0, fmt.Errorf("no available port found starting from %d", startPort)
}

// findAvailablePorts finds available ports for all Temporal services
func findAvailablePorts() (*PortConfig, error) {
	// Try to find available ports starting from preferred defaults
	temporalPort, err := findAvailablePort(7233)
	if err != nil {
		return nil, fmt.Errorf("failed to find available port for Temporal server: %w", err)
	}

	uiPort, err := findAvailablePort(8233)
	if err != nil {
		return nil, fmt.Errorf("failed to find available port for Temporal UI: %w", err)
	}

	// For HTTP port, check environment variable first
	httpPort := 8080
	if portEnv := os.Getenv("PORT"); portEnv != "" {
		if parsed, err := strconv.Atoi(portEnv); err == nil {
			httpPort = parsed
		}
	}

	// Verify HTTP port is available, find alternative if not
	finalHTTPPort, err := findAvailablePort(httpPort)
	if err != nil {
		return nil, fmt.Errorf("failed to find available port for HTTP server: %w", err)
	}

	return &PortConfig{
		TemporalPort: temporalPort,
		UIPort:       uiPort,
		HTTPPort:     finalHTTPPort,
	}, nil
}

// Global service instance for activities to access
var globalService *TemporalService

// ProcessManager tracks and manages spawned processes
type ProcessManager struct {
	processes map[string]*ManagedProcess
	mutex     sync.RWMutex
}

type ManagedProcess struct {
	JobID     string
	Process   *os.Process
	Cancel    context.CancelFunc
	StartTime time.Time
}

var globalProcessManager = &ProcessManager{
	processes: make(map[string]*ManagedProcess),
}

func (pm *ProcessManager) AddProcess(jobID string, process *os.Process, cancel context.CancelFunc) {
	pm.mutex.Lock()
	defer pm.mutex.Unlock()

	pm.processes[jobID] = &ManagedProcess{
		JobID:     jobID,
		Process:   process,
		Cancel:    cancel,
		StartTime: time.Now(),
	}
	log.Printf("Added process %d for job %s to process manager", process.Pid, jobID)
}

func (pm *ProcessManager) RemoveProcess(jobID string) {
	pm.mutex.Lock()
	defer pm.mutex.Unlock()

	if mp, exists := pm.processes[jobID]; exists {
		log.Printf("Removed process %d for job %s from process manager", mp.Process.Pid, jobID)
		delete(pm.processes, jobID)
	}
}

func (pm *ProcessManager) KillProcess(jobID string) error {
	pm.mutex.Lock()
	defer pm.mutex.Unlock()

	mp, exists := pm.processes[jobID]
	if !exists {
		return fmt.Errorf("no process found for job %s", jobID)
	}

	log.Printf("Killing process %d for job %s", mp.Process.Pid, jobID)

	// Cancel the context first
	if mp.Cancel != nil {
		mp.Cancel()
	}

	// Kill the process and its children
	if err := killProcessGroup(mp.Process); err != nil {
		log.Printf("Error killing process group for job %s: %v", jobID, err)
		return err
	}

	delete(pm.processes, jobID)
	return nil
}

func (pm *ProcessManager) KillAllProcesses() {
	pm.mutex.Lock()
	defer pm.mutex.Unlock()

	log.Printf("Killing all %d managed processes", len(pm.processes))

	for jobID, mp := range pm.processes {
		log.Printf("Killing process %d for job %s", mp.Process.Pid, jobID)

		if mp.Cancel != nil {
			mp.Cancel()
		}

		if err := killProcessGroup(mp.Process); err != nil {
			log.Printf("Error killing process group for job %s: %v", jobID, err)
		}
	}

	pm.processes = make(map[string]*ManagedProcess)
}

func (pm *ProcessManager) ListProcesses() map[string]*ManagedProcess {
	pm.mutex.RLock()
	defer pm.mutex.RUnlock()

	result := make(map[string]*ManagedProcess)
	for k, v := range pm.processes {
		result[k] = v
	}
	return result
}

// killProcessGroup kills a process and all its children
func killProcessGroup(process *os.Process) error {
	if process == nil {
		return nil
	}

	pid := process.Pid
	log.Printf("Attempting to kill process group for PID %d", pid)

	switch runtime.GOOS {
	case "windows":
		// On Windows, kill the process tree
		cmd := exec.Command("taskkill", "/F", "/T", "/PID", fmt.Sprintf("%d", pid))
		if err := cmd.Run(); err != nil {
			log.Printf("Failed to kill Windows process tree for PID %d: %v", pid, err)
			return err
		}
		log.Printf("Successfully killed Windows process tree for PID %d", pid)
		return nil
	default:
		// On Unix-like systems, kill the process group more aggressively
		log.Printf("Killing Unix process group for PID %d", pid)
		
		// First, try to kill the entire process group with SIGTERM
		if err := syscall.Kill(-pid, syscall.SIGTERM); err != nil {
			log.Printf("Failed to send SIGTERM to process group -%d: %v", pid, err)
		} else {
			log.Printf("Sent SIGTERM to process group -%d", pid)
		}
		
		// Also try to kill the main process directly
		if err := syscall.Kill(pid, syscall.SIGTERM); err != nil {
			log.Printf("Failed to send SIGTERM to process %d: %v", pid, err)
		} else {
			log.Printf("Sent SIGTERM to process %d", pid)
		}

		// Give processes a brief moment to terminate gracefully
		time.Sleep(1 * time.Second)

		// Force kill the process group with SIGKILL
		if err := syscall.Kill(-pid, syscall.SIGKILL); err != nil {
			log.Printf("Failed to send SIGKILL to process group -%d: %v", pid, err)
		} else {
			log.Printf("Sent SIGKILL to process group -%d", pid)
		}
		
		// Force kill the main process with SIGKILL
		if err := syscall.Kill(pid, syscall.SIGKILL); err != nil {
			log.Printf("Failed to send SIGKILL to process %d: %v", pid, err)
		} else {
			log.Printf("Sent SIGKILL to process %d", pid)
		}

		// Also try using the process.Kill() method as a fallback
		if err := process.Kill(); err != nil {
			log.Printf("Failed to kill process using process.Kill(): %v", err)
		} else {
			log.Printf("Successfully killed process using process.Kill()")
		}

		log.Printf("Completed kill attempts for process group %d", pid)
		return nil
	}
}

// Request/Response types for HTTP API
type JobRequest struct {
	Action       string `json:"action"`      // create, delete, pause, unpause, list, run_now, kill_job, update
	JobID        string `json:"job_id"`
	CronExpr     string `json:"cron"`
	RecipePath   string `json:"recipe_path"`
	ExecutionMode string `json:"execution_mode,omitempty"` // "foreground" or "background"
}

type JobResponse struct {
	Success bool        `json:"success"`
	Message string      `json:"message"`
	Jobs    []JobStatus `json:"jobs,omitempty"`
	Data    interface{} `json:"data,omitempty"`
}

type JobStatus struct {
	ID               string    `json:"id"`
	CronExpr         string    `json:"cron"`
	RecipePath       string    `json:"recipe_path"`
	LastRun          *string   `json:"last_run,omitempty"`
	NextRun          *string   `json:"next_run,omitempty"`
	CurrentlyRunning bool      `json:"currently_running"`
	Paused           bool      `json:"paused"`
	CreatedAt        time.Time `json:"created_at"`
	ExecutionMode    *string   `json:"execution_mode,omitempty"` // "foreground" or "background"
	LastManualRun    *string   `json:"last_manual_run,omitempty"` // Track manual runs separately
}

type RunNowResponse struct {
	SessionID string `json:"session_id"`
}

// ensureTemporalServerRunning checks if Temporal server is running and starts it if needed
func ensureTemporalServerRunning(ports *PortConfig) error {
	log.Println("Checking if Temporal server is running...")
	
	// Check if Temporal server is already running by trying to connect
	if isTemporalServerRunning(ports.TemporalPort) {
		log.Printf("Temporal server is already running on port %d", ports.TemporalPort)
		return nil
	}
	
	log.Printf("Temporal server not running, attempting to start it on port %d...", ports.TemporalPort)
	
	// Find the temporal CLI binary
	temporalCmd, err := findTemporalCLI()
	if err != nil {
		log.Printf("ERROR: Could not find temporal CLI: %v", err)
		return fmt.Errorf("could not find temporal CLI: %w", err)
	}
	
	log.Printf("Using Temporal CLI at: %s", temporalCmd)
	
	// Start Temporal server in background
	args := []string{"server", "start-dev",
		"--db-filename", "temporal.db", 
		"--port", strconv.Itoa(ports.TemporalPort),
		"--ui-port", strconv.Itoa(ports.UIPort),
		"--log-level", "warn"}

	log.Printf("Starting Temporal server with command: %s %v", temporalCmd, args)

	cmd := exec.Command(temporalCmd, args...)
	
	// Properly detach the process so it survives when the parent exits
	cmd.SysProcAttr = &syscall.SysProcAttr{
		Setpgid: true,  // Create new process group
		Pgid:    0,     // Use process ID as group ID
	}

	// Redirect stdin/stdout/stderr to avoid hanging
	cmd.Stdin = nil
	cmd.Stdout = nil
	cmd.Stderr = nil

	// Start the process
	if err := cmd.Start(); err != nil {
		log.Printf("ERROR: Failed to start Temporal server: %v", err)
		return fmt.Errorf("failed to start Temporal server: %w", err)
	}
	
	log.Printf("Temporal server started with PID: %d (port: %d, UI port: %d)",
		cmd.Process.Pid, ports.TemporalPort, ports.UIPort)
	
	// Wait for server to be ready (with timeout)
	log.Println("Waiting for Temporal server to be ready...")
	timeout := time.After(30 * time.Second)
	ticker := time.NewTicker(2 * time.Second)
	defer ticker.Stop()
	
	attemptCount := 0
	for {
		select {
		case <-timeout:
			log.Printf("ERROR: Timeout waiting for Temporal server to start after %d attempts", attemptCount)
			return fmt.Errorf("timeout waiting for Temporal server to start")
		case <-ticker.C:
			attemptCount++
			log.Printf("Checking if Temporal server is ready (attempt %d)...", attemptCount)
			if isTemporalServerRunning(ports.TemporalPort) {
				log.Printf("Temporal server is now ready on port %d", ports.TemporalPort)
				return nil
			} else {
				log.Printf("Temporal server not ready yet (attempt %d)", attemptCount)
			}
		}
	}
}

// isTemporalServerRunning checks if Temporal server is accessible
func isTemporalServerRunning(port int) bool {
	// Try to create a client connection to check if server is running
	c, err := client.Dial(client.Options{
		HostPort:  fmt.Sprintf("127.0.0.1:%d", port),
		Namespace: Namespace,
	})
	if err != nil {
		return false
	}
	defer c.Close()
	
	// Try a simple operation to verify the connection works
	ctx, cancel := context.WithTimeout(context.Background(), 2*time.Second)
	defer cancel()
	
	_, err = c.WorkflowService().GetSystemInfo(ctx, &workflowservice.GetSystemInfoRequest{})
	return err == nil
}

// findTemporalCLI attempts to find the temporal CLI binary
func findTemporalCLI() (string, error) {
	log.Println("Looking for temporal CLI binary...")

	// First, try to find temporal in PATH using exec.LookPath
	log.Println("Checking PATH for temporal CLI...")
	if path, err := exec.LookPath("temporal"); err == nil {
		log.Printf("Found temporal in PATH at: %s", path)
		// Verify it's the correct temporal CLI by checking version
		log.Println("Verifying temporal CLI version...")
		cmd := exec.Command(path, "--version")
		if err := cmd.Run(); err == nil {
			log.Printf("Successfully verified temporal CLI at: %s", path)
			return path, nil
		} else {
			log.Printf("Failed to verify temporal CLI at %s: %v", path, err)
		}
	} else {
		log.Printf("temporal not found in PATH: %v", err)
	}

	// Try using 'which' command to find temporal
	cmd := exec.Command("which", "temporal")
	if output, err := cmd.Output(); err == nil {
		path := strings.TrimSpace(string(output))
		if path != "" {
			// Verify it's the correct temporal CLI by checking version
			cmd := exec.Command(path, "--version")
			if err := cmd.Run(); err == nil {
				return path, nil
			}
		}
	}

	// If not found in PATH, try different possible locations for the temporal CLI
	log.Println("Checking bundled/local locations for temporal CLI...")
	possiblePaths := []string{
		"./temporal",         // Current directory
	}
	
	// Also try relative to the current executable (most important for bundled apps)
	if exePath, err := os.Executable(); err == nil {
		exeDir := filepath.Dir(exePath)
		log.Printf("Executable directory: %s", exeDir)
		additionalPaths := []string{
			filepath.Join(exeDir, "temporal"),
			filepath.Join(exeDir, "temporal.exe"), // Windows
			// Also try one level up (for development)
			filepath.Join(exeDir, "..", "temporal"),
			filepath.Join(exeDir, "..", "temporal.exe"),
		}
		possiblePaths = append(possiblePaths, additionalPaths...)
		log.Printf("Will check these additional paths: %v", additionalPaths)
	} else {
		log.Printf("Failed to get executable path: %v", err)
	}

	log.Printf("Checking %d possible paths for temporal CLI", len(possiblePaths))

	// Test each possible path
	for i, path := range possiblePaths {
		log.Printf("Checking path %d/%d: %s", i+1, len(possiblePaths), path)
		if _, err := os.Stat(path); err == nil {
			log.Printf("File exists at: %s", path)
			// File exists, test if it's executable and the right binary
			cmd := exec.Command(path, "--version")
			if err := cmd.Run(); err == nil {
				log.Printf("Successfully verified temporal CLI at: %s", path)
				return path, nil
			} else {
				log.Printf("Failed to verify temporal CLI at %s: %v", path, err)
			}
		} else {
			log.Printf("File does not exist at %s: %v", path, err)
		}
	}
	
	return "", fmt.Errorf("temporal CLI not found in PATH or any of the expected locations: %v", possiblePaths)
}

// TemporalService manages the Temporal client and provides HTTP API
type TemporalService struct {
	client       client.Client
	worker       worker.Worker
	scheduleJobs map[string]*JobStatus // In-memory job tracking
	runningJobs  map[string]bool       // Track which jobs are currently running
	runningWorkflows map[string][]string // Track workflow IDs for each job
	recipesDir   string                  // Directory for managed recipe storage
	ports        *PortConfig           // Port configuration
}

// NewTemporalService creates a new Temporal service and ensures Temporal server is running
func NewTemporalService() (*TemporalService, error) {
	// First, find available ports
	ports, err := findAvailablePorts()
	if err != nil {
		return nil, fmt.Errorf("failed to find available ports: %w", err)
	}

	log.Printf("Using ports - Temporal: %d, UI: %d, HTTP: %d",
		ports.TemporalPort, ports.UIPort, ports.HTTPPort)

	// Ensure Temporal server is running
	if err := ensureTemporalServerRunning(ports); err != nil {
		return nil, fmt.Errorf("failed to ensure Temporal server is running: %w", err)
	}

	// Set up managed recipes directory in user data directory
	recipesDir, err := getManagedRecipesDir()
	if err != nil {
		return nil, fmt.Errorf("failed to determine managed recipes directory: %w", err)
	}
	if err := os.MkdirAll(recipesDir, 0755); err != nil {
		return nil, fmt.Errorf("failed to create managed recipes directory: %w", err)
	}
	log.Printf("Using managed recipes directory: %s", recipesDir)

	// Create client (Temporal server should now be running)
	c, err := client.Dial(client.Options{
		HostPort:  fmt.Sprintf("127.0.0.1:%d", ports.TemporalPort),
		Namespace: Namespace,
	})
	if err != nil {
		return nil, fmt.Errorf("failed to create temporal client: %w", err)
	}

	// Create worker
	w := worker.New(c, TaskQueueName, worker.Options{})
	w.RegisterWorkflow(GooseJobWorkflow)
	w.RegisterActivity(ExecuteGooseRecipe)

	if err := w.Start(); err != nil {
		c.Close()
		return nil, fmt.Errorf("failed to start worker: %w", err)
	}

	log.Printf("Connected to Temporal server successfully on port %d", ports.TemporalPort)

	service := &TemporalService{
		client:       c,
		worker:       w,
		scheduleJobs: make(map[string]*JobStatus),
		runningJobs:  make(map[string]bool),
		runningWorkflows: make(map[string][]string),
		recipesDir:   recipesDir,
		ports:        ports,
	}
	
	// Set global service for activities
	globalService = service

	return service, nil
}

// Stop gracefully shuts down the Temporal service
func (ts *TemporalService) Stop() {
	log.Println("Shutting down Temporal service...")
	if ts.worker != nil {
		ts.worker.Stop()
	}
	if ts.client != nil {
		ts.client.Close()
	}
	log.Println("Temporal service stopped")
}

// GetHTTPPort returns the HTTP port for this service
func (ts *TemporalService) GetHTTPPort() int {
	return ts.ports.HTTPPort
}

// GetTemporalPort returns the Temporal server port for this service
func (ts *TemporalService) GetTemporalPort() int {
	return ts.ports.TemporalPort
}

// GetUIPort returns the Temporal UI port for this service
func (ts *TemporalService) GetUIPort() int {
	return ts.ports.UIPort
}

// storeRecipeForSchedule copies a recipe file to managed storage and returns the managed path and content
func (ts *TemporalService) storeRecipeForSchedule(jobID, originalPath string) (string, []byte, error) {
	// Validate original recipe file exists
	if _, err := os.Stat(originalPath); os.IsNotExist(err) {
		return "", nil, fmt.Errorf("recipe file not found: %s", originalPath)
	}

	// Read the original recipe content
	recipeContent, err := os.ReadFile(originalPath)
	if err != nil {
		return "", nil, fmt.Errorf("failed to read recipe file: %w", err)
	}

	// Validate it's a valid recipe by trying to parse it
	if _, err := parseRecipeContent(recipeContent); err != nil {
		return "", nil, fmt.Errorf("invalid recipe file: %w", err)
	}

	// Create managed file path
	originalFilename := filepath.Base(originalPath)
	ext := filepath.Ext(originalFilename)
	if ext == "" {
		ext = ".yaml" // Default to yaml if no extension
	}

	managedFilename := fmt.Sprintf("%s%s", jobID, ext)
	managedPath := filepath.Join(ts.recipesDir, managedFilename)

	// Write to managed storage
	if err := os.WriteFile(managedPath, recipeContent, 0644); err != nil {
		return "", nil, fmt.Errorf("failed to write managed recipe file: %w", err)
	}

	log.Printf("Stored recipe for job %s: %s -> %s (size: %d bytes)",
		jobID, originalPath, managedPath, len(recipeContent))

	return managedPath, recipeContent, nil
}

// parseRecipeContent parses recipe content from bytes (YAML or JSON)
func parseRecipeContent(content []byte) (*Recipe, error) {
	var recipe Recipe

	// Try YAML first, then JSON
	if err := yaml.Unmarshal(content, &recipe); err != nil {
		if err := json.Unmarshal(content, &recipe); err != nil {
			return nil, fmt.Errorf("failed to parse as YAML or JSON: %w", err)
		}
	}

	return &recipe, nil
}

// getEmbeddedRecipeContent retrieves embedded recipe content from schedule metadata
func (ts *TemporalService) getEmbeddedRecipeContent(jobID string) (string, error) {
	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()

	scheduleID := fmt.Sprintf("goose-job-%s", jobID)
	handle := ts.client.ScheduleClient().GetHandle(ctx, scheduleID)

	desc, err := handle.Describe(ctx)
	if err != nil {
		return "", fmt.Errorf("failed to get schedule description: %w", err)
	}

	if desc.Schedule.State.Note == "" {
		return "", fmt.Errorf("no metadata found in schedule")
	}

	var metadata map[string]interface{}
	if err := json.Unmarshal([]byte(desc.Schedule.State.Note), &metadata); err != nil {
		return "", fmt.Errorf("failed to parse schedule metadata: %w", err)
	}

	if recipeContent, ok := metadata["recipe_content"].(string); ok {
		return recipeContent, nil
	}

	return "", fmt.Errorf("no embedded recipe content found")
}

// cleanupManagedRecipe removes managed recipe files for a job
func (ts *TemporalService) cleanupManagedRecipe(jobID string) {
	// Clean up both permanent and temporary files
	patterns := []string{
		fmt.Sprintf("%s.*", jobID),      // Permanent files (jobID.yaml, jobID.json, etc.)
		fmt.Sprintf("%s-temp.*", jobID), // Temporary files
	}

	for _, pattern := range patterns {
		matches, err := filepath.Glob(filepath.Join(ts.recipesDir, pattern))
		if err != nil {
			log.Printf("Error finding recipe files for cleanup: %v", err)
			continue
		}

		for _, filePath := range matches {
			if err := os.Remove(filePath); err != nil {
				log.Printf("Warning: Failed to remove recipe file %s: %v", filePath, err)
			} else {
				log.Printf("Cleaned up recipe file: %s", filePath)
			}
		}
	}
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

// HTTP API handlers

func (ts *TemporalService) handleJobs(w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Content-Type", "application/json")

	if r.Method != http.MethodPost {
		ts.writeErrorResponse(w, http.StatusMethodNotAllowed, "Method not allowed")
		return
	}

	var req JobRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		ts.writeErrorResponse(w, http.StatusBadRequest, fmt.Sprintf("Invalid JSON: %v", err))
		return
	}

	var resp JobResponse

	switch req.Action {
	case "create":
		resp = ts.createSchedule(req)
	case "delete":
		resp = ts.deleteSchedule(req)
	case "pause":
		resp = ts.pauseSchedule(req)
	case "unpause":
		resp = ts.unpauseSchedule(req)
	case "update":
		resp = ts.updateSchedule(req)
	case "list":
		resp = ts.listSchedules()
	case "run_now":
		resp = ts.runNow(req)
	case "kill_job":
		resp = ts.killJob(req)
	case "inspect_job":
		resp = ts.inspectJob(req)
	case "mark_completed":
		resp = ts.markCompleted(req)
	case "status":
		resp = ts.getJobStatus(req)
	default:
		resp = JobResponse{Success: false, Message: fmt.Sprintf("Unknown action: %s", req.Action)}
	}

	w.WriteHeader(http.StatusOK)
	json.NewEncoder(w).Encode(resp)
}

func (ts *TemporalService) createSchedule(req JobRequest) JobResponse {
	if req.JobID == "" || req.CronExpr == "" || req.RecipePath == "" {
		return JobResponse{Success: false, Message: "Missing required fields: job_id, cron, recipe_path"}
	}

	// Check if job already exists
	if _, exists := ts.scheduleJobs[req.JobID]; exists {
		return JobResponse{Success: false, Message: fmt.Sprintf("Job with ID '%s' already exists", req.JobID)}
	}

	// Validate and copy recipe file to managed storage
	managedRecipePath, recipeContent, err := ts.storeRecipeForSchedule(req.JobID, req.RecipePath)
	if err != nil {
		return JobResponse{Success: false, Message: fmt.Sprintf("Failed to store recipe: %v", err)}
	}

	scheduleID := fmt.Sprintf("goose-job-%s", req.JobID)

	// Prepare metadata to store with the schedule as a JSON string in the Note field
	executionMode := req.ExecutionMode
	if executionMode == "" {
		executionMode = "background" // Default to background if not specified
	}

	scheduleMetadata := map[string]interface{}{
		"job_id":         req.JobID,
		"cron_expr":      req.CronExpr,
		"recipe_path":    managedRecipePath,        // Use managed path
		"original_path":  req.RecipePath,           // Keep original for reference
		"execution_mode": executionMode,
		"created_at":     time.Now().Format(time.RFC3339),
	}

	// For small recipes, embed content directly in metadata
	if len(recipeContent) < 8192 { // 8KB limit for embedding
		scheduleMetadata["recipe_content"] = string(recipeContent)
		log.Printf("Embedded recipe content in metadata for job %s (size: %d bytes)", req.JobID, len(recipeContent))
	} else {
		log.Printf("Recipe too large for embedding, using managed file for job %s (size: %d bytes)", req.JobID, len(recipeContent))
	}

	metadataJSON, err := json.Marshal(scheduleMetadata)
	if err != nil {
		return JobResponse{Success: false, Message: fmt.Sprintf("Failed to encode metadata: %v", err)}
	}

	// Create Temporal schedule with metadata in Note field
	schedule := client.ScheduleOptions{
		ID: scheduleID,
		Spec: client.ScheduleSpec{
			CronExpressions: []string{req.CronExpr},
		},
		Action: &client.ScheduleWorkflowAction{
			ID:        fmt.Sprintf("workflow-%s-{{.ScheduledTime.Unix}}", req.JobID),
			Workflow:  GooseJobWorkflow,
			Args:      []interface{}{req.JobID, req.RecipePath},
			TaskQueue: TaskQueueName,
		},
		Note: string(metadataJSON), // Store metadata as JSON in the Note field
	}

	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()

	_, err = ts.client.ScheduleClient().Create(ctx, schedule)
	if err != nil {
		return JobResponse{Success: false, Message: fmt.Sprintf("Failed to create schedule: %v", err)}
	}

	// Track job in memory - ensure execution mode has a default value
	jobStatus := &JobStatus{
		ID:               req.JobID,
		CronExpr:         req.CronExpr,
		RecipePath:       req.RecipePath,
		CurrentlyRunning: false,
		Paused:           false,
		CreatedAt:        time.Now(),
		ExecutionMode:    &executionMode,
	}
	ts.scheduleJobs[req.JobID] = jobStatus

	log.Printf("Created schedule for job: %s", req.JobID)
	return JobResponse{Success: true, Message: "Schedule created successfully"}
}

func (ts *TemporalService) deleteSchedule(req JobRequest) JobResponse {
	if req.JobID == "" {
		return JobResponse{Success: false, Message: "Missing job_id"}
	}

	scheduleID := fmt.Sprintf("goose-job-%s", req.JobID)

	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()

	handle := ts.client.ScheduleClient().GetHandle(ctx, scheduleID)
	err := handle.Delete(ctx)
	if err != nil {
		return JobResponse{Success: false, Message: fmt.Sprintf("Failed to delete schedule: %v", err)}
	}

	// Clean up managed recipe files
	ts.cleanupManagedRecipe(req.JobID)

	// Remove from memory
	delete(ts.scheduleJobs, req.JobID)

	log.Printf("Deleted schedule for job: %s", req.JobID)
	return JobResponse{Success: true, Message: "Schedule deleted successfully"}
}

func (ts *TemporalService) pauseSchedule(req JobRequest) JobResponse {
	if req.JobID == "" {
		return JobResponse{Success: false, Message: "Missing job_id"}
	}

	scheduleID := fmt.Sprintf("goose-job-%s", req.JobID)

	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()

	handle := ts.client.ScheduleClient().GetHandle(ctx, scheduleID)
	err := handle.Pause(ctx, client.SchedulePauseOptions{
		Note: "Paused via API",
	})
	if err != nil {
		return JobResponse{Success: false, Message: fmt.Sprintf("Failed to pause schedule: %v", err)}
	}

	// Update in memory
	if job, exists := ts.scheduleJobs[req.JobID]; exists {
		job.Paused = true
	}

	log.Printf("Paused schedule for job: %s", req.JobID)
	return JobResponse{Success: true, Message: "Schedule paused successfully"}
}

func (ts *TemporalService) unpauseSchedule(req JobRequest) JobResponse {
	if req.JobID == "" {
		return JobResponse{Success: false, Message: "Missing job_id"}
	}

	scheduleID := fmt.Sprintf("goose-job-%s", req.JobID)

	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()

	handle := ts.client.ScheduleClient().GetHandle(ctx, scheduleID)
	err := handle.Unpause(ctx, client.ScheduleUnpauseOptions{
		Note: "Unpaused via API",
	})
	if err != nil {
		return JobResponse{Success: false, Message: fmt.Sprintf("Failed to unpause schedule: %v", err)}
	}

	// Update in memory
	if job, exists := ts.scheduleJobs[req.JobID]; exists {
		job.Paused = false
	}

	log.Printf("Unpaused schedule for job: %s", req.JobID)
	return JobResponse{Success: true, Message: "Schedule unpaused successfully"}
}

func (ts *TemporalService) updateSchedule(req JobRequest) JobResponse {
	if req.JobID == "" || req.CronExpr == "" {
		return JobResponse{Success: false, Message: "Missing required fields: job_id, cron"}
	}

	// Check if job exists
	job, exists := ts.scheduleJobs[req.JobID]
	if !exists {
		return JobResponse{Success: false, Message: fmt.Sprintf("Job with ID '%s' not found", req.JobID)}
	}

	// Check if job is currently running
	if job.CurrentlyRunning {
		return JobResponse{Success: false, Message: fmt.Sprintf("Cannot update schedule '%s' while it's currently running", req.JobID)}
	}

	scheduleID := fmt.Sprintf("goose-job-%s", req.JobID)

	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()

	// Get the existing schedule handle
	handle := ts.client.ScheduleClient().GetHandle(ctx, scheduleID)
	
	// Update the schedule with new cron expression while preserving metadata
	err := handle.Update(ctx, client.ScheduleUpdateOptions{
		DoUpdate: func(input client.ScheduleUpdateInput) (*client.ScheduleUpdate, error) {
			// Update the cron expression
			input.Description.Schedule.Spec.CronExpressions = []string{req.CronExpr}

			// Update the cron expression in metadata stored in Note field
			if input.Description.Schedule.State.Note != "" {
				var metadata map[string]interface{}
				if err := json.Unmarshal([]byte(input.Description.Schedule.State.Note), &metadata); err == nil {
					metadata["cron_expr"] = req.CronExpr
					if updatedMetadataJSON, err := json.Marshal(metadata); err == nil {
						input.Description.Schedule.State.Note = string(updatedMetadataJSON)
					}
				}
			}

			return &client.ScheduleUpdate{
				Schedule: &input.Description.Schedule,
			}, nil
		},
	})
	
	if err != nil {
		return JobResponse{Success: false, Message: fmt.Sprintf("Failed to update schedule: %v", err)}
	}

	// Update in memory
	job.CronExpr = req.CronExpr

	log.Printf("Updated schedule for job: %s with new cron: %s", req.JobID, req.CronExpr)
	return JobResponse{Success: true, Message: "Schedule updated successfully"}
}

func (ts *TemporalService) listSchedules() JobResponse {
	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()

	// List all schedules from Temporal
	iter, err := ts.client.ScheduleClient().List(ctx, client.ScheduleListOptions{})
	if err != nil {
		return JobResponse{Success: false, Message: fmt.Sprintf("Failed to list schedules: %v", err)}
	}

	var jobs []JobStatus
	for iter.HasNext() {
		schedule, err := iter.Next()
		if err != nil {
			log.Printf("Error listing schedules: %v", err)
			continue
		}

		// Extract job ID from schedule ID
		if strings.HasPrefix(schedule.ID, "goose-job-") {
			jobID := strings.TrimPrefix(schedule.ID, "goose-job-")

			// Get detailed schedule information to access metadata
			scheduleHandle := ts.client.ScheduleClient().GetHandle(ctx, schedule.ID)
			desc, err := scheduleHandle.Describe(ctx)
			if err != nil {
				log.Printf("Warning: Could not get detailed info for schedule %s: %v", schedule.ID, err)
				continue
			}

			// Initialize job status with defaults
			jobStatus := JobStatus{
				ID:               jobID,
				CurrentlyRunning: ts.isJobCurrentlyRunning(ctx, jobID),
				Paused:           desc.Schedule.State.Paused,
				CreatedAt:        time.Now(), // Fallback if not in metadata
			}

			// Extract metadata from the schedule's Note field (stored as JSON)
			if desc.Schedule.State.Note != "" {
				var metadata map[string]interface{}
				if err := json.Unmarshal([]byte(desc.Schedule.State.Note), &metadata); err == nil {
					// Extract cron expression
					if cronExpr, ok := metadata["cron_expr"].(string); ok {
						jobStatus.CronExpr = cronExpr
					} else if len(desc.Schedule.Spec.CronExpressions) > 0 {
						// Fallback to spec if not in metadata
						jobStatus.CronExpr = desc.Schedule.Spec.CronExpressions[0]
					}

					// Extract recipe path
					if recipePath, ok := metadata["recipe_path"].(string); ok {
						jobStatus.RecipePath = recipePath
					}

					// Extract execution mode
					if executionMode, ok := metadata["execution_mode"].(string); ok {
						jobStatus.ExecutionMode = &executionMode
					}

					// Extract creation time
					if createdAtStr, ok := metadata["created_at"].(string); ok {
						if createdAt, err := time.Parse(time.RFC3339, createdAtStr); err == nil {
							jobStatus.CreatedAt = createdAt
						}
					}
				} else {
					log.Printf("Failed to parse metadata from Note field for schedule %s: %v", schedule.ID, err)
					// Fallback to spec values
					if len(desc.Schedule.Spec.CronExpressions) > 0 {
						jobStatus.CronExpr = desc.Schedule.Spec.CronExpressions[0]
					}
					defaultMode := "background"
					jobStatus.ExecutionMode = &defaultMode
				}
			} else {
				// Fallback for schedules without metadata (legacy schedules)
				log.Printf("Schedule %s has no metadata, using fallback values", schedule.ID)
				if len(desc.Schedule.Spec.CronExpressions) > 0 {
					jobStatus.CronExpr = desc.Schedule.Spec.CronExpressions[0]
				}
				// For legacy schedules, we can't recover recipe path or execution mode
				defaultMode := "background"
				jobStatus.ExecutionMode = &defaultMode
			}

			// Update last run time - use the most recent between scheduled and manual runs
			var mostRecentRun *string

			// Check scheduled runs from Temporal
			if len(desc.Info.RecentActions) > 0 {
				lastAction := desc.Info.RecentActions[len(desc.Info.RecentActions)-1]
				if !lastAction.ActualTime.IsZero() {
					scheduledRunStr := lastAction.ActualTime.Format(time.RFC3339)
					mostRecentRun = &scheduledRunStr
					log.Printf("Job %s scheduled run: %s", jobID, scheduledRunStr)
				}
			}

			// Check manual runs from our in-memory tracking (if available)
			if tracked, exists := ts.scheduleJobs[jobID]; exists && tracked.LastManualRun != nil {
				log.Printf("Job %s manual run: %s", jobID, *tracked.LastManualRun)

				// Compare times if we have both
				if mostRecentRun != nil {
					scheduledTime, err1 := time.Parse(time.RFC3339, *mostRecentRun)
					manualTime, err2 := time.Parse(time.RFC3339, *tracked.LastManualRun)

					if err1 == nil && err2 == nil {
						if manualTime.After(scheduledTime) {
							mostRecentRun = tracked.LastManualRun
							log.Printf("Job %s: manual run is more recent", jobID)
						} else {
							log.Printf("Job %s: scheduled run is more recent", jobID)
						}
					}
				} else {
					// Only manual run available
					mostRecentRun = tracked.LastManualRun
					log.Printf("Job %s: only manual run available", jobID)
				}
			}

			if mostRecentRun != nil {
				jobStatus.LastRun = mostRecentRun
			} else {
				log.Printf("Job %s has no runs (scheduled or manual)", jobID)
			}

			// Update in-memory tracking with latest info for manual run tracking
			ts.scheduleJobs[jobID] = &jobStatus

			jobs = append(jobs, jobStatus)
		}
	}

	return JobResponse{Success: true, Jobs: jobs}
}

// isJobCurrentlyRunning checks if there are any running workflows for the given job ID
func (ts *TemporalService) isJobCurrentlyRunning(ctx context.Context, jobID string) bool {
	// Check our in-memory tracking of running jobs
	if running, exists := ts.runningJobs[jobID]; exists && running {
		return true
	}
	return false
}

// markJobAsRunning sets a job as currently running and tracks the workflow ID
func (ts *TemporalService) markJobAsRunning(jobID string) {
	ts.runningJobs[jobID] = true
	log.Printf("Marked job %s as running", jobID)
}

// markJobAsNotRunning sets a job as not currently running and clears workflow tracking
func (ts *TemporalService) markJobAsNotRunning(jobID string) {
	delete(ts.runningJobs, jobID)
	delete(ts.runningWorkflows, jobID)
	log.Printf("Marked job %s as not running", jobID)
}

// addRunningWorkflow tracks a workflow ID for a job
func (ts *TemporalService) addRunningWorkflow(jobID, workflowID string) {
	if ts.runningWorkflows[jobID] == nil {
		ts.runningWorkflows[jobID] = make([]string, 0)
	}
	ts.runningWorkflows[jobID] = append(ts.runningWorkflows[jobID], workflowID)
	log.Printf("Added workflow %s for job %s", workflowID, jobID)
}

// removeRunningWorkflow removes a workflow ID from job tracking
func (ts *TemporalService) removeRunningWorkflow(jobID, workflowID string) {
	if workflows, exists := ts.runningWorkflows[jobID]; exists {
		for i, id := range workflows {
			if id == workflowID {
				ts.runningWorkflows[jobID] = append(workflows[:i], workflows[i+1:]...)
				break
			}
		}
		if len(ts.runningWorkflows[jobID]) == 0 {
			delete(ts.runningWorkflows, jobID)
			ts.runningJobs[jobID] = false
		}
	}
}

func (ts *TemporalService) runNow(req JobRequest) JobResponse {
	if req.JobID == "" {
		return JobResponse{Success: false, Message: "Missing job_id"}
	}

	// Get job details
	job, exists := ts.scheduleJobs[req.JobID]
	if !exists {
		return JobResponse{Success: false, Message: fmt.Sprintf("Job '%s' not found", req.JobID)}
	}

	// Record the manual run time
	now := time.Now()
	manualRunStr := now.Format(time.RFC3339)
	job.LastManualRun = &manualRunStr
	log.Printf("Recording manual run for job %s at %s", req.JobID, manualRunStr)

	// Execute workflow immediately
	workflowOptions := client.StartWorkflowOptions{
		ID:        fmt.Sprintf("manual-%s-%d", req.JobID, now.Unix()),
		TaskQueue: TaskQueueName,
	}

	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()

	we, err := ts.client.ExecuteWorkflow(ctx, workflowOptions, GooseJobWorkflow, req.JobID, job.RecipePath)
	if err != nil {
		return JobResponse{Success: false, Message: fmt.Sprintf("Failed to start workflow: %v", err)}
	}

	// Track the workflow for this job
	ts.addRunningWorkflow(req.JobID, we.GetID())

	// Don't wait for completion in run_now, just return the workflow ID
	log.Printf("Manual execution started for job: %s, workflow: %s", req.JobID, we.GetID())
	return JobResponse{
		Success: true,
		Message: "Job execution started",
		Data:    RunNowResponse{SessionID: we.GetID()}, // Return workflow ID as session ID for now
	}
}

func (ts *TemporalService) killJob(req JobRequest) JobResponse {
	if req.JobID == "" {
		return JobResponse{Success: false, Message: "Missing job_id"}
	}

	// Check if job exists
	_, exists := ts.scheduleJobs[req.JobID]
	if !exists {
		return JobResponse{Success: false, Message: fmt.Sprintf("Job '%s' not found", req.JobID)}
	}

	// Check if job is currently running
	if !ts.isJobCurrentlyRunning(context.Background(), req.JobID) {
		return JobResponse{Success: false, Message: fmt.Sprintf("Job '%s' is not currently running", req.JobID)}
	}

	ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
	defer cancel()

	log.Printf("Starting kill process for job %s", req.JobID)

	// Step 1: Kill managed processes first
	processKilled := false
	if err := globalProcessManager.KillProcess(req.JobID); err != nil {
		log.Printf("Failed to kill managed process for job %s: %v", req.JobID, err)
	} else {
		log.Printf("Successfully killed managed process for job %s", req.JobID)
		processKilled = true
	}

	// Step 2: Terminate Temporal workflows
	workflowsKilled := 0
	workflowIDs, exists := ts.runningWorkflows[req.JobID]
	if exists && len(workflowIDs) > 0 {
		for _, workflowID := range workflowIDs {
			// Terminate the workflow
			err := ts.client.TerminateWorkflow(ctx, workflowID, "", "Killed by user request")
			if err != nil {
				log.Printf("Error terminating workflow %s for job %s: %v", workflowID, req.JobID, err)
				continue
			}
			log.Printf("Terminated workflow %s for job %s", workflowID, req.JobID)
			workflowsKilled++
		}
		log.Printf("Terminated %d workflow(s) for job %s", workflowsKilled, req.JobID)
	}

	// Step 3: Find and kill any remaining processes by name/pattern
	additionalKills := ts.killProcessesByJobPattern(req.JobID)

	// Step 4: Mark job as not running in our tracking
	ts.markJobAsNotRunning(req.JobID)

	// Prepare response message
	var messages []string
	if processKilled {
		messages = append(messages, "killed managed process")
	}
	if workflowsKilled > 0 {
		messages = append(messages, fmt.Sprintf("terminated %d workflow(s)", workflowsKilled))
	}
	if additionalKills > 0 {
		messages = append(messages, fmt.Sprintf("killed %d additional process(es)", additionalKills))
	}

	if len(messages) == 0 {
		messages = append(messages, "no active processes found but marked as not running")
	}

	log.Printf("Killed job: %s (%s)", req.JobID, strings.Join(messages, ", "))
	return JobResponse{
		Success: true,
		Message: fmt.Sprintf("Successfully killed job '%s': %s", req.JobID, strings.Join(messages, ", ")),
	}
}

// killProcessesByJobPattern finds and kills processes related to a job by searching for patterns
func (ts *TemporalService) killProcessesByJobPattern(jobID string) int {
	log.Printf("Searching for additional processes to kill for job %s", jobID)
	
	killedCount := 0
	
	switch runtime.GOOS {
	case "darwin", "linux":
		// Search for goose processes that might be related to this job
		patterns := []string{
			fmt.Sprintf("scheduled-%s", jobID),  // Session name pattern
			fmt.Sprintf("GOOSE_JOB_ID=%s", jobID), // Environment variable pattern
			jobID, // Job ID itself
		}
		
		for _, pattern := range patterns {
			// Use pgrep to find processes
			cmd := exec.Command("pgrep", "-f", pattern)
			output, err := cmd.Output()
			if err != nil {
				log.Printf("No processes found for pattern '%s': %v", pattern, err)
				continue
			}
			
			pidStr := strings.TrimSpace(string(output))
			if pidStr == "" {
				continue
			}
			
			pids := strings.Split(pidStr, "\n")
			for _, pidStr := range pids {
				if pidStr == "" {
					continue
				}
				
				pid, err := strconv.Atoi(pidStr)
				if err != nil {
					log.Printf("Invalid PID '%s': %v", pidStr, err)
					continue
				}
				
				log.Printf("Found process %d matching pattern '%s' for job %s", pid, pattern, jobID)
				
				// Kill the process
				if err := syscall.Kill(pid, syscall.SIGTERM); err != nil {
					log.Printf("Failed to send SIGTERM to PID %d: %v", pid, err)
				} else {
					log.Printf("Sent SIGTERM to PID %d", pid)
					killedCount++
				}
				
				// Wait a moment then force kill
				time.Sleep(500 * time.Millisecond)
				if err := syscall.Kill(pid, syscall.SIGKILL); err != nil {
					log.Printf("Failed to send SIGKILL to PID %d: %v", pid, err)
				} else {
					log.Printf("Sent SIGKILL to PID %d", pid)
				}
			}
		}
		
	case "windows":
		// On Windows, search for goose.exe processes
		sessionPattern := fmt.Sprintf("scheduled-%s", jobID)
		
		// Use tasklist to find processes
		cmd := exec.Command("tasklist", "/FI", "IMAGENAME eq goose.exe", "/FO", "CSV")
		output, err := cmd.Output()
		if err != nil {
			log.Printf("Failed to list Windows processes: %v", err)
			return killedCount
		}
		
		lines := strings.Split(string(output), "\n")
		for _, line := range lines {
			if strings.Contains(line, sessionPattern) || strings.Contains(line, jobID) {
				// Extract PID from CSV format
				fields := strings.Split(line, ",")
				if len(fields) >= 2 {
					pidStr := strings.Trim(fields[1], "\"")
					if pid, err := strconv.Atoi(pidStr); err == nil {
						log.Printf("Found Windows process %d for job %s", pid, jobID)
						
						// Kill the process
						killCmd := exec.Command("taskkill", "/F", "/PID", fmt.Sprintf("%d", pid))
						if err := killCmd.Run(); err != nil {
							log.Printf("Failed to kill Windows process %d: %v", pid, err)
						} else {
							log.Printf("Killed Windows process %d", pid)
							killedCount++
						}
					}
				}
			}
		}
	}
	
	log.Printf("Killed %d additional processes for job %s", killedCount, jobID)
	return killedCount
}

func (ts *TemporalService) inspectJob(req JobRequest) JobResponse {
	if req.JobID == "" {
		return JobResponse{Success: false, Message: "Missing job_id"}
	}

	// Check if job exists
	_, exists := ts.scheduleJobs[req.JobID]
	if !exists {
		return JobResponse{Success: false, Message: fmt.Sprintf("Job '%s' not found", req.JobID)}
	}

	// Check if job is currently running
	if !ts.isJobCurrentlyRunning(context.Background(), req.JobID) {
		return JobResponse{Success: false, Message: fmt.Sprintf("Job '%s' is not currently running", req.JobID)}
	}

	// Get process information
	processes := globalProcessManager.ListProcesses()
	if mp, exists := processes[req.JobID]; exists {
		duration := time.Since(mp.StartTime)

		inspectData := map[string]interface{}{
			"job_id":              req.JobID,
			"process_id":          mp.Process.Pid,
			"running_duration":    duration.String(),
			"running_duration_seconds": int(duration.Seconds()),
			"start_time":          mp.StartTime.Format(time.RFC3339),
		}

		// Try to get session ID from workflow tracking
		if workflowIDs, exists := ts.runningWorkflows[req.JobID]; exists && len(workflowIDs) > 0 {
			inspectData["session_id"] = workflowIDs[0] // Use the first workflow ID as session ID
		}

		return JobResponse{
			Success: true,
			Message: fmt.Sprintf("Job '%s' is running", req.JobID),
			Data:    inspectData,
		}
	}

	// If no managed process found, check workflows only
	if workflowIDs, exists := ts.runningWorkflows[req.JobID]; exists && len(workflowIDs) > 0 {
		inspectData := map[string]interface{}{
			"job_id":     req.JobID,
			"session_id": workflowIDs[0],
			"message":    "Job is running but process information not available",
		}

		return JobResponse{
			Success: true,
			Message: fmt.Sprintf("Job '%s' is running (workflow only)", req.JobID),
			Data:    inspectData,
		}
	}

	return JobResponse{
		Success: false,
		Message: fmt.Sprintf("Job '%s' appears to be running but no process or workflow information found", req.JobID),
	}
}

func (ts *TemporalService) markCompleted(req JobRequest) JobResponse {
	if req.JobID == "" {
		return JobResponse{Success: false, Message: "Missing job_id"}
	}

	// Check if job exists
	_, exists := ts.scheduleJobs[req.JobID]
	if !exists {
		return JobResponse{Success: false, Message: fmt.Sprintf("Job '%s' not found", req.JobID)}
	}

	log.Printf("Marking job %s as completed (requested by Rust scheduler)", req.JobID)

	// Mark job as not running in our tracking
	ts.markJobAsNotRunning(req.JobID)

	// Also try to clean up any lingering processes
	if err := globalProcessManager.KillProcess(req.JobID); err != nil {
		log.Printf("No process to clean up for job %s: %v", req.JobID, err)
	}

	return JobResponse{
		Success: true,
		Message: fmt.Sprintf("Job '%s' marked as completed", req.JobID),
	}
}

func (ts *TemporalService) getJobStatus(req JobRequest) JobResponse {
	if req.JobID == "" {
		return JobResponse{Success: false, Message: "Missing job_id"}
	}

	// Check if job exists
	job, exists := ts.scheduleJobs[req.JobID]
	if !exists {
		return JobResponse{Success: false, Message: fmt.Sprintf("Job '%s' not found", req.JobID)}
	}

	// Update the currently running status based on our tracking
	job.CurrentlyRunning = ts.isJobCurrentlyRunning(context.Background(), req.JobID)

	// Return the job as a single-item array for consistency with list endpoint
	jobs := []JobStatus{*job}

	return JobResponse{
		Success: true,
		Message: fmt.Sprintf("Status for job '%s'", req.JobID),
		Jobs:    jobs,
	}
}

func (ts *TemporalService) writeErrorResponse(w http.ResponseWriter, statusCode int, message string) {
	w.WriteHeader(statusCode)
	json.NewEncoder(w).Encode(JobResponse{Success: false, Message: message})
}

// Recipe represents the structure we need from recipe files
type Recipe struct {
	Title        string  `json:"title" yaml:"title"`
	Description  string  `json:"description" yaml:"description"`
	Instructions *string `json:"instructions" yaml:"instructions"`
	Prompt       *string `json:"prompt" yaml:"prompt"`
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
	cmd.SysProcAttr = &syscall.SysProcAttr{
		Setpgid: true, // Create new process group
	}

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
	cmd.SysProcAttr = &syscall.SysProcAttr{
		Setpgid: true, // Create new process group
	}

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

func (ts *TemporalService) handleHealth(w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusOK)
	json.NewEncoder(w).Encode(map[string]string{"status": "healthy"})
}

// handlePorts returns the port configuration for this service
func (ts *TemporalService) handlePorts(w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusOK)

	portInfo := map[string]int{
		"http_port":     ts.ports.HTTPPort,
		"temporal_port": ts.ports.TemporalPort,
		"ui_port":       ts.ports.UIPort,
	}

	json.NewEncoder(w).Encode(portInfo)
}

func main() {
	log.Println("Starting Temporal service...")

	// Create Temporal service (this will find available ports automatically)
	service, err := NewTemporalService()
	if err != nil {
		log.Fatalf("Failed to create Temporal service: %v", err)
	}

	// Use the dynamically assigned HTTP port
	httpPort := service.GetHTTPPort()
	temporalPort := service.GetTemporalPort()
	uiPort := service.GetUIPort()

	log.Printf("Temporal server running on port %d", temporalPort)
	log.Printf("Temporal UI available at http://localhost:%d", uiPort)

	// Set up HTTP server
	mux := http.NewServeMux()
	mux.HandleFunc("/jobs", service.handleJobs)
	mux.HandleFunc("/health", service.handleHealth)
	mux.HandleFunc("/ports", service.handlePorts)

	server := &http.Server{
		Addr:    fmt.Sprintf(":%d", httpPort),
		Handler: mux,
	}

	// Handle graceful shutdown
	sigChan := make(chan os.Signal, 1)
	signal.Notify(sigChan, syscall.SIGINT, syscall.SIGTERM)

	go func() {
		<-sigChan
		log.Println("Received shutdown signal")

		// Kill all managed processes first
		globalProcessManager.KillAllProcesses()

		// Shutdown HTTP server
		ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
		defer cancel()
		server.Shutdown(ctx)

		// Stop Temporal service
		service.Stop()

		os.Exit(0)
	}()

	log.Printf("Temporal service starting on port %d", httpPort)
	log.Printf("Health endpoint: http://localhost:%d/health", httpPort)
	log.Printf("Jobs endpoint: http://localhost:%d/jobs", httpPort)
	log.Printf("Ports endpoint: http://localhost:%d/ports", httpPort)

	if err := server.ListenAndServe(); err != nil && err != http.ErrServerClosed {
		log.Fatalf("HTTP server failed: %v", err)
	}
}
