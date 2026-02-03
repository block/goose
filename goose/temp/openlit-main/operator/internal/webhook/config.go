package webhook

import (
	"context"
	"fmt"

	admissionregistrationv1 "k8s.io/api/admissionregistration/v1"
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	"k8s.io/client-go/kubernetes"

	"github.com/openlit/openlit/operator/internal/observability"
)

// WebhookConfigManager manages the MutatingWebhookConfiguration
// Following the Velotio approach for automatic webhook registration
type WebhookConfigManager struct {
	client             kubernetes.Interface
	namespace          string
	serviceName        string
	configName         string
	webhookPath        string
	webhookPort        int32
	failurePolicy      string
	reinvocationPolicy string
	logger             *observability.StructuredLogger
}

// NewWebhookConfigManager creates a new webhook configuration manager
func NewWebhookConfigManager(client kubernetes.Interface, namespace, serviceName, configName, webhookPath string, webhookPort int, failurePolicy, reinvocationPolicy string, logger *observability.StructuredLogger) *WebhookConfigManager {
	return &WebhookConfigManager{
		client:             client,
		namespace:          namespace,
		serviceName:        serviceName,
		configName:         configName,
		webhookPath:        webhookPath,
		webhookPort:        int32(webhookPort),
		failurePolicy:      failurePolicy,
		reinvocationPolicy: reinvocationPolicy,
		logger:             logger,
	}
}

// EnsureWebhookConfiguration creates or updates the MutatingWebhookConfiguration
func (wc *WebhookConfigManager) EnsureWebhookConfiguration(ctx context.Context, caCert []byte) error {
	wc.logger.Info("🔗 Ensuring webhook configuration",
		"component", "webhook-config-manager",
		"k8s.mutatingwebhookconfiguration.name", wc.configName,
		"k8s.namespace.name", wc.namespace,
		"k8s.service.name", wc.serviceName,
		"webhook.path", wc.webhookPath,
		"webhook.port", wc.webhookPort,
		"webhook.failure_policy", wc.failurePolicy,
		"webhook.reinvocation_policy", wc.reinvocationPolicy)

	// Use CA certificate bytes directly (already PEM encoded)
	caBundle := caCert

	// Define webhook configuration
	webhookConfig := &admissionregistrationv1.MutatingWebhookConfiguration{
		ObjectMeta: metav1.ObjectMeta{
			Name: wc.configName,
			Labels: map[string]string{
				"app":       "openlit-operator",
				"component": "webhook-config",
			},
		},
		Webhooks: []admissionregistrationv1.MutatingWebhook{
			{
				Name: "pod-instrumentation.openlit.io",
				ClientConfig: admissionregistrationv1.WebhookClientConfig{
					Service: &admissionregistrationv1.ServiceReference{
						Name:      wc.serviceName,
						Namespace: wc.namespace,
						Path:      stringPtr(wc.webhookPath),
						Port:      int32Ptr(443), // Use service port (443), not target port
					},
					CABundle: []byte(caBundle),
				},
				Rules: []admissionregistrationv1.RuleWithOperations{
					{
						Operations: []admissionregistrationv1.OperationType{
							admissionregistrationv1.Create,
						},
						Rule: admissionregistrationv1.Rule{
							APIGroups:   []string{""},
							APIVersions: []string{"v1"},
							Resources:   []string{"pods"},
						},
					},
				},
				// CRITICAL: Exclude operator's own pods to prevent chicken-and-egg problem
				ObjectSelector: &metav1.LabelSelector{
					MatchExpressions: []metav1.LabelSelectorRequirement{
						{
							Key:      "app",
							Operator: metav1.LabelSelectorOpNotIn,
							Values:   []string{"openlit-operator"}, // Exclude operator pods
						},
					},
				},
				AdmissionReviewVersions: []string{"v1", "v1beta1"},
				SideEffects:             sideEffectsPtr(admissionregistrationv1.SideEffectClassNone),
				FailurePolicy:           wc.getFailurePolicy(),
				ReinvocationPolicy:      wc.getReinvocationPolicy(),
			},
		},
	}

	// Check if webhook configuration exists
	existingConfig, err := wc.client.AdmissionregistrationV1().MutatingWebhookConfigurations().Get(ctx, wc.configName, metav1.GetOptions{})
	if err != nil {
		// Configuration doesn't exist, create it
		wc.logger.Info("📝 Creating new webhook configuration",
			"component", "webhook-config-manager",
			"k8s.mutatingwebhookconfiguration.name", wc.configName)
		_, err = wc.client.AdmissionregistrationV1().MutatingWebhookConfigurations().Create(ctx, webhookConfig, metav1.CreateOptions{})
		if err != nil {
			return fmt.Errorf("failed to create webhook configuration: %w", err)
		}
	} else {
		// Configuration exists, update it with preserved metadata
		wc.logger.Info("🔄 Updating existing webhook configuration",
			"component", "webhook-config-manager",
			"k8s.mutatingwebhookconfiguration.name", wc.configName,
			"k8s.mutatingwebhookconfiguration.resource_version", existingConfig.ObjectMeta.ResourceVersion)
		webhookConfig.ObjectMeta.ResourceVersion = existingConfig.ObjectMeta.ResourceVersion
		webhookConfig.ObjectMeta.UID = existingConfig.ObjectMeta.UID
		_, err = wc.client.AdmissionregistrationV1().MutatingWebhookConfigurations().Update(ctx, webhookConfig, metav1.UpdateOptions{})
		if err != nil {
			return fmt.Errorf("failed to update webhook configuration: %w", err)
		}
	}

	wc.logger.Info("✅ Webhook configuration updated successfully",
		"component", "webhook-config-manager",
		"k8s.mutatingwebhookconfiguration.name", wc.configName)
	return nil
}

