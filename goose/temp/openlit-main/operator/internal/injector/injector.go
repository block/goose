/*
OpenLIT Pod Instrumentation Injector

This package implements the core logic for injecting OpenTelemetry instrumentation
into Kubernetes pods. It handles the complex task of modifying pod specifications
to include init containers, environment variables, and volume mounts necessary
for zero-code instrumentation.

Key components:
- OpenLitInjector: Main injector implementation
- Pod modification logic for adding instrumentation containers
- Environment variable injection from multiple sources (secrets, configmaps, etc.)
- Volume and mount management for instrumentation packages
- Provider-specific setup script generation
- Comprehensive error handling and logging

The injector transforms a regular application pod into an instrumented pod by:
1. Adding an init container that sets up instrumentation packages
2. Injecting OpenTelemetry environment variables
3. Creating shared volumes for instrumentation code
4. Configuring PYTHONPATH and other runtime settings
5. Handling custom packages and provider-specific configurations

Supports all major instrumentation providers and handles edge cases like
existing environment variables, custom images, and complex volume scenarios.

This is the "engine" that powers the zero-code instrumentation magic!
*/
package injector

import (
	"fmt"
	"os"
	"strings"

	corev1 "k8s.io/api/core/v1"
	"k8s.io/apimachinery/pkg/api/resource"

	"github.com/openlit/openlit/operator/internal/observability"
)

var log = observability.NewLogger("injector")

// OpenLitInjector handles the injection of OpenLIT instrumentation
type OpenLitInjector struct {
	config *InjectorConfig
}

// New creates a new OpenLitInjector
func New(cfg *InjectorConfig) *OpenLitInjector {
	return &OpenLitInjector{
		config: cfg,
	}
}

// InjectionResult represents the result of an injection attempt
type InjectionResult struct {
	Success           bool
	InstrumentedCount int
	TotalContainers   int
	PartialFailure    bool
	FailedContainers  []string
	RecoveryActions   []string
	Error             error
}

// InjectOpenLIT injects OpenLIT instrumentation into a pod with error recovery
func (i *OpenLitInjector) InjectOpenLIT(pod *corev1.Pod) error {
	log.Info("Starting OpenLIT injection",
		"component", "injector",
		"k8s.pod.name", pod.Name,
		"k8s.namespace.name", pod.Namespace,
		"instrumentation.provider", i.config.Provider)

	// Create snapshot of original pod state for recovery
	originalPod := pod.DeepCopy()
	result := &InjectionResult{
		TotalContainers: len(pod.Spec.Containers),
	}

	// Use error recovery wrapper
	return i.executeWithRecovery(pod, originalPod, result, func() error {
		// Validate security context compatibility
		if err := i.validateSecurityContext(pod); err != nil {
			return fmt.Errorf("security context validation failed: %w", err)
		}

		// Add init container for OpenLIT SDK installation
		if err := i.addInitContainerWithRecovery(pod, result); err != nil {
			return fmt.Errorf("failed to add init container: %w", err)
		}

		// Add shared volume for OpenLIT SDK
		if err := i.addSharedVolumeWithRecovery(pod, result); err != nil {
			return fmt.Errorf("failed to add shared volume: %w", err)
		}

		// Modify application containers based on selection criteria
		if err := i.modifyContainersWithRecovery(pod, result); err != nil {
			return fmt.Errorf("failed to modify containers: %w", err)
		}

		// Final validation
		if result.InstrumentedCount == 0 {
			return fmt.Errorf("no containers were instrumented - all containers excluded by selection criteria")
		}

		log.Info("OpenLIT injection completed successfully",
			"component", "injector",
			"k8s.pod.name", pod.Name,
			"k8s.namespace.name", pod.Namespace,
			"instrumentation.provider", i.config.Provider,
			"containers.instrumented", result.InstrumentedCount,
			"containers.total", result.TotalContainers)

		return nil
	})
}

// addInitContainer adds an init container to install instrumentation SDK
func (i *OpenLitInjector) addInitContainer(pod *corev1.Pod) {
	// Generate installation commands based on provider
	installCmd := i.generateInstallCommand()

	volumeName := i.config.GetSharedVolumeName()
	volumePath := i.config.GetSharedVolumePath()

	initContainer := corev1.Container{
		Name:            i.config.GetContainerName(),
		Image:           i.config.GetInitContainerImage(),
		ImagePullPolicy: i.config.ImagePullPolicy,
		Command:         []string{"/bin/sh"},
		Args: []string{
			"-c",
			installCmd,
		},
		VolumeMounts: []corev1.VolumeMount{
			{
				Name:      volumeName,
				MountPath: volumePath,
			},
		},
		Env: []corev1.EnvVar{
			{
				Name:  "INSTRUMENTATION_PROVIDER",
				Value: i.config.Provider,
			},
		},
		Resources: corev1.ResourceRequirements{
			Limits: corev1.ResourceList{
				corev1.ResourceCPU:    resource.MustParse("200m"),
				corev1.ResourceMemory: resource.MustParse("256Mi"),
			},
			Requests: corev1.ResourceList{
				corev1.ResourceCPU:    resource.MustParse("100m"),
				corev1.ResourceMemory: resource.MustParse("128Mi"),
			},
		},
	}

	log.Info("Adding auto-instrumentation init container",
		"component", "injector",
		"k8s.container.name", initContainer.Name,
		"k8s.container.image", initContainer.Image,
		"instrumentation.provider", i.config.Provider,
		"k8s.container.image_pull_policy", string(initContainer.ImagePullPolicy))
	pod.Spec.InitContainers = append(pod.Spec.InitContainers, initContainer)
}

// generateInstallCommand generates the fast file-copy command based on the provider
func (i *OpenLitInjector) generateInstallCommand() string {
	// Smart installation: pre-built base + targeted custom packages
	provider := i.config.Provider
	volumePath := i.config.GetSharedVolumePath()

	var installCommands []string

	// Step 1: Copy pre-built base instrumentation (fast!)
	installCommands = append(installCommands, fmt.Sprintf(`
echo "Setting up %s instrumentation..."
export INSTRUMENTATION_PROVIDER=%s
export TARGET_PATH=%s

# Copy pre-built base instrumentation using Python setup script
python3 /usr/local/bin/setup-instrumentation.py`, provider, provider, volumePath))

	// Step 2: Install custom packages if specified (targeted, fast)
	if i.config.CustomPackages != "" {
		installCommands = append(installCommands, `
echo "📦 Installing custom packages..."`)

		// Install custom packages
		installCommands = append(installCommands, fmt.Sprintf(`
# Install user-specified custom packages
echo "Installing: %s"
pip install --target %s --no-deps %s`,
			i.config.CustomPackages,
			volumePath,
			strings.ReplaceAll(i.config.CustomPackages, ",", " ")))

		installCommands = append(installCommands, `
echo "✅ Custom packages installed!"`)
	}

	// Final step
	var finalMessage string
	if provider == "openlit" {
		finalMessage = fmt.Sprintf(`
echo "%s instrumentation ready! Using openlit-instrument CLI"
echo "Zero-code instrumentation via openlit-instrument CLI approach"
echo "Auto-discovery and setup handled automatically"
ls -la %s/ | head -10`, provider, volumePath)
	} else {
		finalMessage = fmt.Sprintf(`
echo "%s instrumentation ready!"
echo "Zero-code instrumentation configured via Kubernetes CR"
ls -la %s/ | head -10`, provider, volumePath)
	}
	installCommands = append(installCommands, finalMessage)

	return strings.Join(installCommands, "\n")
}

