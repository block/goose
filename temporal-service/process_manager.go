package main

import (
	"context"
	"fmt"
	"log"
	"os"
	"os/exec"
	"runtime"
	"strconv"
	"strings"
	"sync"
	"syscall"
	"time"
)

// ProcessManager tracks and manages spawned processes
type ProcessManager struct {
	processes map[string]*ManagedProcess
	mutex     sync.RWMutex
}

// ManagedProcess represents a process being managed by the ProcessManager
type ManagedProcess struct {
	JobID     string
	Process   *os.Process
	Cancel    context.CancelFunc
	StartTime time.Time
}

// Global process manager instance
var globalProcessManager = &ProcessManager{
	processes: make(map[string]*ManagedProcess),
}

// AddProcess adds a process to be managed
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

// RemoveProcess removes a process from management
func (pm *ProcessManager) RemoveProcess(jobID string) {
	pm.mutex.Lock()
	defer pm.mutex.Unlock()

	if mp, exists := pm.processes[jobID]; exists {
		log.Printf("Removed process %d for job %s from process manager", mp.Process.Pid, jobID)
		delete(pm.processes, jobID)
	}
}

// KillProcess kills a specific process and its children
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

// KillAllProcesses kills all managed processes
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

// ListProcesses returns a copy of the current process map
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
		return killProcessGroupByPID(pid, 0) // signal parameter not used on Windows
	default:
		// On Unix-like systems, kill the process group more aggressively
		log.Printf("Killing Unix process group for PID %d", pid)
		
		// First, try to kill the entire process group with SIGTERM
		if err := killProcessGroupByPID(pid, syscall.SIGTERM); err != nil {
			log.Printf("Failed to send SIGTERM to process group -%d: %v", pid, err)
		} else {
			log.Printf("Sent SIGTERM to process group -%d", pid)
		}
		
		// Also try to kill the main process directly
		if err := killProcessByPID(pid, syscall.SIGTERM); err != nil {
			log.Printf("Failed to send SIGTERM to process %d: %v", pid, err)
		} else {
			log.Printf("Sent SIGTERM to process %d", pid)
		}

		// Give processes a brief moment to terminate gracefully
		time.Sleep(1 * time.Second)

		// Force kill the process group with SIGKILL
		if err := killProcessGroupByPID(pid, syscall.SIGKILL); err != nil {
			log.Printf("Failed to send SIGKILL to process group -%d: %v", pid, err)
		} else {
			log.Printf("Sent SIGKILL to process group -%d", pid)
		}
		
		// Force kill the main process with SIGKILL
		if err := killProcessByPID(pid, syscall.SIGKILL); err != nil {
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

// FindAndKillProcessesByPattern finds and kills processes related to a job by searching for patterns
func FindAndKillProcessesByPattern(jobID string) int {
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
				if err := killProcessByPID(pid, syscall.SIGTERM); err != nil {
					log.Printf("Failed to send SIGTERM to PID %d: %v", pid, err)
				} else {
					log.Printf("Sent SIGTERM to PID %d", pid)
					killedCount++
				}
				
				// Wait a moment then force kill
				time.Sleep(500 * time.Millisecond)
				if err := killProcessByPID(pid, syscall.SIGKILL); err != nil {
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