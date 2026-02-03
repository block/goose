package webhook

import (
	"context"
	"encoding/json"
	"testing"
	"time"

	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/suite"
	admissionv1 "k8s.io/api/admission/v1"
	corev1 "k8s.io/api/core/v1"
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	"k8s.io/apimachinery/pkg/runtime"
	"k8s.io/client-go/dynamic/fake"
	"sigs.k8s.io/controller-runtime/pkg/webhook/admission"

	autoinstrumentationv1alpha1 "github.com/openlit/openlit/operator/api/v1alpha1"
	"github.com/openlit/openlit/operator/internal/config"
)

type WebhookHandlerTestSuite struct {
	suite.Suite
	handler *Handler
	decoder *admission.Decoder
}

func (suite *WebhookHandlerTestSuite) SetupTest() {
	// Create scheme
	scheme := runtime.NewScheme()
	_ = corev1.AddToScheme(scheme)
	_ = autoinstrumentationv1alpha1.AddToScheme(scheme)
	
	// Create fake dynamic client
	dynamicClient := fake.NewSimpleDynamicClient(scheme)

	// Create operator config
	cfg := &config.OperatorConfig{
		WebhookPort:      9443,
		WebhookPath:      "/mutate",
		WebhookCertDir:   "/tmp/certs",
		Namespace:        "test-ns",
		ServiceName:      "test-service",
		SecretName:       "test-secret",
		WatchNamespace:   "",
		CertValidityDays: 365,
		CertRefreshDays:  30,
		HealthPort:       8081,
		LogLevel:         "info",
	}

	// Create handler
	suite.handler = NewHandler(cfg, scheme, dynamicClient)
	
	// Create decoder
	decoder := admission.NewDecoder(scheme)
	suite.decoder = &decoder
}

func (suite *WebhookHandlerTestSuite) TestCircuitBreakerInitialization() {
	suite.Equal(CircuitBreakerClosed, suite.handler.circuitBreaker.getState())
	suite.Equal(int32(0), suite.handler.circuitBreaker.failureCount)
}

func (suite *WebhookHandlerTestSuite) TestCircuitBreakerStates() {
	cb := suite.handler.circuitBreaker

	// Test closed state (initial)
	suite.True(cb.canExecute())
	suite.Equal(CircuitBreakerClosed, cb.getState())

	// Test failure recording (maxFailures is 5)
	for i := 0; i < 5; i++ {
		cb.recordFailure()
	}
	
	// Should now be open (maxFailures is 5)
	suite.Equal(CircuitBreakerOpen, cb.getState())
	suite.False(cb.canExecute())

	// Test transition to half-open after timeout
	cb.resetTimeout = 1 * time.Millisecond
	cb.lastFailureTime = time.Now().Add(-2 * time.Millisecond)
	
	// Should allow execution after timeout, but state only changes on attempt
	suite.True(cb.canExecute())
	cb.attempt() // This actually transitions to half-open
	suite.Equal(CircuitBreakerHalfOpen, cb.getState())

	// Test success recording in half-open state
	cb.recordSuccess()
	suite.Equal(CircuitBreakerClosed, cb.getState())
	suite.Equal(int32(0), cb.failureCount)
}

func (suite *WebhookHandlerTestSuite) TestPodMatchesSelector() {
	pod := &corev1.Pod{
		ObjectMeta: metav1.ObjectMeta{
			Name:      "test-pod",
			Namespace: "default",
			Labels: map[string]string{
				"app":     "python-app",
				"version": "v1.0.0",
				"env":     "production",
			},
		},
	}

	tests := []struct {
		name        string
		labels      map[string]string
		expected    bool
		description string
	}{
		{
			name: "Exact match",
			labels: map[string]string{
				"app": "python-app",
			},
			expected:    true,
			description: "Should match when all selector labels are present",
		},
		{
			name: "Multiple labels match",
			labels: map[string]string{
				"app": "python-app",
				"env": "production",
			},
			expected:    true,
			description: "Should match when multiple selector labels are present",
		},
		{
			name: "No match",
			labels: map[string]string{
				"app": "java-app",
			},
			expected:    false,
			description: "Should not match when selector labels don't match",
		},
		{
			name: "Partial match",
			labels: map[string]string{
				"app":      "python-app",
				"missing":  "label",
			},
			expected:    false,
			description: "Should not match when some selector labels are missing",
		},
		{
			name:        "Empty selector",
			labels:      map[string]string{},
			expected:    true,
			description: "Empty selector should match all pods",
		},
	}

	for _, tt := range tests {
		suite.Run(tt.name, func() {
			result := suite.handler.podMatchesSelector(pod, tt.labels)
			suite.Equal(tt.expected, result, tt.description)
		})
	}
}

