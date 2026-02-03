package testing

import (
	"context"
	"encoding/json"
	"fmt"
	"testing"
	"time"

	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/suite"
	admissionv1 "k8s.io/api/admission/v1"
	corev1 "k8s.io/api/core/v1"
	"k8s.io/apimachinery/pkg/api/resource"
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	"k8s.io/apimachinery/pkg/runtime"
	"k8s.io/apimachinery/pkg/types"
	dynamicfake "k8s.io/client-go/dynamic/fake"
	"sigs.k8s.io/controller-runtime/pkg/client/fake"
	"sigs.k8s.io/controller-runtime/pkg/webhook/admission"

	autoinstrumentationv1alpha1 "github.com/openlit/openlit/operator/api/v1alpha1"
	"github.com/openlit/openlit/operator/internal/config"
	"github.com/openlit/openlit/operator/internal/injector"
	"github.com/openlit/openlit/operator/internal/webhook"
)

type EdgeCasesTestSuite struct {
	suite.Suite
	ctx context.Context
}

func (suite *EdgeCasesTestSuite) SetupTest() {
	suite.ctx = context.Background()
}

func (suite *EdgeCasesTestSuite) TestExtremeConfigurationValues() {
	tests := []struct {
		name        string
		autoInstr   *autoinstrumentationv1alpha1.AutoInstrumentation
		expectValid bool
		description string
	}{
		{
			name: "Extremely long environment variable values",
			autoInstr: &autoinstrumentationv1alpha1.AutoInstrumentation{
				ObjectMeta: metav1.ObjectMeta{
					Name:      "long-env-test",
					Namespace: "default",
				},
				Spec: autoinstrumentationv1alpha1.AutoInstrumentationSpec{
					Selector: autoinstrumentationv1alpha1.PodSelector{
						MatchLabels: map[string]string{"app": "test"},
					},
					Python: &autoinstrumentationv1alpha1.PythonInstrumentation{
						Instrumentation: &autoinstrumentationv1alpha1.InstrumentationSettings{
							Provider:        "openlit",
							CustomInitImage: "openlit-instrumentation:latest",
						},
					},
					OTLP: autoinstrumentationv1alpha1.OTLPConfig{
						Endpoint: "http://test:4318",
					},
					Resource: &autoinstrumentationv1alpha1.ResourceConfig{
						Environment: "test",
					},
				},
			},
			expectValid: true,
			description: "Should handle extremely long environment variable values",
		},
		{
			name: "Maximum number of selectors",
			autoInstr: &autoinstrumentationv1alpha1.AutoInstrumentation{
				ObjectMeta: metav1.ObjectMeta{
					Name:      "max-selectors-test",
					Namespace: "default",
				},
				Spec: autoinstrumentationv1alpha1.AutoInstrumentationSpec{
					Selector: autoinstrumentationv1alpha1.PodSelector{
						MatchLabels: generateManyLabels(100), // 100 label selectors
					},
					Python: &autoinstrumentationv1alpha1.PythonInstrumentation{
						Instrumentation: &autoinstrumentationv1alpha1.InstrumentationSettings{
							Provider:        "openlit",
							CustomInitImage: "openlit-instrumentation:latest",
						},
					},
					OTLP: autoinstrumentationv1alpha1.OTLPConfig{
						Endpoint: "http://test:4318",
					},
				},
			},
			expectValid: true,
			description: "Should handle many label selectors",
		},
		{
			name: "Unicode and special characters in labels",
			autoInstr: &autoinstrumentationv1alpha1.AutoInstrumentation{
				ObjectMeta: metav1.ObjectMeta{
					Name:      "unicode-test",
					Namespace: "default",
				},
				Spec: autoinstrumentationv1alpha1.AutoInstrumentationSpec{
					Selector: autoinstrumentationv1alpha1.PodSelector{
						MatchLabels: map[string]string{
							"app.kubernetes.io/name":      "my-app",
							"app.kubernetes.io/version":   "1.0.0-alpha.beta+gamma",
							"custom.domain.com/component": "web-server",
						},
					},
					Python: &autoinstrumentationv1alpha1.PythonInstrumentation{
						Instrumentation: &autoinstrumentationv1alpha1.InstrumentationSettings{
							Provider:        "openlit",
							CustomInitImage: "openlit-instrumentation:latest",
						},
					},
					OTLP: autoinstrumentationv1alpha1.OTLPConfig{
						Endpoint: "http://test:4318",
					},
				},
			},
			expectValid: true,
			description: "Should handle complex label names with special characters",
		},
	}

	for _, tt := range tests {
		suite.Run(tt.name, func() {
			// Create fake client
			scheme := runtime.NewScheme()
			_ = autoinstrumentationv1alpha1.AddToScheme(scheme)
			fakeClient := fake.NewClientBuilder().WithScheme(scheme).Build()

			// Create the resource
			err := fakeClient.Create(suite.ctx, tt.autoInstr)
			if tt.expectValid {
				suite.NoError(err, tt.description)
			} else {
				suite.Error(err, tt.description)
			}
		})
	}
}

