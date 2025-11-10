package main

import (
	"context"
	"encoding/json"
	"fmt"
	"log"
	"os"
	"path/filepath"
	"strings"
	"time"

	"go.temporal.io/sdk/client"
)

type JobStatus struct {
	ID               string    `json:"id"`
	CronExpr         string    `json:"cron"`
	RecipePath       string    `json:"recipe_path"`
	LastRun          *string   `json:"last_run,omitempty"`
	NextRun          *string   `json:"next_run,omitempty"`
	CurrentlyRunning bool      `json:"currently_running"`
	Paused           bool      `json:"paused"`
	CreatedAt        time.Time `json:"created_at"`
	ExecutionMode    *string   `json:"execution_mode,omitempty"`  // "foreground" or "background"
	LastManualRun    *string   `json:"last_manual_run,omitempty"` // Track manual runs separately
}

// Request/Response types for HTTP API
type JobRequest struct {
	Action        string `json:"action"` // create, delete, pause, unpause, list, run_now, kill_job, update
	JobID         string `json:"job_id"`
	CronExpr      string `json:"cron"`
	RecipePath    string `json:"recipe_path"`
	ExecutionMode string `json:"execution_mode,omitempty"` // "foreground" or "background"
}

type JobResponse struct {
	Success bool        `json:"success"`
	Message string      `json:"message"`
	Jobs    []JobStatus `json:"jobs,omitempty"`
	Data    interface{} `json:"data,omitempty"`
}

type RunNowResponse struct {
	SessionID string `json:"session_id"`
}

// createSchedule handles the creation of a new schedule
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
		"recipe_path":    managedRecipePath, // Use managed path
		"original_path":  req.RecipePath,    // Keep original for reference
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

// deleteSchedule handles the deletion of a schedule
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

// pauseSchedule handles pausing a schedule
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

// unpauseSchedule handles unpausing a schedule
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

// updateSchedule handles updating a schedule
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

// listSchedules lists all schedules
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

// runNow executes a job immediately
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

// killJob kills a running job
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
	additionalKills := FindAndKillProcessesByPattern(req.JobID)

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

// inspectJob inspects a running job
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
			"job_id":                   req.JobID,
			"process_id":               mp.Process.Pid,
			"running_duration":         duration.String(),
			"running_duration_seconds": int(duration.Seconds()),
			"start_time":               mp.StartTime.Format(time.RFC3339),
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

// markCompleted marks a job as completed
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

// getJobStatus gets the status of a job
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
	if _, err := ts.parseRecipeContent(recipeContent); err != nil {
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