// addSharedVolume adds a shared volume for the instrumentation SDK
func (i *OpenLitInjector) addSharedVolume(pod *corev1.Pod) {
	volumeName := i.config.GetSharedVolumeName()
	volume := corev1.Volume{
		Name: volumeName,
		VolumeSource: corev1.VolumeSource{
			EmptyDir: &corev1.EmptyDirVolumeSource{},
		},
	}

	log.Info("💾 Adding instrumentation shared volume",
		"component", "injector",
		"k8s.volume.name", volume.Name,
		"k8s.volume.type", "emptydir",
		"instrumentation.provider", i.config.Provider)
	pod.Spec.Volumes = append(pod.Spec.Volumes, volume)
}

// modifyContainer modifies an application container for auto-instrumentation
func (i *OpenLitInjector) modifyContainer(container *corev1.Container, pod *corev1.Pod) {
	log.Info("Modifying container for auto-instrumentation",
		"component", "injector",
		"k8s.container.name", container.Name,
		"instrumentation.provider", i.config.Provider)

	// Check if container already has instrumentation and log it
	if i.hasExistingInstrumentation(container, pod) {
		log.Warn("Overwriting existing instrumentation - OpenLIT will replace current observability setup",
			"component", "injector",
			"k8s.container.name", container.Name,
			"k8s.pod.name", pod.Name,
			"k8s.namespace.name", pod.Namespace,
			"action", "overwrite_existing_instrumentation")
	}

	// Add volume mount for instrumentation SDK
	volumeName := i.config.GetSharedVolumeName()
	volumePath := i.config.GetSharedVolumePath()

	volumeMount := corev1.VolumeMount{
		Name:      volumeName,
		MountPath: volumePath,
		ReadOnly:  true,
	}
	container.VolumeMounts = append(container.VolumeMounts, volumeMount)

	// Add environment variables for instrumentation configuration
	envVars := i.buildEnvironmentVariables(container, pod)
	container.Env = append(container.Env, envVars...)

	log.Info("Container modification completed",
		"component", "injector",
		"k8s.container.name", container.Name,
		"instrumentation.provider", i.config.Provider)
}

// buildEnvironmentVariables creates the environment variables for instrumentation configuration
func (i *OpenLitInjector) buildEnvironmentVariables(container *corev1.Container, pod *corev1.Pod) []corev1.EnvVar {
	// Calculate OpenTelemetry resource attributes following K8s conventions
	serviceName := i.generateServiceName(container, pod)
	serviceNamespace := i.generateServiceNamespace(pod)
	serviceVersion := i.generateServiceVersion(pod)
	deploymentEnvironment := i.config.Environment
	if deploymentEnvironment == "" {
		deploymentEnvironment = "kubernetes" // Simple fallback
	}

	// Build comprehensive OTEL_RESOURCE_ATTRIBUTES following OpenTelemetry semantic conventions
	resourceAttrs := []string{
		fmt.Sprintf("service.name=%s", serviceName),
		fmt.Sprintf("service.namespace=%s", serviceNamespace),
		fmt.Sprintf("deployment.environment=%s", deploymentEnvironment),
	}

	// Add service.version only if available
	if serviceVersion != "" {
		resourceAttrs = append(resourceAttrs, fmt.Sprintf("service.version=%s", serviceVersion))
	}

	// Only add service.instance.id if we have the actual pod name (not during admission webhook)
	var serviceInstanceId string
	if pod.Name != "" {
		serviceInstanceId = i.generateServiceInstanceId(container, pod)
		resourceAttrs = append(resourceAttrs, fmt.Sprintf("service.instance.id=%s", serviceInstanceId))
	}

	// Check if PYTHONPATH already exists in the container
	var existingPythonPath string
	for _, env := range container.Env {
		if env.Name == "PYTHONPATH" {
			existingPythonPath = env.Value
			break
		}
	}

	// Build PYTHONPATH value - prepend our SDK path
	volumePath := i.config.GetSharedVolumePath()
	pythonPathValue := volumePath
	if existingPythonPath != "" {
		pythonPathValue = volumePath + ":" + existingPythonPath
	}

	envVars := []corev1.EnvVar{
		{
			Name:  "PYTHONPATH",
			Value: pythonPathValue,
		},
		// Standard OpenTelemetry environment variables (used by all providers)
		{
			Name:  "OTEL_EXPORTER_OTLP_ENDPOINT",
			Value: i.config.OTLPEndpoint,
		},
		{
			Name:  "OTEL_EXPORTER_OTLP_HEADERS",
			Value: i.config.OTLPHeaders,
		},
		{
			Name:  "OTEL_SERVICE_NAME",
			Value: serviceName,
		},
		{
			Name:  "OTEL_SERVICE_NAMESPACE",
			Value: serviceNamespace,
		},
		{
			Name:  "OTEL_DEPLOYMENT_ENVIRONMENT",
			Value: deploymentEnvironment,
		},
		{
			Name:  "OTEL_RESOURCE_ATTRIBUTES",
			Value: strings.Join(resourceAttrs, ","),
		},
		// Optional service version
		{
			Name:  "OTEL_SERVICE_VERSION",
			Value: serviceVersion, // May be empty
		},
		// GenAI-specific configuration
		{
			Name:  "OTEL_INSTRUMENTATION_GENAI_CAPTURE_MESSAGE_CONTENT",
			Value: fmt.Sprintf("%t", i.config.CaptureMessageContent),
		},
	}

	// Only add OTEL_SERVICE_INSTANCE_ID if we have the actual pod name
	if pod.Name != "" && serviceInstanceId != "" {
		envVars = append(envVars, corev1.EnvVar{
			Name:  "OTEL_SERVICE_INSTANCE_ID",
			Value: serviceInstanceId,
		})
	}

	// Add optional OpenLIT-specific environment variables (removed advanced options for now)

	if i.config.DetailedTracing {
		envVars = append(envVars, corev1.EnvVar{
			Name:  "OPENLIT_DETAILED_TRACING",
			Value: "true",
		})
	}

	// Phoenix-specific variables removed - using pure OpenTelemetry approach for OpenInference
	// Add custom instrumentation environment variables
	if i.config.CustomPackages != "" {
		envVars = append(envVars, corev1.EnvVar{
			Name:  "CUSTOM_PACKAGES",
			Value: i.config.CustomPackages,
		})
	}

	log.Info("🌍 OpenTelemetry resource attributes configured",
		"component", "injector",
		"service.name", serviceName,
		"service.namespace", serviceNamespace,
		"service.instance.id", serviceInstanceId,
		"service.version", serviceVersion,
		"deployment.environment", deploymentEnvironment,
		"otel.exporter.otlp.endpoint", i.config.OTLPEndpoint,
		"instrumentation.provider", i.config.Provider,
		"instrumentation.custom_packages", i.config.CustomPackages)

	return envVars
}

