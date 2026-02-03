/*
OpenLIT AutoInstrumentation Controller

This controller manages the lifecycle of AutoInstrumentation Custom Resources,
providing validation, status updates, and management labeling for zero-code
instrumentation configurations in Kubernetes clusters.

Key responsibilities:
- Validates AutoInstrumentation CR configurations against the central schema
- Updates CR status with validation results and processing information
- Adds "managed-by" labels to track operator ownership
- Provides comprehensive observability through structured logging
- Handles CR reconciliation and error recovery

The controller works in conjunction with the admission webhook to provide
a complete zero-code instrumentation solution, where the webhook handles
real-time pod injection and the controller manages the configuration
lifecycle and provides operational visibility.

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

    http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.
*/

package controller

import (
	"context"
	"fmt"
	"io/ioutil"
	"os"
	"strings"
	"time"

	"k8s.io/apimachinery/pkg/api/errors"
	"k8s.io/apimachinery/pkg/api/meta"
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	"k8s.io/apimachinery/pkg/runtime"
	ctrl "sigs.k8s.io/controller-runtime"
	"sigs.k8s.io/controller-runtime/pkg/client"

	"github.com/openlit/openlit/operator/api/v1alpha1"
	"github.com/openlit/openlit/operator/internal/observability"
	"github.com/openlit/openlit/operator/internal/validation"
)

// AutoInstrumentationReconciler reconciles an AutoInstrumentation object
type AutoInstrumentationReconciler struct {
	client.Client
	Scheme    *runtime.Scheme
	logger    *observability.StructuredLogger
	validator *validation.AutoInstrumentationValidator
}

// NewAutoInstrumentationReconciler creates a new reconciler
func NewAutoInstrumentationReconciler(client client.Client, scheme *runtime.Scheme) *AutoInstrumentationReconciler {
	return &AutoInstrumentationReconciler{
		Client:    client,
		Scheme:    scheme,
		logger:    observability.NewLogger("instrumentationconfig-controller"),
		validator: validation.NewAutoInstrumentationValidator(),
	}
}

//+kubebuilder:rbac:groups=openlit.io,resources=instrumentationconfigs,verbs=get;list;watch;create;update;patch;delete
//+kubebuilder:rbac:groups=openlit.io,resources=instrumentationconfigs/status,verbs=get;update;patch
//+kubebuilder:rbac:groups=openlit.io,resources=instrumentationconfigs/finalizers,verbs=update

// Reconcile handles AutoInstrumentation resources
func (r *AutoInstrumentationReconciler) Reconcile(ctx context.Context, req ctrl.Request) (ctrl.Result, error) {
	// Get the AutoInstrumentation
	var config v1alpha1.AutoInstrumentation
	if err := r.Get(ctx, req.NamespacedName, &config); err != nil {
		if errors.IsNotFound(err) {
			r.logger.Info("🗑️ AutoInstrumentation deleted",
				"component", "controller",
				"config.name", req.Name,
				"config.namespace", req.Namespace,
				"k8s.namespace.name", req.Namespace,
				"k8s.object.name", req.Name)
			return ctrl.Result{}, nil
		}
		r.logger.Error("Failed to get AutoInstrumentation", err,
			"component", "controller",
			"config.name", req.Name,
			"config.namespace", req.Namespace,
			"k8s.namespace.name", req.Namespace,
			"k8s.object.name", req.Name)
		return ctrl.Result{}, err
	}

	r.logger.Info("Reconciling AutoInstrumentation",
		"component", "controller",
		"config.name", config.Name,
		"config.namespace", config.Namespace,
		"config.generation", config.Generation,
		"k8s.namespace.name", config.Namespace,
		"k8s.object.name", config.Name)

	// Add management labels if they don't exist
	if err := r.ensureManagementLabels(ctx, &config); err != nil {
		r.logger.Error("Failed to ensure management labels", err,
			"component", "controller",
			"config.name", config.Name,
			"config.namespace", config.Namespace,
			"k8s.namespace.name", config.Namespace,
			"k8s.object.name", config.Name)
		return ctrl.Result{RequeueAfter: time.Second * 30}, err
	}

	// Re-fetch the resource to get the latest version after label updates
	if err := r.Get(ctx, req.NamespacedName, &config); err != nil {
		r.logger.Error("Failed to re-fetch AutoInstrumentation after label update", err,
			"component", "controller",
			"config.name", config.Name,
			"config.namespace", config.Namespace)
		return ctrl.Result{RequeueAfter: time.Second * 30}, err
	}

	// Validate the configuration
	validationResult := r.validator.Validate(&config)

	// Update status with validation results
	if err := r.updateStatus(ctx, &config, validationResult); err != nil {
		r.logger.Error("Failed to update status", err,
			"component", "controller",
			"config.name", config.Name,
			"config.namespace", config.Namespace,
			"k8s.namespace.name", config.Namespace,
			"k8s.object.name", config.Name)
		return ctrl.Result{RequeueAfter: time.Second * 30}, err
	}

	// If validation failed, don't proceed but don't error
	if !validationResult.Valid {
		r.logger.Warn("⚠️ AutoInstrumentation validation failed, will retry",
			"component", "controller",
			"config.name", config.Name,
			"config.namespace", config.Namespace,
			"validation.errors.count", len(validationResult.Errors),
			"k8s.namespace.name", config.Namespace,
			"k8s.object.name", config.Name)
		return ctrl.Result{RequeueAfter: time.Minute * 5}, nil
	}

	r.logger.Info("AutoInstrumentation reconciled successfully",
		"component", "controller",
		"config.name", config.Name,
		"config.namespace", config.Namespace,
		"validation.warnings.count", len(validationResult.Warnings),
		"k8s.namespace.name", config.Namespace,
		"k8s.object.name", config.Name)

	return ctrl.Result{}, nil
}

