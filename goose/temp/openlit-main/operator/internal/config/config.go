package config

// This file provides the main configuration interface for the OpenLIT operator
// All configuration logic is now centralized and schema-driven

// GetConfig loads and validates the operator configuration from environment variables
// This is the main entry point for getting operator configuration
func GetConfig() (*OperatorConfig, error) {
	return LoadFromEnvironment()
}

// MustGetConfig loads the operator configuration and panics on error
// Use this only in main() or test setup where failure should stop the program
func MustGetConfig() *OperatorConfig {
	config, err := GetConfig()
	if err != nil {
		panic("Failed to load operator configuration: " + err.Error())
	}
	return config
}