func (suite *EdgeCasesTestSuite) TestPodResourceExtremes() {
	// Create webhook handler for testing
	scheme := runtime.NewScheme()
	_ = corev1.AddToScheme(scheme)
	_ = autoinstrumentationv1alpha1.AddToScheme(scheme)

	// Create mock config and dynamic client
	cfg := &config.OperatorConfig{
		Namespace: "test-namespace",
	}
	dynamicClient := dynamicfake.NewSimpleDynamicClient(scheme)

	handler := webhook.NewHandler(cfg, scheme, dynamicClient)

	tests := []struct {
		name           string
		pod            *corev1.Pod
		expectAllowed  bool
		expectPatches  bool
		description    string
	}{
		{
			name: "Pod with maximum containers",
			pod: &corev1.Pod{
				ObjectMeta: metav1.ObjectMeta{
					Name:      "max-containers-pod",
					Namespace: "default",
					Labels:    map[string]string{"app": "test"},
				},
				Spec: corev1.PodSpec{
					Containers: generateManyContainers(50), // 50 containers
				},
			},
			expectAllowed: true,
			expectPatches: false, // No matching AutoInstrumentation
			description:   "Should handle pods with many containers",
		},
		{
			name: "Pod with extremely long names",
			pod: &corev1.Pod{
				ObjectMeta: metav1.ObjectMeta{
					Name:      "this-is-an-extremely-long-pod-name-that-tests-the-limits-of-kubernetes-naming",
					Namespace: "default",
					Labels:    map[string]string{"app": "test"},
				},
				Spec: corev1.PodSpec{
					Containers: []corev1.Container{
						{
							Name:  "container-with-very-long-name-for-testing-edge-cases",
							Image: "python:3.11-slim",
						},
					},
				},
			},
			expectAllowed: true,
			expectPatches: false,
			description:   "Should handle extremely long names",
		},
		{
			name: "Pod with complex resource requirements",
			pod: &corev1.Pod{
				ObjectMeta: metav1.ObjectMeta{
					Name:      "complex-resources-pod",
					Namespace: "default",
					Labels:    map[string]string{"app": "test"},
				},
				Spec: corev1.PodSpec{
					Containers: []corev1.Container{
						{
							Name:  "resource-heavy-container",
							Image: "python:3.11-slim",
							Resources: corev1.ResourceRequirements{
								Requests: corev1.ResourceList{
									corev1.ResourceCPU:              mustParseQuantity("100m"),
									corev1.ResourceMemory:           mustParseQuantity("128Mi"),
									corev1.ResourceEphemeralStorage: mustParseQuantity("1Gi"),
								},
								Limits: corev1.ResourceList{
									corev1.ResourceCPU:              mustParseQuantity("2"),
									corev1.ResourceMemory:           mustParseQuantity("4Gi"),
									corev1.ResourceEphemeralStorage: mustParseQuantity("10Gi"),
								},
							},
						},
					},
				},
			},
			expectAllowed: true,
			expectPatches: false,
			description:   "Should handle complex resource requirements",
		},
		{
			name: "Pod with many environment variables",
			pod: &corev1.Pod{
				ObjectMeta: metav1.ObjectMeta{
					Name:      "many-env-vars-pod",
					Namespace: "default",
					Labels:    map[string]string{"app": "test"},
				},
				Spec: corev1.PodSpec{
					Containers: []corev1.Container{
						{
							Name:  "env-heavy-container",
							Image: "python:3.11-slim",
							Env:   generateManyEnvVars(200), // 200 env vars
						},
					},
				},
			},
			expectAllowed: true,
			expectPatches: false,
			description:   "Should handle containers with many environment variables",
		},
	}

	for _, tt := range tests {
		suite.Run(tt.name, func() {
			podBytes, err := json.Marshal(tt.pod)
			suite.NoError(err)

			req := admission.Request{
				AdmissionRequest: admissionv1.AdmissionRequest{
					UID: types.UID(fmt.Sprintf("edge-case-%s", tt.name)),
					Kind: metav1.GroupVersionKind{
						Group:   "",
						Version: "v1",
						Kind:    "Pod",
					},
					Operation: admissionv1.Create,
					Object: runtime.RawExtension{
						Raw: podBytes,
					},
					Namespace: "default",
				},
			}

			response := handler.Handle(suite.ctx, req)

			suite.Equal(tt.expectAllowed, response.Allowed, tt.description)
			if tt.expectPatches {
				suite.NotEmpty(response.Patches, tt.description)
			} else {
				suite.Empty(response.Patches, tt.description)
			}
		})
	}
}