// ensureManagementLabels adds management labels to the AutoInstrumentation
func (r *AutoInstrumentationReconciler) ensureManagementLabels(ctx context.Context, config *v1alpha1.AutoInstrumentation) error {
	// Get operator identity - try to get actual values from Kubernetes
	operatorName := getOperatorPodName()
	operatorNamespace := getNamespaceFromServiceAccount()

	// Check if management labels already exist
	labels := config.GetLabels()
	if labels == nil {
		labels = make(map[string]string)
	}

	managedByLabel := "openlit.io/managed-by"
	instrumentedByLabel := "openlit.io/instrumented-by"
	expectedManagedBy := fmt.Sprintf("%s.%s", operatorName, operatorNamespace)

	needsUpdate := false

	// Check managed-by label
	if existingManagedBy, exists := labels[managedByLabel]; !exists || existingManagedBy != expectedManagedBy {
		labels[managedByLabel] = expectedManagedBy
		needsUpdate = true
		r.logger.Info("Adding managed-by label",
			"component", "controller",
			"config.name", config.Name,
			"config.namespace", config.Namespace,
			"label.key", managedByLabel,
			"label.value", expectedManagedBy,
			"k8s.namespace.name", config.Namespace,
			"k8s.object.name", config.Name)
	}

	// Check instrumented-by label
	if existingInstrumentedBy, exists := labels[instrumentedByLabel]; !exists || existingInstrumentedBy != operatorName {
		labels[instrumentedByLabel] = operatorName
		needsUpdate = true
		r.logger.Info("Adding instrumented-by label",
			"component", "controller",
			"config.name", config.Name,
			"config.namespace", config.Namespace,
			"label.key", instrumentedByLabel,
			"label.value", operatorName,
			"k8s.namespace.name", config.Namespace,
			"k8s.object.name", config.Name)
	}

	// Update labels if needed
	if needsUpdate {
		config.SetLabels(labels)
		if err := r.Update(ctx, config); err != nil {
			return fmt.Errorf("failed to update AutoInstrumentation labels: %w", err)
		}
		r.logger.Info("Management labels updated successfully",
			"component", "controller",
			"config.name", config.Name,
			"config.namespace", config.Namespace,
			"k8s.namespace.name", config.Namespace,
			"k8s.object.name", config.Name)
	}

	return nil
}

// updateStatus updates the AutoInstrumentation status
func (r *AutoInstrumentationReconciler) updateStatus(ctx context.Context, config *v1alpha1.AutoInstrumentation, validationResult *validation.ValidationResult) error {
	// Create a copy for status update
	configCopy := config.DeepCopy()

	// Update status fields
	now := metav1.Now()
	configCopy.Status.LastProcessed = &now
	configCopy.Status.ValidationErrors = validationResult.Errors

	// Update conditions
	readyCondition := metav1.Condition{
		Type:               "Ready",
		LastTransitionTime: now,
		ObservedGeneration: config.Generation,
	}

	if validationResult.Valid {
		readyCondition.Status = metav1.ConditionTrue
		readyCondition.Reason = "ValidationPassed"
		readyCondition.Message = "AutoInstrumentation is valid and ready"
		if len(validationResult.Warnings) > 0 {
			readyCondition.Message = fmt.Sprintf("AutoInstrumentation is valid with %d warnings", len(validationResult.Warnings))
		}
	} else {
		readyCondition.Status = metav1.ConditionFalse
		readyCondition.Reason = "ValidationFailed"
		readyCondition.Message = fmt.Sprintf("Validation failed with %d errors", len(validationResult.Errors))
	}

	// Update or add the Ready condition
	meta.SetStatusCondition(&configCopy.Status.Conditions, readyCondition)

	// Update status
	if err := r.Status().Update(ctx, configCopy); err != nil {
		return fmt.Errorf("failed to update AutoInstrumentation status: %w", err)
	}

	return nil
}

// SetupWithManager sets up the controller with the Manager
func (r *AutoInstrumentationReconciler) SetupWithManager(mgr ctrl.Manager) error {
	return ctrl.NewControllerManagedBy(mgr).
		For(&v1alpha1.AutoInstrumentation{}).
		Complete(r)
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