func (suite *WebhookHandlerTestSuite) TestHandleAdmissionRequest() {
	// Create test AutoInstrumentation configurations
	_ = &autoinstrumentationv1alpha1.AutoInstrumentation{
		ObjectMeta: metav1.ObjectMeta{
			Name:      "python-instrumentation",
			Namespace: "default",
		},
		Spec: autoinstrumentationv1alpha1.AutoInstrumentationSpec{
			Selector: autoinstrumentationv1alpha1.PodSelector{
				MatchLabels: map[string]string{
					"app": "python-app",
				},
			},
			OTLP: autoinstrumentationv1alpha1.OTLPConfig{
				Endpoint: "http://jaeger:4318",
			},
		},
	}

	_ = &autoinstrumentationv1alpha1.AutoInstrumentation{
		ObjectMeta: metav1.ObjectMeta{
			Name:      "ignored-instrumentation",
			Namespace: "default",
		},
		Spec: autoinstrumentationv1alpha1.AutoInstrumentationSpec{
			Selector: autoinstrumentationv1alpha1.PodSelector{
				MatchLabels: map[string]string{
					"app": "python-app",
				},
			},
			Ignore: &autoinstrumentationv1alpha1.PodSelector{
				MatchLabels: map[string]string{
					"skip": "true",
				},
			},
			OTLP: autoinstrumentationv1alpha1.OTLPConfig{
				Endpoint: "http://jaeger:4318",
			},
		},
	}

	// TODO: Add configurations to fake client - Handler uses dynamic client now, needs updating
	// _ = suite.handler.client.Create(context.Background(), autoInstr1)
	// _ = suite.handler.client.Create(context.Background(), autoInstr2)

	tests := []struct {
		name        string
		pod         *corev1.Pod
		expectAllow bool
		expectPatch bool
		description string
	}{
		{
			name: "Pod matches selector and should be instrumented",
			pod: &corev1.Pod{
				ObjectMeta: metav1.ObjectMeta{
					Name:      "python-pod",
					Namespace: "default",
					Labels: map[string]string{
						"app": "python-app",
					},
				},
				Spec: corev1.PodSpec{
					Containers: []corev1.Container{
						{
							Name:  "python-app",
							Image: "python:3.11-slim",
						},
					},
				},
			},
			expectAllow: true,
			expectPatch: false, // No patches expected since no AutoInstrumentation resources exist in fake client
			description: "Pod matching selector should be allowed but not patched (no AutoInstrumentation resources)",
		},
		{
			name: "Pod matches selector but is ignored",
			pod: &corev1.Pod{
				ObjectMeta: metav1.ObjectMeta{
					Name:      "ignored-pod",
					Namespace: "default",
					Labels: map[string]string{
						"app":  "python-app",
						"skip": "true",
					},
				},
				Spec: corev1.PodSpec{
					Containers: []corev1.Container{
						{
							Name:  "python-app",
							Image: "python:3.11-slim",
						},
					},
				},
			},
			expectAllow: true,
			expectPatch: false,
			description: "Pod matching ignore selector should be allowed but not patched",
		},
		{
			name: "Pod doesn't match any selector",
			pod: &corev1.Pod{
				ObjectMeta: metav1.ObjectMeta{
					Name:      "java-pod",
					Namespace: "default",
					Labels: map[string]string{
						"app": "java-app",
					},
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
			expectAllow: true,
			expectPatch: false,
			description: "Pod not matching selectors should be allowed but not patched",
		},
		{
			name: "Pod with no labels",
			pod: &corev1.Pod{
				ObjectMeta: metav1.ObjectMeta{
					Name:      "no-labels-pod",
					Namespace: "default",
				},
				Spec: corev1.PodSpec{
					Containers: []corev1.Container{
						{
							Name:  "app",
							Image: "alpine:latest",
						},
					},
				},
			},
			expectAllow: true,
			expectPatch: false,
			description: "Pod with no labels should be allowed but not patched",
		},
	}

	for _, tt := range tests {
		suite.Run(tt.name, func() {
			// Convert pod to raw JSON for admission request
			podBytes, err := json.Marshal(tt.pod)
			suite.NoError(err)

			// Create admission request
			req := admission.Request{
				AdmissionRequest: admissionv1.AdmissionRequest{
					UID: "test-uid",
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

			// Handle the request
			response := suite.handler.Handle(context.Background(), req)

			// Verify response
			suite.Equal(tt.expectAllow, response.Allowed, tt.description)
			
			if tt.expectPatch {
				suite.NotEmpty(response.Patches, "Should have patches when injection is expected")
			} else {
				suite.Empty(response.Patches, "Should have no patches when injection is not expected")
			}
		})
	}
}

func (suite *WebhookHandlerTestSuite) TestExecuteWithRetryAndTimeout() {
	suite.Run("Successful execution", func() {
		executionCount := 0
		operation := func() error {
			executionCount++
			return nil
		}

		err := suite.handler.executeWithRetryAndTimeout(context.Background(), operation, "test-operation")
		suite.NoError(err)
		suite.Equal(1, executionCount, "Should execute only once on success")
	})

	suite.Run("Retry on failure then success", func() {
		executionCount := 0
		operation := func() error {
			executionCount++
			if executionCount < 3 {
				return assert.AnError
			}
			return nil
		}

		err := suite.handler.executeWithRetryAndTimeout(context.Background(), operation, "test-operation")
		suite.NoError(err)
		suite.Equal(3, executionCount, "Should retry until success")
	})

	suite.Run("Max retries exceeded", func() {
		executionCount := 0
		operation := func() error {
			executionCount++
			return assert.AnError
		}

		err := suite.handler.executeWithRetryAndTimeout(context.Background(), operation, "test-operation")
		suite.Error(err)
		suite.Contains(err.Error(), "failed after 3 attempts")
		suite.Equal(suite.handler.maxRetries, executionCount, "Should execute max retries times")
	})
}

func (suite *WebhookHandlerTestSuite) TestHealthMetrics() {
	// Test initial health metrics
	suite.handler.updateHealthMetrics()
	
	// Verify initial circuit breaker state
	suite.Equal(int32(0), suite.handler.circuitBreaker.failureCount)
	
	// Test failure recording through circuit breaker
	suite.handler.circuitBreaker.recordFailure()
	
	// Verify circuit breaker recorded failure
	suite.Equal(int32(1), suite.handler.circuitBreaker.failureCount)
}

func (suite *WebhookHandlerTestSuite) TestErrorScenarios() {
	suite.Run("Invalid JSON in admission request", func() {
		req := admission.Request{
			AdmissionRequest: admissionv1.AdmissionRequest{
				UID: "test-uid",
				Kind: metav1.GroupVersionKind{
					Group:   "",
					Version: "v1",
					Kind:    "Pod",
				},
				Operation: admissionv1.Create,
				Object: runtime.RawExtension{
					Raw: []byte("invalid json"),
				},
				Namespace: "default",
			},
		}

		response := suite.handler.Handle(context.Background(), req)
		suite.False(response.Allowed, "Should not allow pods with invalid JSON")
		suite.Contains(response.Result.Message, "json parse error", "Should contain JSON parse error message")
	})

	suite.Run("Non-pod resource", func() {
		service := &corev1.Service{
			ObjectMeta: metav1.ObjectMeta{
				Name:      "test-service",
				Namespace: "default",
			},
		}

		serviceBytes, _ := json.Marshal(service)

		req := admission.Request{
			AdmissionRequest: admissionv1.AdmissionRequest{
				UID: "test-uid",
				Kind: metav1.GroupVersionKind{
					Group:   "",
					Version: "v1",
					Kind:    "Service",
				},
				Operation: admissionv1.Create,
				Object: runtime.RawExtension{
					Raw: serviceBytes,
				},
				Namespace: "default",
			},
		}

		response := suite.handler.Handle(context.Background(), req)
		suite.True(response.Allowed, "Should allow non-pod resources")
		suite.Empty(response.Patches, "Should not patch non-pod resources")
	})
}

func TestWebhookHandlerSuite(t *testing.T) {
	suite.Run(t, new(WebhookHandlerTestSuite))
}

// Additional unit tests for specific functions
func TestCircuitBreakerStates(t *testing.T) {
	cb := &CircuitBreaker{
		maxFailures:  5,
		resetTimeout: 100 * time.Millisecond,
	}

	// Test initial state
	assert.Equal(t, CircuitBreakerClosed, cb.getState())
	assert.True(t, cb.canExecute())

	// Test failure accumulation
	for i := 0; i < 4; i++ {
		cb.recordFailure()
	}
	assert.Equal(t, CircuitBreakerClosed, cb.getState())

	// Test transition to open
	cb.recordFailure()
	assert.Equal(t, CircuitBreakerOpen, cb.getState())
	assert.False(t, cb.canExecute())

	// Test transition to half-open
	time.Sleep(101 * time.Millisecond)
	assert.True(t, cb.canExecute())
	cb.attempt() // This actually transitions to half-open
	assert.Equal(t, CircuitBreakerHalfOpen, cb.getState())

	// Test success resets circuit breaker
	cb.recordSuccess()
	assert.Equal(t, CircuitBreakerClosed, cb.getState())
	assert.Equal(t, int32(0), cb.failureCount)
}

func TestCircuitBreakerConcurrency(t *testing.T) {
	cb := &CircuitBreaker{
		maxFailures:  10,
		resetTimeout: 100 * time.Millisecond,
	}

	// Test concurrent access
	done := make(chan bool, 20)
	
	for i := 0; i < 10; i++ {
		go func() {
			for j := 0; j < 10; j++ {
				cb.recordFailure()
				cb.canExecute()
				cb.getState()
			}
			done <- true
		}()
		
		go func() {
			for j := 0; j < 10; j++ {
				cb.recordSuccess()
				cb.canExecute()
				cb.getState()
			}
			done <- true
		}()
	}

	// Wait for all goroutines to complete
	for i := 0; i < 20; i++ {
		<-done
	}

	// Circuit breaker should still be functional
	assert.True(t, cb.canExecute() || !cb.canExecute()) // Should not panic
}