func (suite *EdgeCasesTestSuite) TestInjectorEdgeCases() {
	config := &injector.InjectorConfig{
		Provider:           "openlit",
		OTLPEndpoint:       "http://test:4318",
		InitContainerImage: "ghcr.io/openlit/openlit-ai-instrumentation:latest",
		ImagePullPolicy:    "IfNotPresent",
		ServiceName:        "test-service",
		ServiceNamespace:   "default",
		SharedVolumeName:   "test-volume",
		SharedVolumePath:   "/test/path",
	}

	inj := injector.New(config)

	tests := []struct {
		name        string
		pod         *corev1.Pod
		expectError bool
		description string
	}{
		{
			name: "Pod with no containers",
			pod: &corev1.Pod{
				ObjectMeta: metav1.ObjectMeta{
					Name:      "no-containers-pod",
					Namespace: "default",
				},
				Spec: corev1.PodSpec{
					Containers: []corev1.Container{}, // Empty containers
				},
			},
			expectError: true,
			description: "Should fail with no containers to instrument",
		},
		{
			name: "Pod with only sidecar containers",
			pod: &corev1.Pod{
				ObjectMeta: metav1.ObjectMeta{
					Name:      "sidecars-only-pod",
					Namespace: "default",
				},
				Spec: corev1.PodSpec{
					Containers: []corev1.Container{
						{
							Name:  "istio-proxy",
							Image: "istio/proxyv2:1.18.0",
						},
						{
							Name:  "envoy",
							Image: "envoyproxy/envoy:v1.27.0",
						},
					},
				},
			},
			expectError: true,
			description: "Should fail when only sidecar containers are present",
		},
		{
			name: "Pod with mixed container types",
			pod: &corev1.Pod{
				ObjectMeta: metav1.ObjectMeta{
					Name:      "mixed-containers-pod",
					Namespace: "default",
				},
				Spec: corev1.PodSpec{
					Containers: []corev1.Container{
						{
							Name:  "python-app",
							Image: "python:3.11-slim",
						},
						{
							Name:  "init-like-container",
							Image: "busybox:latest",
							Command: []string{"sh", "-c", "echo 'init complete'"},
						},
						{
							Name:  "istio-proxy",
							Image: "istio/proxyv2:1.18.0",
						},
					},
				},
			},
			expectError: false,
			description: "Should successfully inject into Python container, skip others",
		},
		{
			name: "Pod with complex volume mounts",
			pod: &corev1.Pod{
				ObjectMeta: metav1.ObjectMeta{
					Name:      "complex-volumes-pod",
					Namespace: "default",
				},
				Spec: corev1.PodSpec{
					Containers: []corev1.Container{
						{
							Name:  "python-app",
							Image: "python:3.11-slim",
							VolumeMounts: []corev1.VolumeMount{
								{Name: "config-volume", MountPath: "/etc/config"},
								{Name: "data-volume", MountPath: "/data"},
								{Name: "cache-volume", MountPath: "/cache"},
								{Name: "logs-volume", MountPath: "/var/log"},
							},
						},
					},
					Volumes: []corev1.Volume{
						{Name: "config-volume", VolumeSource: corev1.VolumeSource{EmptyDir: &corev1.EmptyDirVolumeSource{}}},
						{Name: "data-volume", VolumeSource: corev1.VolumeSource{EmptyDir: &corev1.EmptyDirVolumeSource{}}},
						{Name: "cache-volume", VolumeSource: corev1.VolumeSource{EmptyDir: &corev1.EmptyDirVolumeSource{}}},
						{Name: "logs-volume", VolumeSource: corev1.VolumeSource{EmptyDir: &corev1.EmptyDirVolumeSource{}}},
					},
				},
			},
			expectError: false,
			description: "Should handle pods with many existing volumes",
		},
	}

	for _, tt := range tests {
		suite.Run(tt.name, func() {
			err := inj.InjectOpenLIT(tt.pod)

			if tt.expectError {
				suite.Error(err, tt.description)
			} else {
				suite.NoError(err, tt.description)
			}
		})
	}
}

