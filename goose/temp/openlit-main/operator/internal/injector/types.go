package injector

import (
	"fmt"
	"strings"

	corev1 "k8s.io/api/core/v1"
)

// InjectorConfig contains all configuration needed for instrumentation injection
// This config is created from AutoInstrumentation CR and operator config
type InjectorConfig struct {
	// From AutoInstrumentation CR
	Provider           string            `json:"provider"`
	OTLPEndpoint       string            `json:"otlpEndpoint"`
	OTLPHeaders        string            `json:"otlpHeaders,omitempty"`
	Environment        string            `json:"environment,omitempty"`
	ImagePullPolicy    corev1.PullPolicy `json:"imagePullPolicy"`
	CustomPackages     string            `json:"customPackages,omitempty"`
	CustomInitImage    string            `json:"customInitImage,omitempty"`
	InitContainerImage string            `json:"initContainerImage"`

	// Instrumentation settings
	ServiceName           string `json:"serviceName"`
	ServiceNamespace      string `json:"serviceNamespace"`
	CaptureMessageContent bool   `json:"captureMessageContent"`
	DetailedTracing       bool   `json:"detailedTracing"`

	// Volume and mount configuration
	SharedVolumeName string `json:"sharedVolumeName"`
	SharedVolumePath string `json:"sharedVolumePath"`

	// Environment variables to inject
	EnvVars []corev1.EnvVar `json:"envVars,omitempty"`
}

// Validate validates the injector configuration
func (c *InjectorConfig) Validate() error {
	if c.Provider == "" {
		return fmt.Errorf("provider is required")
	}

	if c.OTLPEndpoint == "" {
		return fmt.Errorf("OTLP endpoint is required")
	}

	if c.InitContainerImage == "" && c.CustomInitImage == "" {
		return fmt.Errorf("either InitContainerImage or CustomInitImage must be specified")
	}

	return nil
}

// GetInitContainerImage returns the appropriate init container image
func (c *InjectorConfig) GetInitContainerImage() string {
	if c.CustomInitImage != "" {
		return c.CustomInitImage
	}
	return c.InitContainerImage
}

// GetContainerName returns the init container name based on provider
func (c *InjectorConfig) GetContainerName() string {
	if c.Provider == "" {
		return "auto-instrumentation"
	}
	return "auto-instrumentation-" + c.Provider
}

// HasCustomPackages returns true if custom packages are specified
func (c *InjectorConfig) HasCustomPackages() bool {
	return c.CustomPackages != ""
}

// GetCustomPackagesList returns the custom packages as a slice
func (c *InjectorConfig) GetCustomPackagesList() []string {
	if c.CustomPackages == "" {
		return nil
	}

	packages := strings.Split(c.CustomPackages, ",")
	result := make([]string, 0, len(packages))

	for _, pkg := range packages {
		if trimmed := strings.TrimSpace(pkg); trimmed != "" {
			result = append(result, trimmed)
		}
	}

	return result
}

// GetSharedVolumeName returns the shared volume name for instrumentation packages
func (c *InjectorConfig) GetSharedVolumeName() string {
	if c.SharedVolumeName != "" {
		return c.SharedVolumeName
	}
	return "instrumentation-packages"
}

// GetSharedVolumePath returns the mount path for the shared volume
func (c *InjectorConfig) GetSharedVolumePath() string {
	if c.SharedVolumePath != "" {
		return c.SharedVolumePath
	}
	return "/instrumentation-packages"
}