// CleanupWebhookConfiguration removes the webhook configuration (for cleanup)
func (wc *WebhookConfigManager) CleanupWebhookConfiguration(ctx context.Context) error {
	wc.logger.Info("🧹 Cleaning up webhook configuration",
		"component", "webhook-config-manager",
		"k8s.mutatingwebhookconfiguration.name", wc.configName)

	err := wc.client.AdmissionregistrationV1().MutatingWebhookConfigurations().Delete(ctx, wc.configName, metav1.DeleteOptions{})
	if err != nil {
		return fmt.Errorf("failed to delete webhook configuration: %w", err)
	}

	wc.logger.Info("✅ Webhook configuration cleaned up",
		"component", "webhook-config-manager",
		"k8s.mutatingwebhookconfiguration.name", wc.configName)
	return nil
}

// Helper functions for pointer types
func stringPtr(s string) *string {
	return &s
}

func int32Ptr(i int32) *int32 {
	return &i
}

func sideEffectsPtr(s admissionregistrationv1.SideEffectClass) *admissionregistrationv1.SideEffectClass {
	return &s
}

func failurePolicyPtr(f admissionregistrationv1.FailurePolicyType) *admissionregistrationv1.FailurePolicyType {
	return &f
}

func reinvocationPolicyPtr(r admissionregistrationv1.ReinvocationPolicyType) *admissionregistrationv1.ReinvocationPolicyType {
	return &r
}

// getFailurePolicy converts string to FailurePolicyType
func (wc *WebhookConfigManager) getFailurePolicy() *admissionregistrationv1.FailurePolicyType {
	switch wc.failurePolicy {
	case "Fail":
		return failurePolicyPtr(admissionregistrationv1.Fail)
	case "Ignore":
		return failurePolicyPtr(admissionregistrationv1.Ignore)
	default:
		// Default to Ignore for safety
		return failurePolicyPtr(admissionregistrationv1.Ignore)
	}
}

// getReinvocationPolicy converts string to ReinvocationPolicyType
func (wc *WebhookConfigManager) getReinvocationPolicy() *admissionregistrationv1.ReinvocationPolicyType {
	switch wc.reinvocationPolicy {
	case "Never":
		return reinvocationPolicyPtr(admissionregistrationv1.NeverReinvocationPolicy)
	case "IfNeeded":
		return reinvocationPolicyPtr(admissionregistrationv1.IfNeededReinvocationPolicy)
	default:
		// Default to Never for safety
		return reinvocationPolicyPtr(admissionregistrationv1.NeverReinvocationPolicy)
	}
}
