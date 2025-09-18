package main

import (
	"context"
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
	"go.temporal.io/sdk/client"
	"temporal-service/i18n"
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
		log.Printf(i18n.T("FoundTemporalInPATH"), path)
		// Verify it's the correct temporal CLI by checking version
		log.Println("Verifying temporal CLI version...")
		cmd := exec.Command(path, "--version")
		if err := cmd.Run(); err == nil {
			log.Printf(i18n.T("SuccessfullyVerifiedTemporalCLI"), path)
			return path, nil
		} else {
			log.Printf(i18n.T("FailedToVerifyTemporalCLI"), path, err)
		}
	} else {
		log.Printf(i18n.T("TemporalNotFoundInPATH"), err)
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
	currentPaths := []string{
		"./temporal",
		"./temporal.exe",
	}
	if path, err := getExistingTemporalCLIFrom(currentPaths); err == nil {
		return path, nil
	} else {
		log.Printf(i18n.T("AttemptToFindInLocalDirectoryFailed"), err)
	}

	// Also try relative to the current executable (most important for bundled apps)
	exePath, err := os.Executable()
	if err != nil {
		log.Printf(i18n.T("FailedToGetExecutablePath"), err)
	}
	exeDir := filepath.Dir(exePath)
	log.Printf(i18n.T("ExecutableDirectory"), exeDir)
	additionalPaths := []string{
		filepath.Join(exeDir, "temporal"),
		filepath.Join(exeDir, "temporal.exe"), // Windows
		// Also try one level up (for development)
		filepath.Join(exeDir, "..", "temporal"),
		filepath.Join(exeDir, "..", "temporal.exe"),
	}
	log.Printf(i18n.T("WillCheckTheseAdditionalPaths"), additionalPaths)
	return getExistingTemporalCLIFrom(additionalPaths)
}

// getExistingTemporalCLIFrom gets a list of paths and returns one of those that is an existing and working Temporal CLI binary
func getExistingTemporalCLIFrom(possiblePaths []string) (string, error) {
	log.Printf(i18n.T("CheckingPossiblePathsForTemporalCLI"), len(possiblePaths))

	// Check all possible paths in parallel, pick the first one that works.
	pathFound := make(chan string)
	var wg sync.WaitGroup
	// This allows us to cancel whatever remaining work is done when we find a valid path.
	psCtx, psCancel := context.WithCancel(context.Background())
	for i, path := range possiblePaths {
		wg.Add(1)
		go func() {
			defer wg.Done()
			log.Printf(i18n.T("CheckingPath"), i+1, len(possiblePaths), path)
			if _, err := os.Stat(path); err != nil {
				log.Printf(i18n.T("FileDoesNotExistAt"), path, err)
				return
			}
			log.Printf(i18n.T("FileExistsAt"), path)
			// File exists, test if it's executable and the right binary
			cmd := exec.CommandContext(psCtx, path, "--version")
			if err := cmd.Run(); err != nil {
				log.Printf(i18n.T("FailedToVerifyTemporalCLI"), path, err)
				return
			}
			select {
			case pathFound <- path:
				log.Printf(i18n.T("SuccessfullyVerifiedTemporalCLI"), path)
			case <-psCtx.Done():
				// No need to report the path not chosen.
			}
		}()
	}
	// We transform the workgroup wait into a channel so we can wait for either this or pathFound
	pathNotFound := make(chan bool)
	go func() {
		wg.Wait()
		pathNotFound <- true
	}()
	select {
	case path := <-pathFound:
		psCancel() // Cancel the remaining search functions otherwise they'll just exist eternally.
		return path, nil
	case <-pathNotFound:
		// No need to do anything, this just says that none of the functions were able to do it and there's nothing left to cleanup
	}

	return "", fmt.Errorf("temporal CLI not found in PATH or any of the expected locations: %v", possiblePaths)
}

