package main

import (
	"context"
	"encoding/json"
	"fmt"
	"log"
	"net/http"
	"os"
	"time"

	"go.temporal.io/sdk/client"
	"go.temporal.io/sdk/worker"
	"gopkg.in/yaml.v2"
)

// Global service instance for activities to access
var globalService *TemporalService

// TemporalService manages the Temporal client and provides HTTP API
type TemporalService struct {
	client          client.Client
	worker          worker.Worker
	scheduleJobs    map[string]*JobStatus // In-memory job tracking
	runningJobs     map[string]bool       // Track which jobs are currently running
	runningWorkflows map[string][]string  // Track workflow IDs for each job
	recipesDir      string                // Directory for managed recipe storage
	ports           *PortConfig           // Port configuration
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
		client:          c,
		worker:          w,
		scheduleJobs:    make(map[string]*JobStatus),
		runningJobs:     make(map[string]bool),
		runningWorkflows: make(map[string][]string),
		recipesDir:      recipesDir,
		ports:           ports,
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

func (ts *TemporalService) handleHealth(w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusOK)
	json.NewEncoder(w).Encode(map[string]string{"status": "healthy"})
}

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

// writeErrorResponse writes a standardized error response
func (ts *TemporalService) writeErrorResponse(w http.ResponseWriter, statusCode int, message string) {
	w.WriteHeader(statusCode)
	json.NewEncoder(w).Encode(JobResponse{Success: false, Message: message})
}

// isJobCurrentlyRunning checks if there are any running workflows for the given job ID
func (ts *TemporalService) isJobCurrentlyRunning(ctx context.Context, jobID string) bool {
	// Check our in-memory tracking of running jobs
	if running, exists := ts.runningJobs[jobID]; exists && running {
		return true
	}
	return false
}

// parseRecipeContent parses recipe content from bytes (YAML or JSON)
func (ts *TemporalService) parseRecipeContent(content []byte) (*Recipe, error) {
	var recipe Recipe

	// Try YAML first, then JSON
	if err := yaml.Unmarshal(content, &recipe); err != nil {
		if err := json.Unmarshal(content, &recipe); err != nil {
			return nil, fmt.Errorf("failed to parse as YAML or JSON: %w", err)
		}
	}

	return &recipe, nil
}