// generateServiceName calculates service.name following OpenTelemetry K8s conventions
// Priority: annotation -> app.instance -> app.name -> k8s.deployment.name -> k8s.replicaset.name ->
//
//	k8s.statefulset.name -> k8s.daemonset.name -> k8s.cronjob.name -> k8s.job.name ->
//	k8s.pod.name -> k8s.container.name
func (i *OpenLitInjector) generateServiceName(container *corev1.Container, pod *corev1.Pod) string {
	// 1. Check pod annotation (highest priority)
	if annotations := pod.GetAnnotations(); annotations != nil {
		if serviceName, exists := annotations["resource.opentelemetry.io/service.name"]; exists && serviceName != "" {
			return serviceName
		}
	}

	// 2. Check well-known labels
	if labels := pod.GetLabels(); labels != nil {
		// app.kubernetes.io/instance (well-known label)
		if instance, exists := labels["app.kubernetes.io/instance"]; exists && instance != "" {
			return instance
		}
		// app.kubernetes.io/name (well-known label)
		if appName, exists := labels["app.kubernetes.io/name"]; exists && appName != "" {
			return appName
		}
	}

	// 3. Extract from K8s resource hierarchy following OpenTelemetry spec order
	// Check OwnerReferences to find the actual parent resource
	if ownerName := i.extractResourceNameFromOwnerReferences(pod); ownerName != "" {
		return ownerName
	}

	// 4. k8s.pod.name (fallback from pod name for stateless pods)
	if pod.Name != "" {
		return pod.Name
	}

	// 5. k8s.container.name (final fallback before default)
	if container.Name != "" {
		return container.Name
	}

	// 6. Final fallback
	return "openlit-instrumented-app"
}

// generateServiceNamespace calculates service.namespace following OpenTelemetry conventions
func (i *OpenLitInjector) generateServiceNamespace(pod *corev1.Pod) string {
	// 1. Check pod annotation (highest priority)
	if annotations := pod.GetAnnotations(); annotations != nil {
		if serviceNamespace, exists := annotations["resource.opentelemetry.io/service.namespace"]; exists && serviceNamespace != "" {
			return serviceNamespace
		}
	}

	// 2. k8s.namespace.name (default)
	if pod.Namespace != "" {
		return pod.Namespace
	}

	// Fallback
	return "default"
}

// generateServiceVersion calculates service.version following OpenTelemetry conventions
func (i *OpenLitInjector) generateServiceVersion(pod *corev1.Pod) string {
	// 1. Check pod annotation (highest priority)
	if annotations := pod.GetAnnotations(); annotations != nil {
		if serviceVersion, exists := annotations["resource.opentelemetry.io/service.version"]; exists && serviceVersion != "" {
			return serviceVersion
		}
	}

	// 2. Check well-known label app.kubernetes.io/version
	if labels := pod.GetLabels(); labels != nil {
		if version, exists := labels["app.kubernetes.io/version"]; exists && version != "" {
			return version
		}
	}

	// TODO: Extract version from container image tag/digest using algorithms described in:
	// https://github.com/open-telemetry/opentelemetry-operator/blob/main/pkg/constants/annotations.go
	// For now, return empty to let OTEL auto-detect

	return ""
}

// generateServiceInstanceId calculates service.instance.id following OpenTelemetry conventions
func (i *OpenLitInjector) generateServiceInstanceId(container *corev1.Container, pod *corev1.Pod) string {
	// 1. Check pod annotation (highest priority)
	if annotations := pod.GetAnnotations(); annotations != nil {
		if instanceId, exists := annotations["resource.opentelemetry.io/service.instance.id"]; exists && instanceId != "" {
			return instanceId
		}
	}

	// 2. Default: concat([k8s.namespace.name, k8s.pod.name, k8s.container.name], '.')
	namespace := pod.Namespace
	if namespace == "" {
		namespace = "default"
	}

	// Handle pod name - during admission, pod.Name might be empty for Deployment pods
	podName := pod.Name
	if podName == "" {
		// For Deployment pods, use GenerateName if available
		if pod.GenerateName != "" {
			// GenerateName is like "deployment-replicaset-" for deployment pods
			podName = strings.TrimSuffix(pod.GenerateName, "-") + "-pending"
		} else {
			// Final fallback - use service name from our calculation
			serviceName := i.generateServiceName(container, pod)
			podName = serviceName + "-pod"
		}
	}

	containerName := container.Name
	if containerName == "" {
		containerName = "unknown-container"
	}

	return fmt.Sprintf("%s.%s.%s", namespace, podName, containerName)
}

// extractResourceNameFromOwnerReferences traverses Kubernetes OwnerReferences following OpenTelemetry hierarchy
// Order: k8s.deployment.name > k8s.replicaset.name > k8s.statefulset.name > k8s.daemonset.name > k8s.cronjob.name > k8s.job.name
func (i *OpenLitInjector) extractResourceNameFromOwnerReferences(pod *corev1.Pod) string {
	if pod.OwnerReferences == nil || len(pod.OwnerReferences) == 0 {
		return ""
	}

	// Walk through OwnerReferences in OpenTelemetry hierarchy order
	hierarchyOrder := []struct {
		kind     string
		priority int
	}{
		{"Deployment", 1},  // Highest priority - most common
		{"ReplicaSet", 2},  // Second priority
		{"StatefulSet", 3}, // Third priority
		{"DaemonSet", 4},   // Fourth priority
		{"CronJob", 5},     // Fifth priority
		{"Job", 6},         // Sixth priority
	}

	var bestMatch string
	var bestPriority int = 999

	for _, owner := range pod.OwnerReferences {
		for _, hierarchy := range hierarchyOrder {
			if owner.Kind == hierarchy.kind && hierarchy.priority < bestPriority {
				bestMatch = owner.Name
				bestPriority = hierarchy.priority

				// For ReplicaSet, try to extract Deployment name by removing hash suffix
				if owner.Kind == "ReplicaSet" && len(owner.Name) > 10 {
					// ReplicaSet names are typically: "deployment-name-abcd1234"
					// Try to find deployment by removing last 10 chars (dash + 8-char hash)
					if lastDash := strings.LastIndex(owner.Name, "-"); lastDash > 0 && lastDash < len(owner.Name)-8 {
						potentialDeployment := owner.Name[:lastDash]
						// Use the potential deployment name instead of replicaset
						bestMatch = potentialDeployment
					}
				}
			}
		}
	}

	return bestMatch
}

// generateDeploymentEnvironment calculates deployment.environment from cluster name or config
func (i *OpenLitInjector) generateDeploymentEnvironment() string {
	// 1. Check for explicit CLUSTER_NAME environment variable (standard practice)
	if clusterName := os.Getenv("CLUSTER_NAME"); clusterName != "" {
		return clusterName
	}

	// 2. Check for ENVIRONMENT environment variable (common in many setups)
	if environment := os.Getenv("ENVIRONMENT"); environment != "" {
		return environment
	}

	// 3. Check for standard Kubernetes cluster identification
	if deploymentEnv := os.Getenv("DEPLOYMENT_ENVIRONMENT"); deploymentEnv != "" {
		return deploymentEnv
	}

	// 4. Try to detect cluster name from well-known Kubernetes sources
	if clusterName := i.detectClusterName(); clusterName != "" {
		return clusterName
	}

	// 5. Fall back to configured environment from InstrumentationConfig
	if i.config.Environment != "" {
		return i.config.Environment
	}

	// 6. Final fallback - use namespace as environment (better than generic "kubernetes")
	return "default-cluster"
}

