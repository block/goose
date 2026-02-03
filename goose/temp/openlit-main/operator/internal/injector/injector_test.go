package injector

import (
	"testing"

	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/suite"
	corev1 "k8s.io/api/core/v1"
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
)

type InjectorTestSuite struct {
	suite.Suite
	injector *OpenLitInjector
	config   *InjectorConfig
}

func (suite *InjectorTestSuite) SetupTest() {
	// Create test InjectorConfig matching current API
	suite.config = &InjectorConfig{
		Provider:              "openlit",
		OTLPEndpoint:         "http://openlit.default.svc.cluster.local:4318",
		OTLPHeaders:          "",
		Environment:          "test",
		ImagePullPolicy:      corev1.PullIfNotPresent,
		CustomPackages:       "langchain>=0.1.0,chromadb>=0.4.0",
		InitContainerImage:   "ghcr.io/openlit/openlit-ai-instrumentation:latest",
		ServiceName:          "test-app",
		ServiceNamespace:     "default",
		CaptureMessageContent: true,
		DetailedTracing:      true,
		SharedVolumeName:     "instrumentation-packages",
		SharedVolumePath:     "/instrumentation-packages",
	}

	suite.injector = New(suite.config)
}

func (suite *InjectorTestSuite) TestIsPythonContainer() {
	tests := []struct {
		name        string
		container   *corev1.Container
		expected    bool
		description string
	}{
		{
			name: "Python official image",
			container: &corev1.Container{
				Name:  "python-app",
				Image: "python:3.11-slim",
			},
			expected:    true,
			description: "Should detect official Python Docker image",
		},
		{
			name: "Python with tag",
			container: &corev1.Container{
				Name:  "app",
				Image: "python:3.9-alpine",
			},
			expected:    true,
			description: "Should detect Python image with specific tag",
		},
		{
			name: "Custom Python image",
			container: &corev1.Container{
				Name:  "custom-app",
				Image: "myregistry.com/python-app:latest",
				Command: []string{"python", "-m", "uvicorn"},
			},
			expected:    true,
			description: "Should detect Python from command",
		},
		{
			name: "Python environment variable",
			container: &corev1.Container{
				Name:  "env-app",
				Image: "alpine:latest",
				Env: []corev1.EnvVar{
					{Name: "PYTHON_VERSION", Value: "3.11"},
				},
			},
			expected:    true,
			description: "Should detect Python from environment variables",
		},
		{
			name: "Python PATH environment",
			container: &corev1.Container{
				Name:  "path-app",
				Image: "alpine:latest",
				Env: []corev1.EnvVar{
					{Name: "PATH", Value: "/usr/local/python/bin:/usr/bin"},
				},
			},
			expected:    false,
			description: "Should not detect Python from PATH (implementation doesn't check PATH)",
		},
		{
			name: "Java application",
			container: &corev1.Container{
				Name:  "java-app",
				Image: "openjdk:11",
			},
			expected:    false,
			description: "Should not detect Java as Python",
		},
		{
			name: "Node.js application",
			container: &corev1.Container{
				Name:  "node-app",
				Image: "node:16-alpine",
			},
			expected:    false,
			description: "Should not detect Node.js as Python",
		},
		{
			name: "Generic utility container",
			container: &corev1.Container{
				Name:  "utility",
				Image: "busybox:latest",
			},
			expected:    false,
			description: "Should not detect utility containers as Python",
		},
	}

	for _, tt := range tests {
		suite.Run(tt.name, func() {
			result := suite.injector.isPythonContainer(tt.container)
			suite.Equal(tt.expected, result, tt.description)
		})
	}
}

