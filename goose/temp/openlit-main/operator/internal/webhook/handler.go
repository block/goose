/*
OpenLIT Admission Webhook Handler

This is the core admission webhook that implements zero-code instrumentation
by intercepting pod creation requests and automatically injecting OpenTelemetry
instrumentation into matching pods based on AutoInstrumentation Custom Resources.

Key responsibilities:
- Intercepts pod creation through Kubernetes admission controller
- Matches pods against AutoInstrumentation selector criteria
- Injects init containers with instrumentation packages
- Configures environment variables for OpenTelemetry
- Handles multiple instrumentation providers (OpenLIT, OpenInference, OpenLLMetry)
- Provides comprehensive error handling and observability

The webhook works by:
1. Receiving admission review requests for pod creation
2. Finding matching AutoInstrumentation CRs for the pod's namespace and labels
3. Creating injector configuration from the CR settings
4. Modifying the pod spec to include instrumentation init containers
5. Returning the modified pod specification to Kubernetes

Supports advanced features:
- Custom package installation
- Provider-specific configurations
- Environment variable injection from secrets/configmaps
- Namespace-scoped and cluster-wide instrumentation
- Skip/ignore patterns for fine-grained control

This is where the "magic" of zero-code instrumentation happens!
*/
package webhook

import (
	"context"
	"encoding/json"
	"fmt"
	"io/ioutil"
	"net/http"
	"os"
	"sort"
	"strings"
	"sync"
	"sync/atomic"
	"time"

	corev1 "k8s.io/api/core/v1"
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	"k8s.io/apimachinery/pkg/runtime"
	"k8s.io/apimachinery/pkg/runtime/schema"
	"k8s.io/apimachinery/pkg/types"
	"k8s.io/client-go/dynamic"
	"sigs.k8s.io/controller-runtime/pkg/webhook/admission"

	"github.com/openlit/openlit/operator/api/v1alpha1"
	"github.com/openlit/openlit/operator/internal/config"
	"github.com/openlit/openlit/operator/internal/injector"
	"github.com/openlit/openlit/operator/internal/observability"
)

var log = observability.NewLogger("webhook")

// Use central schema - no local type aliases needed

// Functions removed - using central v1alpha1.AutoInstrumentation type with generated methods

// All type definitions removed - using central v1alpha1 package types

// CircuitBreakerState represents the state of the circuit breaker
type CircuitBreakerState int32

const (
	CircuitBreakerClosed CircuitBreakerState = iota
	CircuitBreakerOpen
	CircuitBreakerHalfOpen
)

// CircuitBreaker implements circuit breaker pattern for webhook reliability
type CircuitBreaker struct {
	maxFailures     int32
	resetTimeout    time.Duration
	failureCount    int32
	state           CircuitBreakerState
	lastFailureTime time.Time
	mutex           sync.RWMutex
}

// Handler implements the admission.Handler interface
type Handler struct {
	decoder         admission.Decoder
	dynamicClient   dynamic.Interface
	config          *config.OperatorConfig
	circuitBreaker  *CircuitBreaker
	requestTimeout  time.Duration
	maxRetries      int
	retryDelay      time.Duration
	requestCount    int64
	failureCount    int64
	lastHealthCheck time.Time
}

// NewHandler creates a new webhook handler
func NewHandler(cfg *config.OperatorConfig, scheme *runtime.Scheme, dynamicClient dynamic.Interface) *Handler {
	decoder := admission.NewDecoder(scheme)
	return &Handler{
		config:        cfg,
		dynamicClient: dynamicClient,
		decoder:       decoder,
		circuitBreaker: &CircuitBreaker{
			maxFailures:  5,                // Open circuit after 5 consecutive failures
			resetTimeout: 30 * time.Second, // Try to close circuit after 30 seconds
			state:        CircuitBreakerClosed,
		},
		requestTimeout:  10 * time.Second,       // 10 second timeout for webhook operations
		maxRetries:      3,                      // Retry up to 3 times
		retryDelay:      100 * time.Millisecond, // 100ms delay between retries
		lastHealthCheck: time.Now(),
	}
}