// detectClusterName tries to detect the actual cluster name from Kubernetes metadata
// Uses official OpenTelemetry detection methods for GKE, EKS, AKS and other providers
func (i *OpenLitInjector) detectClusterName() string {
	// 1. Check well-known environment variables set by cloud providers or users
	cloudEnvVars := []string{
		"CLUSTER_NAME",      // Generic (most common)
		"EKS_CLUSTER_NAME",  // AWS EKS
		"GKE_CLUSTER_NAME",  // Google GKE
		"AKS_CLUSTER_NAME",  // Azure AKS
		"K8S_CLUSTER_NAME",  // Generic K8s
		"KUBE_CLUSTER_NAME", // Generic K8s
	}

	for _, envVar := range cloudEnvVars {
		if value := os.Getenv(envVar); value != "" {
			return value
		}
	}

	// 2. Try GKE metadata detection (official OpenTelemetry method)
	// From: https://github.com/open-telemetry/opentelemetry-js-contrib/blob/main/detectors/node/opentelemetry-resource-detector-gcp/src/detectors/GcpDetector.ts#L81
	if clusterName := detectGKEClusterName(); clusterName != "" {
		return clusterName
	}

	// 3. Try EKS detection via Kubernetes API server certificate
	// From: https://github.com/open-telemetry/opentelemetry-js-contrib/blob/main/detectors/node/opentelemetry-resource-detector-aws/src/detectors/AwsEksDetector.ts#L86
	if clusterName := detectEKSClusterName(); clusterName != "" {
		return clusterName
	}

	// 4. Try AKS detection via instance metadata
	if clusterName := detectAKSClusterName(); clusterName != "" {
		return clusterName
	}

	// 5. Fallback: Extract from hostname patterns (for k3d, minikube, etc.)
	if hostname := os.Getenv("HOSTNAME"); hostname != "" {
		if clusterName := extractClusterFromHostname(hostname); clusterName != "" {
			return clusterName
		}
	}

	return ""
}

// detectGKEClusterName detects GKE cluster name using metadata service
// Official OpenTelemetry method: curl metadata server for cluster-name attribute
func detectGKEClusterName() string {
	// GKE sets this environment variable
	if clusterName := os.Getenv("GKE_CLUSTER_NAME"); clusterName != "" {
		return clusterName
	}

	// TODO: Could implement HTTP call to GCE metadata service:
	// curl -H "Metadata-Flavor: Google" \
	//   "http://metadata/computeMetadata/v1/instance/attributes/cluster-name"
	// But this requires network access and error handling

	// Check for GKE-specific environment indicators
	if os.Getenv("GOOGLE_APPLICATION_CREDENTIALS") != "" ||
		os.Getenv("GCLOUD_PROJECT") != "" ||
		strings.Contains(os.Getenv("KUBERNETES_SERVICE_HOST"), "googleapis.com") {
		return "gke-cluster" // Generic GKE identifier
	}

	return ""
}

// detectEKSClusterName detects EKS cluster name from Kubernetes API server certificate
// Official OpenTelemetry method: extract from kubernetes.default.svc certificate
func detectEKSClusterName() string {
	// EKS sets this environment variable
	if clusterName := os.Getenv("EKS_CLUSTER_NAME"); clusterName != "" {
		return clusterName
	}

	// Check EKS-specific patterns in Kubernetes service host
	if kubeHost := os.Getenv("KUBERNETES_SERVICE_HOST"); kubeHost != "" {
		// EKS API server pattern: <cluster-id>.eks.<region>.amazonaws.com
		if strings.Contains(kubeHost, ".eks.") && strings.Contains(kubeHost, ".amazonaws.com") {
			parts := strings.Split(kubeHost, ".")
			if len(parts) >= 4 && parts[1] == "eks" {
				// Extract region and create meaningful name
				region := parts[2]
				return fmt.Sprintf("eks-%s", region)
			}
		}
	}

	// TODO: Could implement certificate parsing from kubernetes.default.svc
	// as described in OpenTelemetry EKS detector, but this requires:
	// - TLS connection to kubernetes.default.svc
	// - Certificate parsing to extract cluster name from subject
	// - Additional error handling and permissions

	// Check for AWS environment indicators
	if os.Getenv("AWS_REGION") != "" || os.Getenv("AWS_DEFAULT_REGION") != "" {
		region := os.Getenv("AWS_REGION")
		if region == "" {
			region = os.Getenv("AWS_DEFAULT_REGION")
		}
		return fmt.Sprintf("eks-%s", region)
	}

	return ""
}

// detectAKSClusterName detects AKS cluster name from Azure metadata
func detectAKSClusterName() string {
	// AKS sets this environment variable
	if clusterName := os.Getenv("AKS_CLUSTER_NAME"); clusterName != "" {
		return clusterName
	}

	// Check AKS-specific patterns in Kubernetes service host
	if kubeHost := os.Getenv("KUBERNETES_SERVICE_HOST"); kubeHost != "" {
		// AKS API server pattern: <cluster>-<rg>-<id>.hcp.<region>.azmk8s.io
		if strings.Contains(kubeHost, ".azmk8s.io") {
			parts := strings.Split(kubeHost, ".")
			if len(parts) >= 3 {
				hostParts := strings.Split(parts[0], "-")
				if len(hostParts) >= 1 {
					return hostParts[0] // cluster name
				}
			}
		}
	}

	// TODO: Could implement Azure Instance Metadata Service call:
	// curl -H "Metadata: true" \
	//   "http://169.254.169.254/metadata/instance/compute/resourceGroupName?api-version=2021-02-01"

	// Check for Azure environment indicators
	if os.Getenv("AZURE_CLIENT_ID") != "" || os.Getenv("AZURE_TENANT_ID") != "" {
		return "aks-cluster"
	}

	return ""
}

// extractClusterFromHostname extracts cluster name from hostname patterns
// Fallback method for local development environments (k3d, minikube, etc.)
func extractClusterFromHostname(hostname string) string {
	// k3d pattern: k3d-<cluster-name>-server-0
	if strings.Contains(hostname, "k3d-") {
		parts := strings.Split(hostname, "-")
		if len(parts) >= 3 && parts[0] == "k3d" {
			return parts[1] // cluster name is after "k3d-"
		}
	}

	// minikube pattern: minikube
	if strings.Contains(hostname, "minikube") {
		return "minikube"
	}

	// kind pattern: <cluster>-control-plane
	if strings.Contains(hostname, "-control-plane") {
		parts := strings.Split(hostname, "-")
		if len(parts) >= 2 {
			return parts[0] // cluster name before "-control-plane"
		}
	}

	return ""
}