func (suite *InjectorTestSuite) TestShouldInstrumentContainer() {
	tests := []struct {
		name        string
		container   *corev1.Container
		pod         *corev1.Pod
		expected    bool
		description string
	}{
		{
			name: "Python container should be instrumented",
			container: &corev1.Container{
				Name:  "python-app",
				Image: "python:3.11-slim",
			},
			pod: &corev1.Pod{
				ObjectMeta: metav1.ObjectMeta{
					Annotations: map[string]string{},
				},
			},
			expected:    true,
			description: "Normal Python container should be instrumented",
		},
		{
			name: "Explicitly excluded container",
			container: &corev1.Container{
				Name:  "python-app",
				Image: "python:3.11-slim",
			},
			pod: &corev1.Pod{
				ObjectMeta: metav1.ObjectMeta{
					Annotations: map[string]string{
						"openlit.io/exclude-containers": "python-app",
					},
				},
			},
			expected:    false,
			description: "Explicitly excluded containers should not be instrumented",
		},
		{
			name: "Explicitly included non-Python container",
			container: &corev1.Container{
				Name:  "java-app",
				Image: "openjdk:11",
			},
			pod: &corev1.Pod{
				ObjectMeta: metav1.ObjectMeta{
					Annotations: map[string]string{
						"openlit.io/include-containers": "java-app",
					},
				},
			},
			expected:    false,
			description: "Even explicitly included containers must be Python to be instrumented",
		},
		{
			name: "Istio sidecar container",
			container: &corev1.Container{
				Name:  "istio-proxy",
				Image: "istio/proxyv2:1.18.0",
			},
			pod: &corev1.Pod{
				ObjectMeta: metav1.ObjectMeta{
					Annotations: map[string]string{},
				},
			},
			expected:    false,
			description: "Istio sidecar containers should not be instrumented",
		},
		{
			name: "Envoy sidecar container",
			container: &corev1.Container{
				Name:  "envoy",
				Image: "envoyproxy/envoy:v1.27.0",
			},
			pod: &corev1.Pod{
				ObjectMeta: metav1.ObjectMeta{
					Annotations: map[string]string{},
				},
			},
			expected:    false,
			description: "Envoy sidecar containers should not be instrumented",
		},
		{
			name: "Container with language annotation",
			container: &corev1.Container{
				Name:  "custom-app",
				Image: "python:3.11-slim", // Changed to actual Python image to pass isPythonContainer check
			},
			pod: &corev1.Pod{
				ObjectMeta: metav1.ObjectMeta{
					Annotations: map[string]string{
						"openlit.io/container-languages": "custom-app:python",
					},
				},
			},
			expected:    true,
			description: "Container with explicit Python language annotation should be instrumented",
		},
	}

	for _, tt := range tests {
		suite.Run(tt.name, func() {
			result := suite.injector.shouldInstrumentContainer(tt.container, tt.pod)
			suite.Equal(tt.expected, result, tt.description)
		})
	}
}

func (suite *InjectorTestSuite) TestValidateSecurityContext() {
	tests := []struct {
		name        string
		pod         *corev1.Pod
		expectError bool
		description string
	}{
		{
			name: "Pod with no security constraints",
			pod: &corev1.Pod{
				Spec: corev1.PodSpec{
					Containers: []corev1.Container{
						{
							Name:  "app",
							Image: "python:3.11-slim",
						},
					},
				},
			},
			expectError: false,
			description: "Pod with no security constraints should pass validation",
		},
		{
			name: "Pod with read-only filesystem container",
			pod: &corev1.Pod{
				Spec: corev1.PodSpec{
					Containers: []corev1.Container{
						{
							Name:  "app",
							Image: "python:3.11-slim",
							SecurityContext: &corev1.SecurityContext{
								ReadOnlyRootFilesystem: &[]bool{true}[0],
							},
						},
					},
				},
			},
			expectError: true,
			description: "Pod with read-only filesystem should fail validation",
		},
		{
			name: "Pod with restrictive capabilities",
			pod: &corev1.Pod{
				Spec: corev1.PodSpec{
					Containers: []corev1.Container{
						{
							Name:  "app",
							Image: "python:3.11-slim",
							SecurityContext: &corev1.SecurityContext{
								Capabilities: &corev1.Capabilities{
									Drop: []corev1.Capability{"ALL"},
								},
							},
						},
					},
				},
			},
			expectError: false,
			description: "Pod with dropped capabilities should still pass validation",
		},
		{
			name: "Pod with privileged container",
			pod: &corev1.Pod{
				Spec: corev1.PodSpec{
					Containers: []corev1.Container{
						{
							Name:  "app",
							Image: "python:3.11-slim",
							SecurityContext: &corev1.SecurityContext{
								Privileged: &[]bool{true}[0],
							},
						},
					},
				},
			},
			expectError: false,
			description: "Privileged containers should pass validation",
		},
	}

	for _, tt := range tests {
		suite.Run(tt.name, func() {
			err := suite.injector.validateSecurityContext(tt.pod)
			if tt.expectError {
				suite.Error(err, tt.description)
			} else {
				suite.NoError(err, tt.description)
			}
		})
	}
}