// Handle handles the admission requests for pod injection
func (h *Handler) Handle(ctx context.Context, req admission.Request) admission.Response {
	log.Info("Webhook received admission request",
		"component", "webhook-handler",
		"k8s.admission.request.kind", req.Kind.Kind,
		"k8s.admission.request.namespace", req.Namespace,
		"k8s.admission.request.operation", string(req.Operation))

	pod := &corev1.Pod{}

	err := h.decoder.Decode(req, pod)
	if err != nil {
		log.Error("Failed to decode admission request", err,
			"component", "webhook-handler",
			"k8s.admission.request.uid", string(req.UID))
		return admission.Errored(http.StatusBadRequest, err)
	}

	log.Info("Checking pod for instrumentation",
		"component", "webhook-handler",
		"k8s.namespace.name", pod.Namespace)

	// Find matching AutoInstrumentation CRs
	instrConfig, err := h.findMatchingAutoInstrumentation(ctx, pod)
	if err != nil {
		log.Error("Failed to find matching AutoInstrumentation", err,
			"component", "webhook-handler",
			"k8s.pod.name", pod.Name,
			"k8s.namespace.name", pod.Namespace)
		return admission.Errored(http.StatusInternalServerError, err)
	}

	if instrConfig == nil {
		log.Info("⏭️ No matching AutoInstrumentation found",
			"component", "webhook-handler",
			"k8s.namespace.name", pod.Namespace)
		return admission.Allowed("no matching instrumentation config")
	}

	log.Info("🚀 Found matching AutoInstrumentation, injecting instrumentation",
		"component", "webhook-handler",
		"k8s.pod.name", pod.Name,
		"k8s.namespace.name", pod.Namespace,
		"instrumentation.config.name", instrConfig.Name,
		"instrumentation.config.namespace", instrConfig.Namespace,
		"instrumentation.provider", instrConfig.Spec.Python.Instrumentation.Provider)

	// Create injector with the CR configuration
	injectorConfig := h.createInjectorConfig(instrConfig)
	injector := injector.New(injectorConfig)

	// Inject instrumentation
	injector.InjectOpenLIT(pod)

	// Create JSON patch from the differences
	podBytes, err := json.Marshal(pod)
	if err != nil {
		return admission.Errored(http.StatusInternalServerError, err)
	}

	log.Info("Instrumentation injected successfully",
		"component", "webhook-handler",
		"k8s.pod.name", pod.Name,
		"k8s.namespace.name", pod.Namespace,
		"instrumentation.provider", instrConfig.Spec.Python.Instrumentation.Provider,
		"instrumentation.config.name", instrConfig.Name)
	return admission.PatchResponseFromRaw(req.Object.Raw, podBytes)
}