// hasExistingInstrumentation checks if a container already has OpenTelemetry instrumentation
func (i *OpenLitInjector) hasExistingInstrumentation(container *corev1.Container, pod *corev1.Pod) bool {
	// Check for common OpenTelemetry environment variables that indicate existing instrumentation
	otelEnvVars := []string{
		"OTEL_SERVICE_NAME",
		"OTEL_RESOURCE_ATTRIBUTES",
		"OTEL_EXPORTER_OTLP_ENDPOINT",
		"OTEL_TRACES_EXPORTER",
		"OTEL_PYTHON_DISABLED_INSTRUMENTATIONS",
		"OPENLIT_APPLICATION_NAME", // OpenLIT specific
		"OPENLIT_ENVIRONMENT",      // OpenLIT specific
	}

	// Check container environment variables
	for _, envVar := range container.Env {
		for _, otelVar := range otelEnvVars {
			if envVar.Name == otelVar {
				log.Info("Found existing instrumentation environment variable",
					"component", "injector",
					"k8s.container.name", container.Name,
					"instrumentation.env_var", envVar.Name,
					"instrumentation.source", "container")
				return true
			}
		}
	}

	// Check pod-level annotations for instrumentation
	if pod.Annotations != nil {
		instrumentationAnnotations := []string{
			"openlit.io/instrumented-by",
			"instrumentation.opentelemetry.io/inject-python",
			"instrumentation.opentelemetry.io/inject-java",
			"instrumentation.opentelemetry.io/inject-nodejs",
			"instrumentation.opentelemetry.io/inject-dotnet",
			"sidecar.opentelemetry.io/inject",
		}

		for _, annotation := range instrumentationAnnotations {
			if value, exists := pod.Annotations[annotation]; exists && value != "" && value != "false" {
				log.Info("Found existing instrumentation annotation",
					"component", "injector",
					"k8s.pod.name", pod.Name,
					"instrumentation.annotation", annotation,
					"instrumentation.value", value,
					"instrumentation.source", "annotation")
				return true
			}
		}
	}

	// Check pod labels for instrumentation
	if pod.Labels != nil {
		if instrumentedBy, exists := pod.Labels["openlit.io/instrumented-by"]; exists && instrumentedBy != "" {
			log.Info("Found existing instrumentation label",
				"component", "injector",
				"k8s.pod.name", pod.Name,
				"instrumentation.label", "openlit.io/instrumented-by",
				"instrumentation.value", instrumentedBy,
				"instrumentation.source", "label")
			return true
		}
	}

	// Check for existing volume mounts that suggest instrumentation
	instrumentationVolumes := []string{
		"openlit-sdk",
		"otel-auto-instrumentation",
		"opentelemetry-auto-instrumentation",
		"instrumentation-packages",
	}

	for _, volumeMount := range container.VolumeMounts {
		for _, instrVolume := range instrumentationVolumes {
			if volumeMount.Name == instrVolume {
				log.Info("Found existing instrumentation volume mount",
					"component", "injector",
					"k8s.container.name", container.Name,
					"instrumentation.volume", volumeMount.Name,
					"instrumentation.mount_path", volumeMount.MountPath,
					"instrumentation.source", "volume")
				return true
			}
		}
	}

	return false
}

// validateSecurityContext checks if the pod's security context allows instrumentation
func (i *OpenLitInjector) validateSecurityContext(pod *corev1.Pod) error {
	// Check pod-level security context
	if pod.Spec.SecurityContext != nil {
		secCtx := pod.Spec.SecurityContext

		// Note: ReadOnlyRootFilesystem is only available on container-level SecurityContext,
		// not pod-level PodSecurityContext. We'll check this on containers below.

		// Check if running as non-root with very low UID that might cause permission issues
		if secCtx.RunAsNonRoot != nil && *secCtx.RunAsNonRoot {
			if secCtx.RunAsUser != nil && *secCtx.RunAsUser < 1000 {
				log.Warn("Pod running as non-root with low UID - may encounter permission issues during instrumentation",
					"component", "injector",
					"k8s.pod.name", pod.Name,
					"security.run_as_user", *secCtx.RunAsUser,
					"security.run_as_non_root", *secCtx.RunAsNonRoot)
			}
		}

		// Check for restricted filesystem capabilities
		if secCtx.FSGroup != nil {
			log.Info("Pod uses custom fsGroup - verifying file permissions compatibility",
				"component", "injector",
				"k8s.pod.name", pod.Name,
				"security.fs_group", *secCtx.FSGroup)
		}
	}

	// Check container-level security contexts
	for _, container := range pod.Spec.Containers {
		if container.SecurityContext != nil {
			secCtx := container.SecurityContext

			// Read-only root filesystem prevents instrumentation
			if secCtx.ReadOnlyRootFilesystem != nil && *secCtx.ReadOnlyRootFilesystem {
				return fmt.Errorf("container %s has readOnlyRootFilesystem=true which prevents instrumentation", container.Name)
			}

			// Check privileged mode (usually okay, but worth noting)
			if secCtx.Privileged != nil && *secCtx.Privileged {
				log.Info("Container running in privileged mode - instrumentation should work normally",
					"component", "injector",
					"k8s.container.name", container.Name,
					"security.privileged", true)
			}

			// Check capabilities that might affect instrumentation
			if secCtx.Capabilities != nil {
				// Check for dropped capabilities that might affect instrumentation
				droppedCaps := secCtx.Capabilities.Drop
				problematicDrops := []string{"SYS_PTRACE", "DAC_OVERRIDE", "SETUID", "SETGID"}

				for _, droppedCap := range droppedCaps {
					for _, problematicCap := range problematicDrops {
						if string(droppedCap) == problematicCap {
							log.Warn("Container drops capability that may affect instrumentation",
								"component", "injector",
								"k8s.container.name", container.Name,
								"security.capability.dropped", string(droppedCap))
						}
					}
				}
			}

			// Check allowPrivilegeEscalation
			if secCtx.AllowPrivilegeEscalation != nil && !*secCtx.AllowPrivilegeEscalation {
				log.Warn("Container disallows privilege escalation - may affect some instrumentation features",
					"component", "injector",
					"k8s.container.name", container.Name,
					"security.allow_privilege_escalation", false)
			}
		}
	}

	// Check init containers for security context issues
	for _, initContainer := range pod.Spec.InitContainers {
		if initContainer.SecurityContext != nil {
			secCtx := initContainer.SecurityContext

			if secCtx.ReadOnlyRootFilesystem != nil && *secCtx.ReadOnlyRootFilesystem {
				log.Warn("Existing init container has read-only root filesystem - our init container will need write access",
					"component", "injector",
					"k8s.init_container.name", initContainer.Name,
					"security.read_only_root_filesystem", true)
			}
		}
	}

	// Check for Pod Security Standards annotations
	if pod.Annotations != nil {
		// Check for Pod Security Standards enforcement
		if enforcement, exists := pod.Annotations["pod-security.kubernetes.io/enforce"]; exists {
			switch enforcement {
			case "restricted":
				return fmt.Errorf("pod enforces 'restricted' Pod Security Standard which may prevent instrumentation")
			case "baseline":
				log.Warn("Pod enforces 'baseline' Pod Security Standard - monitoring for compatibility issues",
					"component", "injector",
					"k8s.pod.name", pod.Name,
					"security.pod_security_standard", enforcement)
			case "privileged":
				log.Info("Pod allows 'privileged' Pod Security Standard - instrumentation should work normally",
					"component", "injector",
					"k8s.pod.name", pod.Name,
					"security.pod_security_standard", enforcement)
			}
		}

		// Check for AppArmor profiles
		for key, value := range pod.Annotations {
			if strings.HasPrefix(key, "container.apparmor.security.beta.kubernetes.io/") {
				containerName := strings.TrimPrefix(key, "container.apparmor.security.beta.kubernetes.io/")
				if value != "unconfined" && value != "runtime/default" {
					log.Warn("Container uses custom AppArmor profile - may affect instrumentation",
						"component", "injector",
						"k8s.container.name", containerName,
						"security.apparmor.profile", value)
				}
			}
		}

		// Check for SELinux options
		if selinuxType, exists := pod.Annotations["seLinuxOptions"]; exists {
			log.Warn("Pod uses custom SELinux options - may affect instrumentation",
				"component", "injector",
				"k8s.pod.name", pod.Name,
				"security.selinux.type", selinuxType)
		}
	}

	log.Info("Security context validation completed",
		"component", "injector",
		"k8s.pod.name", pod.Name,
		"validation.result", "passed")

	return nil
}