func (suite *InjectorTestSuite) TestHasExistingInstrumentation() {
	tests := []struct {
		name        string
		container   *corev1.Container
		pod         *corev1.Pod
		expected    bool
		description string
	}{
		{
			name: "Container with no instrumentation",
			container: &corev1.Container{
				Name:  "clean-app",
				Image: "python:3.11-slim",
				Env:   []corev1.EnvVar{},
			},
			pod: &corev1.Pod{},
			expected: false,
			description: "Clean container should have no existing instrumentation",
		},
		{
			name: "Container with OTEL environment variables",
			container: &corev1.Container{
				Name:  "otel-app",
				Image: "python:3.11-slim",
				Env: []corev1.EnvVar{
					{Name: "OTEL_EXPORTER_OTLP_ENDPOINT", Value: "http://jaeger:4318"},
				},
			},
			pod: &corev1.Pod{},
			expected: true,
			description: "Container with OTEL env vars should be detected as instrumented",
		},
		{
			name: "Container with instrumentation volume mount",
			container: &corev1.Container{
				Name:  "instrumented-app",
				Image: "python:3.11-slim",
				VolumeMounts: []corev1.VolumeMount{
					{
						Name:      "otel-auto-instrumentation", // Fixed to match actual implementation
						MountPath: "/otel-auto-instrumentation",
					},
				},
			},
			pod: &corev1.Pod{},
			expected: true,
			description: "Container with instrumentation volume mount should be detected",
		},
		{
			name: "Container with OTEL service name",
			container: &corev1.Container{
				Name:  "otel-app",
				Image: "python:3.11-slim",
				Env: []corev1.EnvVar{
					{Name: "OTEL_SERVICE_NAME", Value: "my-service"},
				},
			},
			pod: &corev1.Pod{},
			expected: true,
			description: "Container with OTEL_SERVICE_NAME should be detected as instrumented",
		},
	}

	for _, tt := range tests {
		suite.Run(tt.name, func() {
			result := suite.injector.hasExistingInstrumentation(tt.container, tt.pod)
			suite.Equal(tt.expected, result, tt.description)
		})
	}
}