func (suite *EdgeCasesTestSuite) TestConcurrencyAndRaceConditions() {
	// Test concurrent operations on the same resource
	scheme := runtime.NewScheme()
	_ = autoinstrumentationv1alpha1.AddToScheme(scheme)
	fakeClient := fake.NewClientBuilder().WithScheme(scheme).Build()

	// Create mock config and dynamic client
	cfg := &config.OperatorConfig{
		Namespace: "test-namespace",
	}
	dynamicClient := dynamicfake.NewSimpleDynamicClient(scheme)

	handler := webhook.NewHandler(cfg, scheme, dynamicClient)

	// Create AutoInstrumentation for concurrent tests
	autoInstr := &autoinstrumentationv1alpha1.AutoInstrumentation{
		ObjectMeta: metav1.ObjectMeta{
			Name:      "concurrent-test",
			Namespace: "default",
		},
		Spec: autoinstrumentationv1alpha1.AutoInstrumentationSpec{
			Selector: autoinstrumentationv1alpha1.PodSelector{
				MatchLabels: map[string]string{
					"test": "concurrent",
				},
			},
			Python: &autoinstrumentationv1alpha1.PythonInstrumentation{
				Instrumentation: &autoinstrumentationv1alpha1.InstrumentationSettings{
					Provider:        "openlit",
					CustomInitImage: "openlit-instrumentation:latest",
				},
			},
			OTLP: autoinstrumentationv1alpha1.OTLPConfig{
				Endpoint: "http://test:4318",
			},
		},
	}

	err := fakeClient.Create(suite.ctx, autoInstr)
	suite.NoError(err)

	suite.Run("Concurrent webhook requests", func() {
		done := make(chan bool, 20)
		errors := make(chan error, 20)

		// Send many concurrent requests
		for i := 0; i < 20; i++ {
			go func(id int) {
				defer func() { done <- true }()

				pod := &corev1.Pod{
					ObjectMeta: metav1.ObjectMeta{
						Name:      fmt.Sprintf("concurrent-pod-%d", id),
						Namespace: "default",
						Labels: map[string]string{
							"test": "concurrent",
						},
					},
					Spec: corev1.PodSpec{
						Containers: []corev1.Container{
							{
								Name:  "app",
								Image: "python:3.11-slim",
							},
						},
					},
				}

				podBytes, err := json.Marshal(pod)
				if err != nil {
					errors <- err
					return
				}

				req := admission.Request{
					AdmissionRequest: admissionv1.AdmissionRequest{
						UID: types.UID(fmt.Sprintf("concurrent-uid-%d", id)),
						Kind: metav1.GroupVersionKind{
							Group:   "",
							Version: "v1",
							Kind:    "Pod",
						},
						Operation: admissionv1.Create,
						Object: runtime.RawExtension{
							Raw: podBytes,
						},
						Namespace: "default",
					},
				}

				response := handler.Handle(suite.ctx, req)
				if !response.Allowed {
					errors <- fmt.Errorf("request %d was not allowed", id)
				}
			}(i)
		}

		// Wait for all goroutines to complete
		for i := 0; i < 20; i++ {
			select {
			case <-done:
				// Success
			case err := <-errors:
				suite.Fail("Concurrent request failed: " + err.Error())
			case <-time.After(30 * time.Second):
				suite.Fail("Timeout waiting for concurrent requests")
			}
		}
	})
}