// ensureTemporalServerRunning checks if Temporal server is running and starts it if needed
func ensureTemporalServerRunning(ports *PortConfig) error {
	log.Println("Checking if Temporal server is running...")

	// Check if Temporal server is already running by trying to connect
	if isTemporalServerRunning(ports.TemporalPort) {
		log.Printf(i18n.T("TemporalServerAlreadyRunningOnPort"), ports.TemporalPort)
		return nil
	}

	log.Printf(i18n.T("TemporalServerNotRunningAttemptingToStart"), ports.TemporalPort)

	// Find the temporal CLI binary
	temporalCmd, err := findTemporalCLI()
	if err != nil {
		log.Printf(i18n.T("CouldNotFindTemporalCLI"), err)
		return fmt.Errorf("could not find temporal CLI: %w", err)
	}

	log.Printf(i18n.T("UsingTemporalCLIAt"), temporalCmd)

	// Start Temporal server in background
	args := []string{"server", "start-dev",
		"--db-filename", "temporal.db",
		"--port", strconv.Itoa(ports.TemporalPort),
		"--ui-port", strconv.Itoa(ports.UIPort),
		"--log-level", "warn"}

	log.Printf(i18n.T("StartingTemporalServerWithCommand"), temporalCmd, args)

	cmd := exec.Command(temporalCmd, args...)

	// Properly detach the process so it survives when the parent exits
	configureSysProcAttr(cmd)

	// Redirect stdin/stdout/stderr to avoid hanging
	cmd.Stdin = nil
	cmd.Stdout = nil
	cmd.Stderr = nil

	// Start the process
	if err := cmd.Start(); err != nil {
		log.Printf(i18n.T("FailedToStartTemporalServer"), err)
		return fmt.Errorf("failed to start Temporal server: %w", err)
	}

	log.Printf(i18n.T("TemporalServerStartedWithPID"),
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
			log.Printf(i18n.T("TimeoutWaitingForTemporalServerToStart"), attemptCount)
			return fmt.Errorf("timeout waiting for Temporal server to start")
		case <-ticker.C:
			attemptCount++
			log.Printf(i18n.T("CheckingIfTemporalServerIsReady"), attemptCount)
			if isTemporalServerRunning(ports.TemporalPort) {
				log.Printf(i18n.T("TemporalServerIsNowReadyOnPort"), ports.TemporalPort)
				return nil
			} else {
				log.Printf(i18n.T("TemporalServerNotReadyYet"), attemptCount)
			}
		}
	}
}

func main() {
	// Parse command line arguments for language
	var lang string
	if len(os.Args) > 1 && os.Args[1] == "--lang" && len(os.Args) > 2 {
		lang = os.Args[2]
		// Remove the --lang flag and value from os.Args
		os.Args = append(os.Args[:1], os.Args[3:]...)
	} else {
		lang = i18n.GetLocale()
	}
	
	// Initialize i18n system
	if err := i18n.Init(lang); err != nil {
		log.Printf("Warning: Failed to initialize i18n system: %v, falling back to English", err)
	}
	
	log.Printf("Using language: %s", lang)
	log.Println(i18n.T("StartingTemporalService"))
	log.Printf(i18n.T("RuntimeOS"), runtime.GOOS)
	log.Printf(i18n.T("RuntimeARCH"), runtime.GOARCH)
	
	// Log current working directory for debugging
	if cwd, err := os.Getwd(); err == nil {
		log.Printf(i18n.T("CurrentWorkingDirectory"), cwd)
	}
	
	// Log environment variables that might affect behavior
	if port := os.Getenv("PORT"); port != "" {
		log.Printf(i18n.T("PortEnvironmentVariable"), port)
	}
	if rustLog := os.Getenv("RUST_LOG"); rustLog != "" {
		log.Printf(i18n.T("RustLogEnvironmentVariable"), rustLog)
	}
	if temporalLog := os.Getenv("TEMPORAL_LOG_LEVEL"); temporalLog != "" {
		log.Printf(i18n.T("TemporalLogLevelEnvironmentVariable"), temporalLog)
	}

	// Create Temporal service (this will find available ports automatically)
	log.Println(i18n.T("CreatingTemporalService"))
	service, err := NewTemporalService()
	if err != nil {
		log.Printf(i18n.T("FailedToCreateTemporalService"), err)
		log.Fatalf(i18n.T("FailedToCreateTemporalService"), err)
	}
	log.Println(i18n.T("TemporalServiceCreatedSuccessfully"))

	// Use the dynamically assigned HTTP port
	httpPort := service.GetHTTPPort()
	temporalPort := service.GetTemporalPort()
	uiPort := service.GetUIPort()

	log.Printf(i18n.T("TemporalServerRunningOnPort"), temporalPort)
	log.Printf(i18n.T("TemporalUIAvailableAt"), uiPort)

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
		log.Println(i18n.T("ReceivedShutdownSignal"))

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

	log.Printf(i18n.T("TemporalServiceStartingOnPort"), httpPort)
	log.Printf(i18n.T("HealthEndpoint"), httpPort)
	log.Printf(i18n.T("JobsEndpoint"), httpPort)
	log.Printf(i18n.T("PortsEndpoint"), httpPort)

	if err := server.ListenAndServe(); err != nil && err != http.ErrServerClosed {
		log.Fatalf(i18n.T("HTTPServerFailed"), err)
	}
}