// findMatchingAutoInstrumentation finds the best matching AutoInstrumentation with conflict detection
func (h *Handler) findMatchingAutoInstrumentation(ctx context.Context, pod *corev1.Pod) (*v1alpha1.AutoInstrumentation, error) {
	// Define the AutoInstrumentation GVR
	gvr := schema.GroupVersionResource{
		Group:    "openlit.io",
		Version:  "v1alpha1",
		Resource: "autoinstrumentations",
	}

	// Search order: App-specific (same namespace) → Global (openlit)
	namespaces := []string{pod.Namespace, "openlit"}

	for _, namespace := range namespaces {
		log.Info("Searching for AutoInstrumentations",
			"component", "webhook-handler",
			"k8s.namespace.name", namespace)

		// List AutoInstrumentations in this namespace
		result, err := h.dynamicClient.Resource(gvr).Namespace(namespace).List(ctx, metav1.ListOptions{})
		if err != nil {
			log.Error("Failed to list AutoInstrumentations", err,
				"component", "webhook-handler",
				"k8s.namespace.name", namespace)
			continue
		}

		log.Info("Found AutoInstrumentations",
			"component", "webhook-handler",
			"k8s.namespace.name", namespace,
			"instrumentation.config.count", len(result.Items))

		// Collect all matching configs in this namespace
		var matchingConfigs []*v1alpha1.AutoInstrumentation

		for _, item := range result.Items {
			cfg, err := h.parseAutoInstrumentation(item.Object)
			if err != nil {
				log.Error("Failed to parse AutoInstrumentation", err,
					"component", "webhook-handler",
					"instrumentation.config.name", item.GetName(),
					"k8s.namespace.name", namespace)
				continue
			}

			if cfg.Spec.Selector.MatchLabels != nil {
				log.Info("Checking selector",
					"component", "webhook-handler",
					"instrumentation.config.name", cfg.Name,
					"instrumentation.config.namespace", cfg.Namespace)

				if h.podMatchesSelector(pod, cfg.Spec.Selector.MatchLabels) {
					// Check if pod should be ignored
					if cfg.Spec.Ignore != nil && cfg.Spec.Ignore.MatchLabels != nil {
						if h.podMatchesSelector(pod, cfg.Spec.Ignore.MatchLabels) {
							log.Info("Pod matches selector but is explicitly ignored",
								"component", "webhook-handler",
								"k8s.pod.name", pod.Name,
								"k8s.namespace.name", pod.Namespace,
								"instrumentation.config.name", cfg.Name,
								"instrumentation.config.namespace", cfg.Namespace,
								"selector.matched", true,
								"ignore.matched", true)
							continue // Skip this config
						}
					}

					log.Info("Found matching config",
						"component", "webhook-handler",
						"instrumentation.config.name", cfg.Name,
						"instrumentation.config.namespace", cfg.Namespace,
						"instrumentation.provider", cfg.Spec.Python.Instrumentation.Provider)

					// Note: Management labels are handled proactively by the AutoInstrumentation controller

					matchingConfigs = append(matchingConfigs, cfg)
				}
			}
		}

		// Handle conflicts and select the oldest match (first created)
		if len(matchingConfigs) > 1 {
			// Sort by creation timestamp for deterministic first-created-wins behavior
			sort.Slice(matchingConfigs, func(i, j int) bool {
				return matchingConfigs[i].CreationTimestamp.Before(&matchingConfigs[j].CreationTimestamp)
			})

			// Log warning about conflicts
			var conflictNames []string
			for _, cfg := range matchingConfigs {
				conflictNames = append(conflictNames, fmt.Sprintf("%s/%s(%s,created:%s)",
					cfg.Namespace, cfg.Name, cfg.Spec.Python.Instrumentation.Provider,
					cfg.CreationTimestamp.Format("15:04:05")))
			}

			log.Info("Multiple AutoInstrumentations match this pod - using oldest one (first created)",
				"component", "webhook-handler",
				"k8s.pod.name", pod.Name,
				"k8s.pod.namespace", pod.Namespace,
				"instrumentation.config.selected", fmt.Sprintf("%s/%s(%s,created:%s)",
					matchingConfigs[0].Namespace, matchingConfigs[0].Name,
					matchingConfigs[0].Spec.Python.Instrumentation.Provider,
					matchingConfigs[0].CreationTimestamp.Format("15:04:05")),
				"instrumentation.config.conflicts", conflictNames)
		}

		// Return the first matching config if any found
		if len(matchingConfigs) > 0 {
			return matchingConfigs[0], nil
		}
	}

	return nil, nil
}