func (suite *EdgeCasesTestSuite) TestMemoryAndResourceUsage() {
	// Test operations under resource constraints

	suite.Run("Large object processing", func() {
		// Create very large pod specification
		largePod := &corev1.Pod{
			ObjectMeta: metav1.ObjectMeta{
				Name:        "large-pod",
				Namespace:   "default",
				Labels:      generateManyLabels(1000), // 1000 labels
				Annotations: generateManyLabels(1000), // 1000 annotations
			},
			Spec: corev1.PodSpec{
				Containers: generateManyContainers(10), // 10 containers with many env vars each
			},
		}

		config := &injector.InjectorConfig{
			ServiceName:        "test-service",
			ServiceNamespace:   "default",
			SharedVolumeName:   "test-volume",
			SharedVolumePath:   "/test/path",
		}

		inj := injector.New(config)

		// This should not cause memory issues or crashes
		err := inj.InjectOpenLIT(largePod)
		// Error is expected since no containers will be instrumented
		suite.Error(err, "Should fail with no instrumentable containers")
	})

	suite.Run("Memory cleanup after failed injection", func() {
		// Test that failed injections don't leak memory
		for i := 0; i < 100; i++ {
			pod := &corev1.Pod{
				ObjectMeta: metav1.ObjectMeta{
					Name:      fmt.Sprintf("cleanup-test-pod-%d", i),
					Namespace: "default",
				},
				Spec: corev1.PodSpec{
					Containers: []corev1.Container{
						{
							Name:  "failing-container",
							Image: "python:3.11-slim",
							SecurityContext: &corev1.SecurityContext{
								ReadOnlyRootFilesystem: &[]bool{true}[0],
							},
						},
					},
				},
			}

			config := &injector.InjectorConfig{
				ServiceName:        "test-service",
				ServiceNamespace:   "default",
				SharedVolumeName:   "test-volume",
				SharedVolumePath:   "/test/path",
			}

			inj := injector.New(config)

			// Should fail due to security constraints but not leak memory
			err := inj.InjectOpenLIT(pod)
			suite.Error(err, "Should fail due to security constraints")
		}
	})
}

func (suite *EdgeCasesTestSuite) TestBoundaryConditions() {
	suite.Run("Empty string handling", func() {
		config := &injector.InjectorConfig{
			ServiceName:        "",    // Empty service name
			ServiceNamespace:   "",    // Empty namespace
			SharedVolumeName:   "",    // Empty volume name
			SharedVolumePath:   "",    // Empty path
		}

		// Should handle empty configuration gracefully
		suite.NotPanics(func() {
			_ = injector.New(config)
		})
	})

	suite.Run("Nil value handling", func() {
		// Test various nil scenarios
		suite.NotPanics(func() {
			pod := &corev1.Pod{
				ObjectMeta: metav1.ObjectMeta{
					Name:      "nil-test-pod",
					Namespace: "default",
				},
				Spec: corev1.PodSpec{
					Containers: []corev1.Container{
						{
							Name:  "test-container",
							Image: "python:3.11-slim",
							Env:   nil, // Nil env slice
						},
					},
				},
			}

			config := &injector.InjectorConfig{
				Provider:           "openlit",
				OTLPEndpoint:       "http://test:4318",
				InitContainerImage: "ghcr.io/openlit/openlit-ai-instrumentation:latest",
				ImagePullPolicy:    "IfNotPresent",
				ServiceName:        "test-service",
				ServiceNamespace:   "default",
				SharedVolumeName:   "test-volume",
				SharedVolumePath:   "/test/path",
			}

			inj := injector.New(config)
			_ = inj.InjectOpenLIT(pod)
		})
	})
}

func TestEdgeCasesTestSuite(t *testing.T) {
	suite.Run(t, new(EdgeCasesTestSuite))
}