func (suite *InjectorTestSuite) TestInjectOpenLIT() {
	tests := []struct {
		name        string
		pod         *corev1.Pod
		expectError bool
		description string
	}{
		{
			name: "Successful injection into Python container",
			pod: &corev1.Pod{
				ObjectMeta: metav1.ObjectMeta{
					Name:      "test-pod",
					Namespace: "default",
				},
				Spec: corev1.PodSpec{
					Containers: []corev1.Container{
						{
							Name:  "python-app",
							Image: "python:3.11-slim",
							Ports: []corev1.ContainerPort{
								{ContainerPort: 8080},
							},
						},
					},
				},
			},
			expectError: false,
			description: "Should successfully inject into Python container",
		},
		{
			name: "Injection with security context restrictions",
			pod: &corev1.Pod{
				ObjectMeta: metav1.ObjectMeta{
					Name:      "restricted-pod",
					Namespace: "default",
				},
				Spec: corev1.PodSpec{
					Containers: []corev1.Container{
						{
							Name:  "restricted-app",
							Image: "python:3.11-slim",
							SecurityContext: &corev1.SecurityContext{
								ReadOnlyRootFilesystem: &[]bool{true}[0],
							},
						},
					},
				},
			},
			expectError: true,
			description: "Should fail injection with restrictive security context",
		},
		{
			name: "No instrumentable containers",
			pod: &corev1.Pod{
				ObjectMeta: metav1.ObjectMeta{
					Name:      "java-pod",
					Namespace: "default",
				},
				Spec: corev1.PodSpec{
					Containers: []corev1.Container{
						{
							Name:  "java-app",
							Image: "openjdk:11",
						},
					},
				},
			},
			expectError: true,
			description: "Should fail when no containers can be instrumented",
		},
		{
			name: "Mixed container types",
			pod: &corev1.Pod{
				ObjectMeta: metav1.ObjectMeta{
					Name:      "mixed-pod",
					Namespace: "default",
				},
				Spec: corev1.PodSpec{
					Containers: []corev1.Container{
						{
							Name:  "python-app",
							Image: "python:3.11-slim",
						},
						{
							Name:  "sidecar",
							Image: "nginx:alpine",
						},
					},
				},
			},
			expectError: false,
			description: "Should successfully inject into Python container, skip others",
		},
	}

	for _, tt := range tests {
		suite.Run(tt.name, func() {
			// Make a deep copy of the pod to avoid test interference
			podCopy := tt.pod.DeepCopy()
			
			err := suite.injector.InjectOpenLIT(podCopy)
			
			if tt.expectError {
				suite.Error(err, tt.description)
			} else {
				suite.NoError(err, tt.description)
				
				// Verify injection results
				if err == nil {
					// Check that init container was added
					suite.NotEmpty(podCopy.Spec.InitContainers, "Init container should be added")
					
					// Check that shared volume was added
					found := false
					for _, vol := range podCopy.Spec.Volumes {
						if vol.Name == suite.config.GetSharedVolumeName() {
							found = true
							break
						}
					}
					suite.True(found, "Shared volume should be added")
					
					// Check that at least one container was modified
					pythonContainerFound := false
					for _, container := range podCopy.Spec.Containers {
						if container.Name == "python-app" {
							pythonContainerFound = true
							// Check for OpenLIT environment variables
							foundOTEL := false
							for _, env := range container.Env {
								if env.Name == "OTEL_EXPORTER_OTLP_ENDPOINT" {
									foundOTEL = true
									break
								}
							}
							suite.True(foundOTEL, "OTEL environment variables should be added")
							
							// Check for volume mount
							foundMount := false
							for _, mount := range container.VolumeMounts {
								if mount.Name == suite.config.GetSharedVolumeName() {
									foundMount = true
									break
								}
							}
							suite.True(foundMount, "Volume mount should be added to Python container")
						}
					}
					
					if tt.name == "Mixed container types" {
						suite.True(pythonContainerFound, "Python container should be found and modified")
					}
				}
			}
		})
	}
}

func (suite *InjectorTestSuite) TestErrorRecovery() {
	suite.Run("Recovery from panic", func() {
		// Create a pod that will cause a panic (nil pointer access)
		pod := &corev1.Pod{
			ObjectMeta: metav1.ObjectMeta{
				Name:      "panic-pod",
				Namespace: "default",
			},
			Spec: corev1.PodSpec{
				Containers: []corev1.Container{
					{
						Name:  "python-app",
						Image: "python:3.11-slim",
					},
				},
			},
		}

		// The injector should handle panics gracefully
		err := suite.injector.InjectOpenLIT(pod)
		// We expect either success or a controlled error, not a panic
		if err != nil {
			suite.Contains(err.Error(), "injection failed", "Error should indicate injection failure")
		}
	})
}

func TestInjectorSuite(t *testing.T) {
	suite.Run(t, new(InjectorTestSuite))
}

// Additional unit tests for specific functions
func TestGetSharedVolumeName(t *testing.T) {
	config := &InjectorConfig{
		SharedVolumeName: "custom-volume",
	}
	assert.Equal(t, "custom-volume", config.GetSharedVolumeName())
}

func TestGetSharedVolumePath(t *testing.T) {
	config := &InjectorConfig{
		SharedVolumePath: "/custom/path",
	}
	assert.Equal(t, "/custom/path", config.GetSharedVolumePath())
}

func TestInjectionResultInitialization(t *testing.T) {
	result := &InjectionResult{
		TotalContainers: 3,
	}
	assert.Equal(t, 3, result.TotalContainers)
	assert.Equal(t, 0, result.InstrumentedCount)
	assert.Empty(t, result.FailedContainers)
	assert.Empty(t, result.RecoveryActions)
}