// parseAutoInstrumentation converts unstructured data to AutoInstrumentation
func (h *Handler) parseAutoInstrumentation(obj map[string]interface{}) (*v1alpha1.AutoInstrumentation, error) {
	// Extract metadata
	metadata := obj["metadata"].(map[string]interface{})
	name := metadata["name"].(string)
	namespace := metadata["namespace"].(string)

	// Parse creation timestamp
	var creationTimestamp metav1.Time
	if creationTimeStr, ok := metadata["creationTimestamp"].(string); ok {
		if parsedTime, err := time.Parse(time.RFC3339, creationTimeStr); err == nil {
			creationTimestamp = metav1.NewTime(parsedTime)
		}
	}

	// Extract spec
	spec := obj["spec"].(map[string]interface{})

	cfg := &v1alpha1.AutoInstrumentation{
		ObjectMeta: metav1.ObjectMeta{
			Name:              name,
			Namespace:         namespace,
			CreationTimestamp: creationTimestamp,
		},
		Spec: v1alpha1.AutoInstrumentationSpec{},
	}

	// Parse selector
	if selectorData, ok := spec["selector"].(map[string]interface{}); ok {
		if matchLabels, ok := selectorData["matchLabels"].(map[string]interface{}); ok {
			cfg.Spec.Selector.MatchLabels = make(map[string]string)
			for k, v := range matchLabels {
				cfg.Spec.Selector.MatchLabels[k] = v.(string)
			}
		}
	}

	// Parse python config
	if pythonData, ok := spec["python"].(map[string]interface{}); ok {
		cfg.Spec.Python = &v1alpha1.PythonInstrumentation{
			Instrumentation: &v1alpha1.InstrumentationSettings{},
		}
		if instrumentationData, ok := pythonData["instrumentation"].(map[string]interface{}); ok {
			if enabled, ok := instrumentationData["enabled"].(bool); ok {
				cfg.Spec.Python.Instrumentation.Enabled = &enabled
			}
			if provider, ok := instrumentationData["provider"].(string); ok {
				cfg.Spec.Python.Instrumentation.Provider = provider
			}
			if version, ok := instrumentationData["version"].(string); ok {
				cfg.Spec.Python.Instrumentation.Version = version
			}
			if imagePullPolicy, ok := instrumentationData["imagePullPolicy"].(string); ok {
				cfg.Spec.Python.Instrumentation.ImagePullPolicy = imagePullPolicy
			}
			if customPackages, ok := instrumentationData["customPackages"].(string); ok {
				cfg.Spec.Python.Instrumentation.CustomPackages = customPackages
			}
			if customInitImage, ok := instrumentationData["customInitImage"].(string); ok {
				cfg.Spec.Python.Instrumentation.CustomInitImage = customInitImage
			}
		}
	}

	// Parse OTLP config
	if otlpData, ok := spec["otlp"].(map[string]interface{}); ok {
		if endpoint, ok := otlpData["endpoint"].(string); ok {
			cfg.Spec.OTLP.Endpoint = endpoint
		}
		if headers, ok := otlpData["headers"].(string); ok {
			cfg.Spec.OTLP.Headers = headers
		}
		if timeout, ok := otlpData["timeout"].(float64); ok {
			timeoutInt32 := int32(timeout)
			cfg.Spec.OTLP.Timeout = &timeoutInt32
		}
	}

	// Parse resource config
	if resourceData, ok := spec["resource"].(map[string]interface{}); ok {
		cfg.Spec.Resource = &v1alpha1.ResourceConfig{}
		if environment, ok := resourceData["environment"].(string); ok {
			cfg.Spec.Resource.Environment = environment
		}
	}

	return cfg, nil
}

// podMatchesSelector checks if pod labels match the selector
func (h *Handler) podMatchesSelector(pod *corev1.Pod, selector map[string]string) bool {
	podLabels := pod.GetLabels()
	if podLabels == nil {
		return false
	}

	for key, value := range selector {
		if podLabels[key] != value {
			return false
		}
	}
	return true
}