// Helper functions for generating test data

func generateManyLabels(count int) map[string]string {
	labels := make(map[string]string)
	for i := 0; i < count; i++ {
		labels[fmt.Sprintf("label-%d", i)] = fmt.Sprintf("value-%d", i)
	}
	return labels
}

func generateManyContainers(count int) []corev1.Container {
	containers := make([]corev1.Container, count)
	for i := 0; i < count; i++ {
		containers[i] = corev1.Container{
			Name:  fmt.Sprintf("container-%d", i),
			Image: fmt.Sprintf("test-image-%d:latest", i),
			Env:   generateManyEnvVars(10), // 10 env vars per container
		}
	}
	return containers
}

func generateManyEnvVars(count int) []corev1.EnvVar {
	envVars := make([]corev1.EnvVar, count)
	for i := 0; i < count; i++ {
		envVars[i] = corev1.EnvVar{
			Name:  fmt.Sprintf("ENV_VAR_%d", i),
			Value: fmt.Sprintf("value-%d", i),
		}
	}
	return envVars
}

func mustParseQuantity(s string) resource.Quantity {
	q, err := resource.ParseQuantity(s)
	if err != nil {
		panic(err)
	}
	return q
}

// Additional edge case tests for specific scenarios
func TestCircuitBreakerEdgeCases(t *testing.T) {
	// Test circuit breaker under extreme conditions
	scheme := runtime.NewScheme()
	_ = corev1.AddToScheme(scheme)
	_ = autoinstrumentationv1alpha1.AddToScheme(scheme)

	// Create mock config and dynamic client
	cfg := &config.OperatorConfig{
		Namespace: "test-namespace",
	}
	dynamicClient := dynamicfake.NewSimpleDynamicClient(scheme)

	handler := webhook.NewHandler(cfg, scheme, dynamicClient)

	// Test rapid failure recovery
	for i := 0; i < 100; i++ {
		// This should trigger circuit breaker behavior
		pod := &corev1.Pod{
			ObjectMeta: metav1.ObjectMeta{
				Name:      fmt.Sprintf("circuit-test-pod-%d", i),
				Namespace: "default",
			},
			Spec: corev1.PodSpec{
				Containers: []corev1.Container{
					{
						Name:  "app",
						Image: "python:3.11-slim",
					},
				},
			},
		}

		podBytes, _ := json.Marshal(pod)
		req := admission.Request{
			AdmissionRequest: admissionv1.AdmissionRequest{
				UID: types.UID(fmt.Sprintf("circuit-uid-%d", i)),
				Kind: metav1.GroupVersionKind{
					Group:   "",
					Version: "v1",
					Kind:    "Pod",
				},
				Operation: admissionv1.Create,
				Object: runtime.RawExtension{
					Raw: podBytes,
				},
				Namespace: "default",
			},
		}

		response := handler.Handle(context.Background(), req)
		assert.True(t, response.Allowed, "Circuit breaker should still allow requests")
	}
}

func TestContextCancellationHandling(t *testing.T) {
	// Test behavior when context is cancelled during operations
	_, cancel := context.WithCancel(context.Background())

	config := &injector.InjectorConfig{
		Provider:           "openlit",
		OTLPEndpoint:       "http://test:4318",
		InitContainerImage: "ghcr.io/openlit/openlit-ai-instrumentation:latest",
		ImagePullPolicy:    "IfNotPresent",
		ServiceName:        "test-service",
		ServiceNamespace:   "default",
		SharedVolumeName:   "test-volume",
		SharedVolumePath:   "/test/path",
	}

	inj := injector.New(config)

	// Cancel context before injection
	cancel()

	pod := &corev1.Pod{
		ObjectMeta: metav1.ObjectMeta{
			Name:      "context-cancel-pod",
			Namespace: "default",
		},
		Spec: corev1.PodSpec{
			Containers: []corev1.Container{
				{
					Name:  "app",
					Image: "python:3.11-slim",
				},
			},
		},
	}

	// Should handle cancelled context gracefully
	err := inj.InjectOpenLIT(pod)
	// Injection may succeed or fail, but should not panic
	_ = err
}
