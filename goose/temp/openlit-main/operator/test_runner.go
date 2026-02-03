package main

import (
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"time"
)

// TestSuite represents a test suite with its metadata
type TestSuite struct {
	Name        string
	Package     string
	Description string
	Tags        []string
}

// TestResult represents the result of running a test suite
type TestResult struct {
	Suite    TestSuite
	Success  bool
	Duration time.Duration
	Output   string
}

var testSuites = []TestSuite{
	{
		Name:        "Configuration Tests",
		Package:     "./internal/config",
		Description: "Tests for operator configuration loading, validation, and environment variable handling",
		Tags:        []string{"unit", "config"},
	},
	{
		Name:        "Validation Tests",
		Package:     "./internal/validation",
		Description: "Tests for AutoInstrumentation CR validation, edge cases, and error scenarios",
		Tags:        []string{"unit", "validation"},
	},
	{
		Name:        "Observability Tests",
		Package:     "./internal/observability",
		Description: "Tests for OpenTelemetry setup, OTLP connectivity, and logr integration",
		Tags:        []string{"unit", "observability"},
	},
	{
		Name:        "Injector Tests",
		Package:     "./internal/injector",
		Description: "Tests for container selection, security validation, error recovery, and Python detection",
		Tags:        []string{"unit", "injector"},
	},
	{
		Name:        "Controller Tests",
		Package:     "./internal/controller",
		Description: "Tests for reconciliation logic, multiple CR scenarios, and status updates",
		Tags:        []string{"unit", "controller"},
	},
	{
		Name:        "Webhook Handler Tests",
		Package:     "./internal/webhook",
		Description: "Tests for admission requests, selector matching, ignore logic, and circuit breaker",
		Tags:        []string{"unit", "webhook"},
	},
	{
		Name:        "Integration Tests",
		Package:     "./internal/testing",
		Description: "End-to-end integration tests with real Kubernetes API using envtest",
		Tags:        []string{"integration", "e2e"},
	},
}

func main() {
	fmt.Println("🧪 OpenLIT Operator Test Suite Runner")
	fmt.Println("=====================================")
	fmt.Println()

	// Parse command line arguments
	args := os.Args[1:]
	var selectedTags []string
	var verbose bool
	var coverage bool
	var parallel bool

	for _, arg := range args {
		switch {
		case strings.HasPrefix(arg, "--tags="):
			selectedTags = strings.Split(strings.TrimPrefix(arg, "--tags="), ",")
		case arg == "--verbose" || arg == "-v":
			verbose = true
		case arg == "--coverage" || arg == "-c":
			coverage = true
		case arg == "--parallel" || arg == "-p":
			parallel = true
		case arg == "--help" || arg == "-h":
			printHelp()
			return
		}
	}

	// Filter test suites by tags
	suitesToRun := filterSuitesByTags(testSuites, selectedTags)

	if len(suitesToRun) == 0 {
		fmt.Println("❌ No test suites match the specified criteria")
		return
	}

	fmt.Printf("📋 Running %d test suite(s)\n", len(suitesToRun))
	if len(selectedTags) > 0 {
		fmt.Printf("🏷️  Filtered by tags: %s\n", strings.Join(selectedTags, ", "))
	}
	fmt.Println()

	// Run test suites
	results := make([]TestResult, 0, len(suitesToRun))

	for i, suite := range suitesToRun {
		fmt.Printf("[%d/%d] %s\n", i+1, len(suitesToRun), suite.Name)
		fmt.Printf("📦 Package: %s\n", suite.Package)
		fmt.Printf("📝 %s\n", suite.Description)
		fmt.Printf("🏷️  Tags: %s\n", strings.Join(suite.Tags, ", "))

		result := runTestSuite(suite, verbose, coverage, parallel)
		results = append(results, result)

		if result.Success {
			fmt.Printf("✅ PASSED in %v\n", result.Duration)
		} else {
			fmt.Printf("❌ FAILED in %v\n", result.Duration)
			if !verbose {
				fmt.Println("📄 Output:")
				fmt.Println(result.Output)
			}
		}
		fmt.Println()
	}

	// Print summary
	printSummary(results)

	// Exit with appropriate code
	if hasFailures(results) {
		os.Exit(1)
	}
}