// createInjectorConfig creates injector config from AutoInstrumentation CR
func (h *Handler) createInjectorConfig(instrConfig *v1alpha1.AutoInstrumentation) *injector.InjectorConfig {
	// Convert ImagePullPolicy string to corev1.PullPolicy
	var pullPolicy corev1.PullPolicy
	switch instrConfig.Spec.Python.Instrumentation.ImagePullPolicy {
	case "Always":
		pullPolicy = corev1.PullAlways
	case "Never":
		pullPolicy = corev1.PullNever
	case "IfNotPresent":
		fallthrough
	default:
		pullPolicy = corev1.PullIfNotPresent
	}

	cfg := &injector.InjectorConfig{
		// From AutoInstrumentation CR
		Provider:        instrConfig.Spec.Python.Instrumentation.Provider,
		OTLPEndpoint:    instrConfig.Spec.OTLP.Endpoint,
		OTLPHeaders:     instrConfig.Spec.OTLP.Headers,
		ImagePullPolicy: pullPolicy,

		// Resource configuration
		Environment:      "",
		ServiceName:      "instrumented-app",
		ServiceNamespace: instrConfig.Namespace,

		// Instrumentation settings (set defaults)
		CaptureMessageContent: true,
		DetailedTracing:       true,

		// Volume configuration (using configurable names instead of hardcoded)
		SharedVolumeName: "instrumentation-packages",
		SharedVolumePath: "/instrumentation-packages",

		// Custom packages/image
		CustomPackages:  "",
		CustomInitImage: "",
	}

	// Set environment from resource config
	if instrConfig.Spec.Resource != nil {
		cfg.Environment = instrConfig.Spec.Resource.Environment
	}

	// Set custom packages and image if specified
	if instrConfig.Spec.Python != nil && instrConfig.Spec.Python.Instrumentation != nil {
		if instrConfig.Spec.Python.Instrumentation.CustomPackages != "" {
			cfg.CustomPackages = instrConfig.Spec.Python.Instrumentation.CustomPackages
		}

		// Set custom init image if specified
		if instrConfig.Spec.Python.Instrumentation.CustomInitImage != "" {
			cfg.CustomInitImage = instrConfig.Spec.Python.Instrumentation.CustomInitImage
		} else {
			// Use provider-specific image
			cfg.InitContainerImage = h.getProviderImage(instrConfig.Spec.Python.Instrumentation.Provider)
		}
	}

	// Convert CR env vars to corev1.EnvVar
	if instrConfig.Spec.Python != nil && instrConfig.Spec.Python.Instrumentation != nil {
		cfg.EnvVars = make([]corev1.EnvVar, len(instrConfig.Spec.Python.Instrumentation.Env))
		for i, envVar := range instrConfig.Spec.Python.Instrumentation.Env {
			cfg.EnvVars[i] = corev1.EnvVar{
				Name:  envVar.Name,
				Value: envVar.Value,
			}

			// Handle valueFrom if specified
			if envVar.ValueFrom != nil {
				cfg.EnvVars[i].ValueFrom = &corev1.EnvVarSource{}

				if envVar.ValueFrom.SecretKeyRef != nil {
					cfg.EnvVars[i].ValueFrom.SecretKeyRef = &corev1.SecretKeySelector{
						LocalObjectReference: corev1.LocalObjectReference{
							Name: envVar.ValueFrom.SecretKeyRef.Name,
						},
						Key:      envVar.ValueFrom.SecretKeyRef.Key,
						Optional: envVar.ValueFrom.SecretKeyRef.Optional,
					}
				}

				if envVar.ValueFrom.ConfigMapKeyRef != nil {
					cfg.EnvVars[i].ValueFrom.ConfigMapKeyRef = &corev1.ConfigMapKeySelector{
						LocalObjectReference: corev1.LocalObjectReference{
							Name: envVar.ValueFrom.ConfigMapKeyRef.Name,
						},
						Key:      envVar.ValueFrom.ConfigMapKeyRef.Key,
						Optional: envVar.ValueFrom.ConfigMapKeyRef.Optional,
					}
				}

				if envVar.ValueFrom.FieldRef != nil {
					cfg.EnvVars[i].ValueFrom.FieldRef = &corev1.ObjectFieldSelector{
						APIVersion: envVar.ValueFrom.FieldRef.APIVersion,
						FieldPath:  envVar.ValueFrom.FieldRef.FieldPath,
					}
				}
			}
		}
	}

	return cfg
}

// getProviderImage returns the appropriate image for the provider
func (h *Handler) getProviderImage(provider string) string {
	switch provider {
	case "openlit":
		return "ghcr.io/openlit/openlit-ai-instrumentation:latest"
	case "openinference":
		return "ghcr.io/openlit/openinference-ai-instrumentation:latest"
	case "openllmetry":
		return "ghcr.io/openlit/openllmetry-ai-instrumentation:latest"
	case "custom", "base":
		return "ghcr.io/openlit/openlit-ai-instrumentation:latest"
	default:
		return "ghcr.io/openlit/openlit-ai-instrumentation:latest"
	}
}