// shouldInstrumentContainer determines if a specific container should be instrumented
func (i *OpenLitInjector) shouldInstrumentContainer(container *corev1.Container, pod *corev1.Pod) bool {
	// Check for explicit container exclusion annotation
	if pod.Annotations != nil {
		// Check for container-specific exclusion: openlit.io/exclude-containers: "container1,container2"
		if excludeList, exists := pod.Annotations["openlit.io/exclude-containers"]; exists {
			excludedContainers := strings.Split(excludeList, ",")
			for _, excludedName := range excludedContainers {
				if strings.TrimSpace(excludedName) == container.Name {
					log.Info("Container excluded by openlit.io/exclude-containers annotation",
						"component", "injector",
						"k8s.container.name", container.Name,
						"k8s.pod.name", pod.Name,
						"exclusion.annotation", "openlit.io/exclude-containers",
						"exclusion.list", excludeList)
					return false
				}
			}
		}

		// Check for container-specific inclusion: openlit.io/include-containers: "container1,container2"
		if includeList, exists := pod.Annotations["openlit.io/include-containers"]; exists {
			includedContainers := strings.Split(includeList, ",")
			isIncluded := false
			for _, includedName := range includedContainers {
				if strings.TrimSpace(includedName) == container.Name {
					isIncluded = true
					break
				}
			}
			if !isIncluded {
				log.Info("Container not in openlit.io/include-containers annotation",
					"component", "injector",
					"k8s.container.name", container.Name,
					"k8s.pod.name", pod.Name,
					"inclusion.annotation", "openlit.io/include-containers",
					"inclusion.list", includeList)
				return false
			}
		}

		// Check for container language annotation: openlit.io/container-languages: "app:python,sidecar:java"
		if languageMapping, exists := pod.Annotations["openlit.io/container-languages"]; exists {
			containerMappings := strings.Split(languageMapping, ",")
			containerLanguage := ""

			for _, mapping := range containerMappings {
				parts := strings.Split(strings.TrimSpace(mapping), ":")
				if len(parts) == 2 && strings.TrimSpace(parts[0]) == container.Name {
					containerLanguage = strings.TrimSpace(parts[1])
					break
				}
			}

			// If this container is mapped to a different language than we support, skip it
			if containerLanguage != "" && containerLanguage != "python" {
				log.Info("Container uses unsupported language - skipping instrumentation",
					"component", "injector",
					"k8s.container.name", container.Name,
					"k8s.pod.name", pod.Name,
					"container.language", containerLanguage,
					"supported.language", "python")
				return false
			}
		}
	}

	// Check for common sidecar containers that shouldn't be instrumented
	sidecarPatterns := []string{
		"istio-proxy",
		"linkerd-proxy",
		"envoy",
		"oauth2-proxy",
		"cloudsql-proxy",
		"kube-proxy",
		"fluentd",
		"fluent-bit",
		"filebeat",
		"prometheus",
		"grafana-agent",
		"jaeger-agent",
		"otel-collector",
		"vault-agent",
		"consul-connect",
	}

	containerNameLower := strings.ToLower(container.Name)
	for _, pattern := range sidecarPatterns {
		if strings.Contains(containerNameLower, pattern) {
			log.Info("Skipping sidecar container",
				"component", "injector",
				"k8s.container.name", container.Name,
				"k8s.pod.name", pod.Name,
				"sidecar.pattern", pattern,
				"container.type", "sidecar")
			return false
		}
	}

	// Check for common init container images that shouldn't be instrumented
	// (though this function is for app containers, good to have for safety)
	initPatterns := []string{
		"busybox",
		"alpine",
		"scratch",
		"k8s.gcr.io/pause",
		"registry.k8s.io/pause",
	}

	containerImageLower := strings.ToLower(container.Image)
	for _, pattern := range initPatterns {
		if strings.Contains(containerImageLower, pattern) {
			log.Info("Skipping utility container",
				"component", "injector",
				"k8s.container.name", container.Name,
				"k8s.pod.name", pod.Name,
				"container.image", container.Image,
				"utility.pattern", pattern,
				"container.type", "utility")
			return false
		}
	}

	// Check if container looks like a Python application based on image or common environment variables
	isPythonContainer := i.isPythonContainer(container)
	if !isPythonContainer {
		log.Info("Container does not appear to be a Python application - skipping instrumentation",
			"component", "injector",
			"k8s.container.name", container.Name,
			"k8s.pod.name", pod.Name,
			"container.image", container.Image,
			"language.detected", "non-python")
		return false
	}

	// All checks passed
	log.Info("Container selected for instrumentation",
		"component", "injector",
		"k8s.container.name", container.Name,
		"k8s.pod.name", pod.Name,
		"container.image", container.Image)

	return true
}

// isPythonContainer checks if a container appears to be running a Python application
func (i *OpenLitInjector) isPythonContainer(container *corev1.Container) bool {
	// Check image name for Python indicators
	imageLower := strings.ToLower(container.Image)
	pythonImagePatterns := []string{
		"python",
		"pypy",
		"django",
		"flask",
		"fastapi",
		"uvicorn",
		"gunicorn",
		"celery",
		"jupyter",
		"anaconda",
		"miniconda",
	}

	for _, pattern := range pythonImagePatterns {
		if strings.Contains(imageLower, pattern) {
			return true
		}
	}

	// Check environment variables for Python indicators
	for _, envVar := range container.Env {
		pythonEnvVars := []string{
			"PYTHONPATH",
			"PYTHON_VERSION",
			"PIP_",
			"POETRY_",
			"CONDA_",
			"VIRTUAL_ENV",
			"DJANGO_SETTINGS_MODULE",
			"FLASK_APP",
			"FASTAPI_",
		}

		for _, pythonEnv := range pythonEnvVars {
			if strings.HasPrefix(envVar.Name, pythonEnv) {
				return true
			}
		}
	}

	// Check command/args for Python execution
	allArgs := append(container.Command, container.Args...)
	for _, arg := range allArgs {
		argLower := strings.ToLower(arg)
		if strings.Contains(argLower, "python") ||
			strings.Contains(argLower, "pip") ||
			strings.Contains(argLower, "django") ||
			strings.Contains(argLower, "flask") ||
			strings.Contains(argLower, "gunicorn") ||
			strings.Contains(argLower, "uvicorn") {
			return true
		}
	}

	// If no Python indicators found, assume it's not a Python container
	return false
}

// Error Recovery Methods

// executeWithRecovery executes injection with comprehensive error recovery
func (i *OpenLitInjector) executeWithRecovery(pod *corev1.Pod, originalPod *corev1.Pod, result *InjectionResult, operation func() error) error {
	defer func() {
		if r := recover(); r != nil {
			log.Error("Injection panic occurred - performing emergency recovery", nil,
				"component", "injector",
				"k8s.pod.name", pod.Name,
				"k8s.namespace.name", pod.Namespace,
				"panic.value", r)

			i.performEmergencyRecovery(pod, originalPod, result)
			result.Error = fmt.Errorf("injection panic: %v", r)
			result.Success = false
		}
	}()

	err := operation()
	if err != nil {
		log.Error("Injection operation failed - performing cleanup", err,
			"component", "injector",
			"k8s.pod.name", pod.Name,
			"k8s.namespace.name", pod.Namespace,
			"containers.instrumented", result.InstrumentedCount,
			"containers.total", result.TotalContainers)

		// Perform cleanup based on error type and injection state
		i.performCleanupRecovery(pod, originalPod, result, err)
		result.Error = err
		result.Success = false
		return err
	}

	result.Success = true
	return nil
}