func runTestSuite(suite TestSuite, verbose, coverage, parallel bool) TestResult {
	start := time.Now()

	// Build test command
	cmd := exec.Command("go", "test")
	cmd.Args = append(cmd.Args, suite.Package)

	if verbose {
		cmd.Args = append(cmd.Args, "-v")
	}

	if coverage {
		cmd.Args = append(cmd.Args, "-cover")
		cmd.Args = append(cmd.Args, "-coverprofile="+generateCoverageFile(suite))
	}

	if parallel {
		cmd.Args = append(cmd.Args, "-parallel", "4")
	}

	// Add timeout
	cmd.Args = append(cmd.Args, "-timeout", "10m")

	// Set environment variables
	cmd.Env = os.Environ()

	// Run the command
	output, err := cmd.CombinedOutput()

	duration := time.Since(start)
	success := err == nil

	if verbose {
		fmt.Println("📄 Output:")
		fmt.Println(string(output))
	}

	return TestResult{
		Suite:    suite,
		Success:  success,
		Duration: duration,
		Output:   string(output),
	}
}

func filterSuitesByTags(suites []TestSuite, tags []string) []TestSuite {
	if len(tags) == 0 {
		return suites
	}

	var filtered []TestSuite
	for _, suite := range suites {
		if hasMatchingTag(suite.Tags, tags) {
			filtered = append(filtered, suite)
		}
	}
	return filtered
}

func hasMatchingTag(suiteTags, filterTags []string) bool {
	for _, filterTag := range filterTags {
		for _, suiteTag := range suiteTags {
			if suiteTag == filterTag {
				return true
			}
		}
	}
	return false
}

func generateCoverageFile(suite TestSuite) string {
	packageName := filepath.Base(suite.Package)
	if packageName == "." {
		packageName = "all"
	}
	return fmt.Sprintf("coverage-%s.out", packageName)
}

func printSummary(results []TestResult) {
	fmt.Println("📊 Test Summary")
	fmt.Println("===============")
	fmt.Println()

	totalTests := len(results)
	passedTests := 0
	totalDuration := time.Duration(0)

	for _, result := range results {
		status := "❌ FAILED"
		if result.Success {
			status = "✅ PASSED"
			passedTests++
		}

		fmt.Printf("%-40s %s (%v)\n", result.Suite.Name, status, result.Duration)
		totalDuration += result.Duration
	}

	fmt.Println()
	fmt.Printf("📈 Results: %d/%d tests passed\n", passedTests, totalTests)
	fmt.Printf("⏱️  Total time: %v\n", totalDuration)

	if passedTests == totalTests {
		fmt.Println("🎉 All tests passed!")
	} else {
		fmt.Printf("⚠️  %d test suite(s) failed\n", totalTests-passedTests)
	}
}

func hasFailures(results []TestResult) bool {
	for _, result := range results {
		if !result.Success {
			return true
		}
	}
	return false
}

func printHelp() {
	fmt.Println("OpenLIT Operator Test Suite Runner")
	fmt.Println()
	fmt.Println("Usage:")
	fmt.Println("  go run test_runner.go [OPTIONS]")
	fmt.Println()
	fmt.Println("Options:")
	fmt.Println("  --tags=TAG1,TAG2     Run only tests with specified tags")
	fmt.Println("  --verbose, -v        Show verbose test output")
	fmt.Println("  --coverage, -c       Generate coverage reports")
	fmt.Println("  --parallel, -p       Run tests in parallel")
	fmt.Println("  --help, -h           Show this help message")
	fmt.Println()
	fmt.Println("Available tags:")
	fmt.Println("  unit                 Unit tests")
	fmt.Println("  integration          Integration tests")
	fmt.Println("  e2e                  End-to-end tests")
	fmt.Println("  config               Configuration tests")
	fmt.Println("  validation           Validation tests")
	fmt.Println("  observability        Observability tests")
	fmt.Println("  injector             Injector tests")
	fmt.Println("  controller           Controller tests")
	fmt.Println("  webhook              Webhook tests")
	fmt.Println()
	fmt.Println("Examples:")
	fmt.Println("  go run test_runner.go                    # Run all tests")
	fmt.Println("  go run test_runner.go --tags=unit        # Run only unit tests")
	fmt.Println("  go run test_runner.go --tags=integration # Run only integration tests")
	fmt.Println("  go run test_runner.go --verbose --coverage # Run with verbose output and coverage")
}