// addManagementLabel adds a management label to the AutoInstrumentation CR to indicate
// it is being managed/used by this OpenLIT operator instance
func (h *Handler) addManagementLabel(ctx context.Context, cfg *v1alpha1.AutoInstrumentation, namespace string, gvr schema.GroupVersionResource) error {
	// Check if the label already exists
	if cfg.Labels != nil {
		if _, exists := cfg.Labels["openlit.io/managed-by"]; exists {
			// Label already exists, no need to update
			return nil
		}
	}

	// Get the operator instance details for the label value
	operatorNamespace := h.config.Namespace
	if operatorNamespace == "" {
		// Try to get from service account, fallback to schema default
		operatorNamespace = getNamespaceFromServiceAccount()
	}
	// Try to get actual pod name from hostname, fallback to static name
	operatorName := getOperatorPodName()

	// Create the management label value
	managementLabel := fmt.Sprintf("%s.%s", operatorName, operatorNamespace)

	log.Info("Adding management label to AutoInstrumentation",
		"component", "webhook-handler",
		"instrumentation.config.name", cfg.Name,
		"instrumentation.config.namespace", cfg.Namespace,
		"management.label", managementLabel)

	// Prepare the patch to add the label
	patch := map[string]interface{}{
		"metadata": map[string]interface{}{
			"labels": map[string]interface{}{
				"openlit.io/managed-by":      managementLabel,
				"openlit.io/instrumented-by": "openlit-operator",
			},
		},
	}

	// Convert patch to JSON
	patchBytes, err := json.Marshal(patch)
	if err != nil {
		return fmt.Errorf("failed to marshal patch: %w", err)
	}

	// Apply the patch
	_, err = h.dynamicClient.Resource(gvr).Namespace(cfg.Namespace).Patch(
		ctx,
		cfg.Name,
		types.MergePatchType,
		patchBytes,
		metav1.PatchOptions{},
	)
	if err != nil {
		return fmt.Errorf("failed to patch AutoInstrumentation with management labels: %w", err)
	}

	log.Info("Successfully added management labels to AutoInstrumentation",
		"component", "webhook-handler",
		"instrumentation.config.name", cfg.Name,
		"instrumentation.config.namespace", cfg.Namespace,
		"openlit.io/managed-by", managementLabel,
		"openlit.io/instrumented-by", "openlit-operator")

	return nil
}

// InjectDecoder injects the decoder
func (h *Handler) InjectDecoder(d admission.Decoder) error {
	h.decoder = d
	return nil
}

// getOperatorPodName gets the actual pod name from hostname or falls back to static name
func getOperatorPodName() string {
	// Try to get actual pod name from hostname (Kubernetes sets this to pod name)
	if hostname, err := os.Hostname(); err == nil && hostname != "" {
		return hostname
	}
	// Fallback to operator name
	return "openlit-operator"
}

// getNamespaceFromServiceAccount gets the current namespace from the service account token
func getNamespaceFromServiceAccount() string {
	// Read namespace from service account token (standard Kubernetes mount)
	if data, err := ioutil.ReadFile("/var/run/secrets/kubernetes.io/serviceaccount/namespace"); err == nil {
		namespace := strings.TrimSpace(string(data))
		if namespace != "" {
			return namespace
		}
	}
	// Fallback to default
	return "openlit"
}

// Circuit Breaker Methods

// canExecute checks if the circuit breaker allows execution
func (cb *CircuitBreaker) canExecute() bool {
	cb.mutex.RLock()
	defer cb.mutex.RUnlock()

	switch cb.state {
	case CircuitBreakerClosed:
		return true
	case CircuitBreakerOpen:
		// Check if it's time to attempt recovery
		return time.Since(cb.lastFailureTime) >= cb.resetTimeout
	case CircuitBreakerHalfOpen:
		return true
	default:
		return false
	}
}

// recordSuccess records a successful operation
func (cb *CircuitBreaker) recordSuccess() {
	cb.mutex.Lock()
	defer cb.mutex.Unlock()

	cb.failureCount = 0
	if cb.state == CircuitBreakerHalfOpen {
		cb.state = CircuitBreakerClosed
		log.Info("Circuit breaker closed - operations are healthy",
			"component", "circuit-breaker",
			"circuit.state", "closed")
	}
}

// recordFailure records a failed operation
func (cb *CircuitBreaker) recordFailure() {
	cb.mutex.Lock()
	defer cb.mutex.Unlock()

	cb.failureCount++
	cb.lastFailureTime = time.Now()

	switch cb.state {
	case CircuitBreakerClosed:
		if cb.failureCount >= cb.maxFailures {
			cb.state = CircuitBreakerOpen
			log.Warn("Circuit breaker opened - too many failures detected",
				"component", "circuit-breaker",
				"circuit.state", "open",
				"failure.count", cb.failureCount,
				"failure.threshold", cb.maxFailures)
		}
	case CircuitBreakerHalfOpen:
		cb.state = CircuitBreakerOpen
		log.Warn("Circuit breaker reopened - failure during recovery",
			"component", "circuit-breaker",
			"circuit.state", "open")
	}
}