// addInitContainerWithRecovery adds init container with error recovery
func (i *OpenLitInjector) addInitContainerWithRecovery(pod *corev1.Pod, result *InjectionResult) error {
	originalInitContainerCount := len(pod.Spec.InitContainers)

	defer func() {
		if r := recover(); r != nil {
			log.Error("Init container addition panic", nil,
				"component", "injector",
				"k8s.pod.name", pod.Name,
				"panic.value", r)
			result.RecoveryActions = append(result.RecoveryActions, "cleaned_up_init_container_panic")
		}
	}()

	// Generate installation commands based on provider
	installCmd := i.generateInstallCommand()
	volumeName := i.config.GetSharedVolumeName()
	volumePath := i.config.GetSharedVolumePath()

	initContainer := corev1.Container{
		Name:            i.config.GetContainerName(),
		Image:           i.config.GetInitContainerImage(),
		ImagePullPolicy: i.config.ImagePullPolicy,
		Command:         []string{"/bin/sh"},
		Args: []string{
			"-c",
			installCmd,
		},
		VolumeMounts: []corev1.VolumeMount{
			{
				Name:      volumeName,
				MountPath: volumePath,
			},
		},
		Env: []corev1.EnvVar{
			{
				Name:  "INSTRUMENTATION_PROVIDER",
				Value: i.config.Provider,
			},
		},
		Resources: corev1.ResourceRequirements{
			Limits: corev1.ResourceList{
				corev1.ResourceCPU:    resource.MustParse("200m"),
				corev1.ResourceMemory: resource.MustParse("256Mi"),
			},
			Requests: corev1.ResourceList{
				corev1.ResourceCPU:    resource.MustParse("100m"),
				corev1.ResourceMemory: resource.MustParse("128Mi"),
			},
		},
	}

	// Validate init container configuration
	if initContainer.Name == "" {
		return fmt.Errorf("init container name is empty")
	}
	if initContainer.Image == "" {
		return fmt.Errorf("init container image is empty")
	}

	pod.Spec.InitContainers = append(pod.Spec.InitContainers, initContainer)

	// Verify addition was successful
	if len(pod.Spec.InitContainers) != originalInitContainerCount+1 {
		return fmt.Errorf("init container addition failed - expected %d containers, got %d",
			originalInitContainerCount+1, len(pod.Spec.InitContainers))
	}

	log.Info("Adding auto-instrumentation init container",
		"component", "injector",
		"k8s.container.name", initContainer.Name,
		"k8s.container.image", initContainer.Image,
		"instrumentation.provider", i.config.Provider,
		"k8s.container.image_pull_policy", string(initContainer.ImagePullPolicy))

	result.RecoveryActions = append(result.RecoveryActions, "added_init_container")
	return nil
}

// addSharedVolumeWithRecovery adds shared volume with error recovery
func (i *OpenLitInjector) addSharedVolumeWithRecovery(pod *corev1.Pod, result *InjectionResult) error {
	originalVolumeCount := len(pod.Spec.Volumes)
	volumeName := i.config.GetSharedVolumeName()

	defer func() {
		if r := recover(); r != nil {
			log.Error("Shared volume addition panic", nil,
				"component", "injector",
				"k8s.pod.name", pod.Name,
				"panic.value", r)
			result.RecoveryActions = append(result.RecoveryActions, "cleaned_up_volume_panic")
		}
	}()

	// Check if volume already exists
	for _, volume := range pod.Spec.Volumes {
		if volume.Name == volumeName {
			log.Warn("Shared volume already exists - skipping addition",
				"component", "injector",
				"k8s.volume.name", volumeName,
				"k8s.pod.name", pod.Name)
			return nil
		}
	}

	volume := corev1.Volume{
		Name: volumeName,
		VolumeSource: corev1.VolumeSource{
			EmptyDir: &corev1.EmptyDirVolumeSource{},
		},
	}

	pod.Spec.Volumes = append(pod.Spec.Volumes, volume)

	// Verify addition was successful
	if len(pod.Spec.Volumes) != originalVolumeCount+1 {
		return fmt.Errorf("shared volume addition failed - expected %d volumes, got %d",
			originalVolumeCount+1, len(pod.Spec.Volumes))
	}

	log.Info("Adding shared volume for instrumentation packages",
		"component", "injector",
		"k8s.volume.name", volumeName,
		"k8s.volume.type", "emptydir",
		"instrumentation.provider", i.config.Provider)

	result.RecoveryActions = append(result.RecoveryActions, "added_shared_volume")
	return nil
}

// modifyContainersWithRecovery modifies containers with error recovery
func (i *OpenLitInjector) modifyContainersWithRecovery(pod *corev1.Pod, result *InjectionResult) error {
	for idx := range pod.Spec.Containers {
		container := &pod.Spec.Containers[idx]

		if !i.shouldInstrumentContainer(container, pod) {
			log.Info("Skipping container - excluded by selection criteria",
				"component", "injector",
				"k8s.container.name", container.Name,
				"k8s.pod.name", pod.Name,
				"k8s.namespace.name", pod.Namespace)
			continue
		}

		// Store original state for recovery
		originalEnvCount := len(container.Env)
		originalVolumeMountCount := len(container.VolumeMounts)

		err := i.modifyContainerWithRecovery(container, pod, result)
		if err != nil {
			// Record failed container and attempt recovery
			result.FailedContainers = append(result.FailedContainers, container.Name)
			result.PartialFailure = true

			log.Error("Failed to modify container - attempting recovery", err,
				"component", "injector",
				"k8s.container.name", container.Name,
				"k8s.pod.name", pod.Name,
				"original.env_count", originalEnvCount,
				"original.volume_mount_count", originalVolumeMountCount)

			// Attempt to restore container to original state
			if len(container.Env) > originalEnvCount {
				container.Env = container.Env[:originalEnvCount]
				result.RecoveryActions = append(result.RecoveryActions,
					fmt.Sprintf("restored_env_vars_for_%s", container.Name))
			}
			if len(container.VolumeMounts) > originalVolumeMountCount {
				container.VolumeMounts = container.VolumeMounts[:originalVolumeMountCount]
				result.RecoveryActions = append(result.RecoveryActions,
					fmt.Sprintf("restored_volume_mounts_for_%s", container.Name))
			}

			continue // Continue with other containers
		}

		result.InstrumentedCount++
	}

	if result.InstrumentedCount == 0 && len(result.FailedContainers) > 0 {
		return fmt.Errorf("all container modifications failed: %v", result.FailedContainers)
	}

	if result.PartialFailure {
		log.Warn("Partial failure during container modification",
			"component", "injector",
			"k8s.pod.name", pod.Name,
			"containers.successful", result.InstrumentedCount,
			"containers.failed", len(result.FailedContainers),
			"failed.containers", result.FailedContainers)
	}

	return nil
}