// attempt executes an operation with circuit breaker protection
func (cb *CircuitBreaker) attempt() bool {
	cb.mutex.Lock()
	defer cb.mutex.Unlock()

	if cb.state == CircuitBreakerOpen && time.Since(cb.lastFailureTime) >= cb.resetTimeout {
		cb.state = CircuitBreakerHalfOpen
		log.Info("Circuit breaker half-open - attempting recovery",
			"component", "circuit-breaker",
			"circuit.state", "half-open")
		return true
	}

	return cb.state != CircuitBreakerOpen
}

// getState returns the current circuit breaker state
func (cb *CircuitBreaker) getState() CircuitBreakerState {
	cb.mutex.RLock()
	defer cb.mutex.RUnlock()
	return cb.state
}

// Reliability Methods

// executeWithRetryAndTimeout executes an operation with retry and timeout logic
func (h *Handler) executeWithRetryAndTimeout(ctx context.Context, operation func() error, operationName string) error {
	atomic.AddInt64(&h.requestCount, 1)

	// Check circuit breaker first
	if !h.circuitBreaker.canExecute() {
		atomic.AddInt64(&h.failureCount, 1)
		return fmt.Errorf("circuit breaker is open - rejecting %s operation", operationName)
	}

	// Create context with timeout
	timeoutCtx, cancel := context.WithTimeout(ctx, h.requestTimeout)
	defer cancel()

	var lastErr error
	for attempt := 0; attempt < h.maxRetries; attempt++ {
		if attempt > 0 {
			log.Warn("Retrying operation after failure",
				"component", "webhook-handler",
				"operation", operationName,
				"attempt", attempt+1,
				"max_retries", h.maxRetries,
				"previous_error", lastErr.Error())

			// Wait before retry
			select {
			case <-timeoutCtx.Done():
				h.circuitBreaker.recordFailure()
				atomic.AddInt64(&h.failureCount, 1)
				return fmt.Errorf("operation %s timed out during retry attempt %d: %w", operationName, attempt, timeoutCtx.Err())
			case <-time.After(h.retryDelay):
				// Continue to retry
			}
		}

		// Execute operation with timeout context
		operationDone := make(chan error, 1)
		go func() {
			operationDone <- operation()
		}()

		select {
		case <-timeoutCtx.Done():
			lastErr = fmt.Errorf("operation %s timed out on attempt %d: %w", operationName, attempt+1, timeoutCtx.Err())
		case err := <-operationDone:
			if err == nil {
				// Success
				h.circuitBreaker.recordSuccess()
				log.Info("Operation completed successfully",
					"component", "webhook-handler",
					"operation", operationName,
					"attempt", attempt+1)
				return nil
			}
			lastErr = err
		}

		// Log attempt failure
		log.Warn("Operation attempt failed",
			"component", "webhook-handler",
			"operation", operationName,
			"attempt", attempt+1,
			"error", lastErr.Error())
	}

	// All retries exhausted
	h.circuitBreaker.recordFailure()
	atomic.AddInt64(&h.failureCount, 1)
	log.Error("Operation failed after all retry attempts", lastErr,
		"component", "webhook-handler",
		"operation", operationName,
		"max_retries", h.maxRetries)

	return fmt.Errorf("operation %s failed after %d attempts: %w", operationName, h.maxRetries, lastErr)
}

// updateHealthMetrics updates health tracking metrics
func (h *Handler) updateHealthMetrics() {
	h.lastHealthCheck = time.Now()

	requestCount := atomic.LoadInt64(&h.requestCount)
	failureCount := atomic.LoadInt64(&h.failureCount)

	var failureRate float64
	if requestCount > 0 {
		failureRate = float64(failureCount) / float64(requestCount) * 100
	}

	cbState := h.circuitBreaker.getState()
	var stateStr string
	switch cbState {
	case CircuitBreakerClosed:
		stateStr = "closed"
	case CircuitBreakerOpen:
		stateStr = "open"
	case CircuitBreakerHalfOpen:
		stateStr = "half-open"
	}

	log.Info("Webhook health metrics updated",
		"component", "webhook-handler",
		"requests.total", requestCount,
		"requests.failed", failureCount,
		"failure.rate_percent", failureRate,
		"circuit.state", stateStr,
		"health.check_time", h.lastHealthCheck.Format(time.RFC3339))
}