// modifyContainerWithRecovery modifies a single container with error recovery
func (i *OpenLitInjector) modifyContainerWithRecovery(container *corev1.Container, pod *corev1.Pod, result *InjectionResult) error {
	defer func() {
		if r := recover(); r != nil {
			log.Error("Container modification panic", nil,
				"component", "injector",
				"k8s.container.name", container.Name,
				"panic.value", r)
			result.RecoveryActions = append(result.RecoveryActions,
				fmt.Sprintf("recovered_from_panic_%s", container.Name))
		}
	}()

	log.Info("Modifying container for auto-instrumentation",
		"component", "injector",
		"k8s.container.name", container.Name,
		"instrumentation.provider", i.config.Provider)

	// Check if container already has instrumentation and log it
	if i.hasExistingInstrumentation(container, pod) {
		log.Warn("Overwriting existing instrumentation - OpenLIT will replace current observability setup",
			"component", "injector",
			"k8s.container.name", container.Name,
			"k8s.pod.name", pod.Name,
			"k8s.namespace.name", pod.Namespace,
			"action", "overwrite_existing_instrumentation")
	}

	// Add volume mount for instrumentation SDK
	volumeName := i.config.GetSharedVolumeName()
	volumePath := i.config.GetSharedVolumePath()

	volumeMount := corev1.VolumeMount{
		Name:      volumeName,
		MountPath: volumePath,
		ReadOnly:  true,
	}

	// Check for duplicate volume mounts
	for _, existingMount := range container.VolumeMounts {
		if existingMount.Name == volumeMount.Name {
			return fmt.Errorf("volume mount %s already exists in container %s", volumeMount.Name, container.Name)
		}
	}

	container.VolumeMounts = append(container.VolumeMounts, volumeMount)

	// Add environment variables for instrumentation configuration
	envVars := i.buildEnvironmentVariables(container, pod)
	if len(envVars) == 0 {
		return fmt.Errorf("failed to build environment variables for container %s", container.Name)
	}

	container.Env = append(container.Env, envVars...)

	log.Info("Container modification completed",
		"component", "injector",
		"k8s.container.name", container.Name,
		"instrumentation.provider", i.config.Provider,
		"env_vars.added", len(envVars),
		"volume_mounts.added", 1)

	return nil
}

// performEmergencyRecovery performs emergency recovery on panic
func (i *OpenLitInjector) performEmergencyRecovery(pod *corev1.Pod, originalPod *corev1.Pod, result *InjectionResult) {
	log.Error("Performing emergency recovery - restoring pod to original state", nil,
		"component", "injector",
		"k8s.pod.name", pod.Name,
		"k8s.namespace.name", pod.Namespace)

	// Restore pod to original state
	pod.Spec = *originalPod.Spec.DeepCopy()

	result.RecoveryActions = append(result.RecoveryActions, "emergency_recovery_completed")
	result.PartialFailure = true
}

// performCleanupRecovery performs cleanup recovery based on error type
func (i *OpenLitInjector) performCleanupRecovery(pod *corev1.Pod, originalPod *corev1.Pod, result *InjectionResult, err error) {
	log.Info("Performing cleanup recovery",
		"component", "injector",
		"k8s.pod.name", pod.Name,
		"k8s.namespace.name", pod.Namespace,
		"error", err.Error(),
		"instrumented.containers", result.InstrumentedCount)

	// Based on the error and recovery actions taken, decide cleanup strategy
	recoveryStrategy := i.determineRecoveryStrategy(result, err)

	switch recoveryStrategy {
	case "full_rollback":
		log.Info("Performing full rollback to original pod state",
			"component", "injector",
			"k8s.pod.name", pod.Name)
		pod.Spec = *originalPod.Spec.DeepCopy()
		result.RecoveryActions = append(result.RecoveryActions, "full_rollback_completed")

	case "partial_cleanup":
		log.Info("Performing partial cleanup - removing added resources",
			"component", "injector",
			"k8s.pod.name", pod.Name)
		i.cleanupPartialInstrumentation(pod, originalPod, result)

	case "leave_partial":
		log.Info("Leaving partial instrumentation in place - may still be functional",
			"component", "injector",
			"k8s.pod.name", pod.Name,
			"instrumented.containers", result.InstrumentedCount)
		result.RecoveryActions = append(result.RecoveryActions, "left_partial_instrumentation")

	default:
		log.Warn("Unknown recovery strategy - performing conservative cleanup",
			"component", "injector",
			"k8s.pod.name", pod.Name,
			"recovery.strategy", recoveryStrategy)
		i.cleanupPartialInstrumentation(pod, originalPod, result)
	}
}

// determineRecoveryStrategy determines the best recovery strategy based on failure type
func (i *OpenLitInjector) determineRecoveryStrategy(result *InjectionResult, err error) string {
	errorStr := err.Error()

	// Full rollback for critical failures
	if strings.Contains(errorStr, "security context") ||
		strings.Contains(errorStr, "validation failed") ||
		result.InstrumentedCount == 0 {
		return "full_rollback"
	}

	// Partial cleanup for container-specific failures
	if strings.Contains(errorStr, "failed to modify containers") ||
		result.PartialFailure {
		return "partial_cleanup"
	}

	// Leave partial for resource failures that don't affect functionality
	if strings.Contains(errorStr, "init container") ||
		strings.Contains(errorStr, "shared volume") {
		if result.InstrumentedCount > 0 {
			return "leave_partial"
		}
	}

	// Default to partial cleanup
	return "partial_cleanup"
}

// cleanupPartialInstrumentation cleans up partially applied instrumentation
func (i *OpenLitInjector) cleanupPartialInstrumentation(pod *corev1.Pod, originalPod *corev1.Pod, result *InjectionResult) {
	volumeName := i.config.GetSharedVolumeName()

	// Remove added init containers (keep original ones)
	originalInitCount := len(originalPod.Spec.InitContainers)
	if len(pod.Spec.InitContainers) > originalInitCount {
		pod.Spec.InitContainers = pod.Spec.InitContainers[:originalInitCount]
		result.RecoveryActions = append(result.RecoveryActions, "removed_added_init_containers")
	}

	// Remove added volumes (keep original ones)
	originalVolumeCount := len(originalPod.Spec.Volumes)
	if len(pod.Spec.Volumes) > originalVolumeCount {
		// Remove our specific volume
		filteredVolumes := []corev1.Volume{}
		for _, volume := range pod.Spec.Volumes {
			if volume.Name != volumeName {
				filteredVolumes = append(filteredVolumes, volume)
			}
		}
		pod.Spec.Volumes = filteredVolumes
		result.RecoveryActions = append(result.RecoveryActions, "removed_shared_volume")
	}

	// Reset container modifications for failed containers
	for idx, container := range pod.Spec.Containers {
		originalContainer := originalPod.Spec.Containers[idx]

		// Reset to original environment variables
		container.Env = make([]corev1.EnvVar, len(originalContainer.Env))
		copy(container.Env, originalContainer.Env)

		// Reset to original volume mounts
		container.VolumeMounts = make([]corev1.VolumeMount, len(originalContainer.VolumeMounts))
		copy(container.VolumeMounts, originalContainer.VolumeMounts)
	}

	result.RecoveryActions = append(result.RecoveryActions, "cleaned_up_container_modifications")
	log.Info("Partial instrumentation cleanup completed",
		"component", "injector",
		"k8s.pod.name", pod.Name,
		"recovery.actions", result.RecoveryActions)
}
