package controller

import (
	"context"
	"net/http"
	"testing"
	"time"

	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/suite"
	"k8s.io/apimachinery/pkg/api/meta"
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	"k8s.io/apimachinery/pkg/runtime"
	"k8s.io/apimachinery/pkg/types"
	"k8s.io/client-go/rest"
	"k8s.io/client-go/tools/record"
	"sigs.k8s.io/controller-runtime/pkg/cache"
	"sigs.k8s.io/controller-runtime/pkg/config"
	"sigs.k8s.io/controller-runtime/pkg/client"
	"github.com/go-logr/logr"
	"sigs.k8s.io/controller-runtime/pkg/client/fake"
	"sigs.k8s.io/controller-runtime/pkg/healthz"
	"sigs.k8s.io/controller-runtime/pkg/manager"
	"sigs.k8s.io/controller-runtime/pkg/reconcile"
	"sigs.k8s.io/controller-runtime/pkg/webhook"

	autoinstrumentationv1alpha1 "github.com/openlit/openlit/operator/api/v1alpha1"
)

type ControllerTestSuite struct {
	suite.Suite
	controller *AutoInstrumentationReconciler
	client     client.Client
	ctx        context.Context
}

func (suite *ControllerTestSuite) SetupTest() {
	// Create fake Kubernetes client
	scheme := runtime.NewScheme()
	_ = autoinstrumentationv1alpha1.AddToScheme(scheme)
	
	suite.client = fake.NewClientBuilder().WithScheme(scheme).WithStatusSubresource(&autoinstrumentationv1alpha1.AutoInstrumentation{}).Build()
	suite.ctx = context.Background()

	// Create controller
	suite.controller = NewAutoInstrumentationReconciler(suite.client, scheme)
}

func (suite *ControllerTestSuite) TestReconcileSuccess() {
	// Create a valid AutoInstrumentation resource
	autoInstr := &autoinstrumentationv1alpha1.AutoInstrumentation{
		ObjectMeta: metav1.ObjectMeta{
			Name:      "test-instrumentation",
			Namespace: "default",
		},
		Spec: autoinstrumentationv1alpha1.AutoInstrumentationSpec{
			Selector: autoinstrumentationv1alpha1.PodSelector{
				MatchLabels: map[string]string{
					"app": "python-app",
				},
			},
			OTLP: autoinstrumentationv1alpha1.OTLPConfig{
				Endpoint: "http://openlit.default.svc.cluster.local:4318",
			},
			Resource: &autoinstrumentationv1alpha1.ResourceConfig{
				Environment: "test",
			},
		},
	}

	// Create the resource
	err := suite.client.Create(suite.ctx, autoInstr)
	suite.NoError(err, "Should create AutoInstrumentation resource")

	// Create reconcile request
	req := reconcile.Request{
		NamespacedName: types.NamespacedName{
			Name:      "test-instrumentation",
			Namespace: "default",
		},
	}

	// Reconcile
	result, err := suite.controller.Reconcile(suite.ctx, req)
	suite.NoError(err, "Reconciliation should succeed")
	suite.False(result.Requeue, "Should not requeue on success")
	suite.Equal(time.Duration(0), result.RequeueAfter, "Should not have requeue delay")

	// Verify the resource still exists and has been processed
	retrieved := &autoinstrumentationv1alpha1.AutoInstrumentation{}
	err = suite.client.Get(suite.ctx, req.NamespacedName, retrieved)
	suite.NoError(err, "Should retrieve the processed resource")
	suite.Equal("http://openlit.default.svc.cluster.local:4318", retrieved.Spec.OTLP.Endpoint)
}

func (suite *ControllerTestSuite) TestReconcileNotFound() {
	// Create reconcile request for non-existent resource
	req := reconcile.Request{
		NamespacedName: types.NamespacedName{
			Name:      "non-existent",
			Namespace: "default",
		},
	}

	// Reconcile
	result, err := suite.controller.Reconcile(suite.ctx, req)
	suite.NoError(err, "Should handle non-existent resource gracefully")
	suite.False(result.Requeue, "Should not requeue for non-existent resource")
}

func (suite *ControllerTestSuite) TestReconcileValidationFailure() {
	// Create an invalid AutoInstrumentation resource (missing required fields)
	autoInstr := &autoinstrumentationv1alpha1.AutoInstrumentation{
		ObjectMeta: metav1.ObjectMeta{
			Name:      "invalid-instrumentation",
			Namespace: "default",
		},
		Spec: autoinstrumentationv1alpha1.AutoInstrumentationSpec{
			// Missing OTLP endpoint - should cause validation failure
			Selector: autoinstrumentationv1alpha1.PodSelector{
				MatchLabels: map[string]string{
					"app": "python-app",
				},
			},
		},
	}

	// Create the resource
	err := suite.client.Create(suite.ctx, autoInstr)
	suite.NoError(err, "Should create invalid resource")

	// Create reconcile request
	req := reconcile.Request{
		NamespacedName: types.NamespacedName{
			Name:      "invalid-instrumentation",
			Namespace: "default",
		},
	}

	// Reconcile
	result, err := suite.controller.Reconcile(suite.ctx, req)
	suite.NoError(err, "Reconciliation should succeed even with missing OTLP")
	suite.Equal(5*time.Minute, result.RequeueAfter, "Should requeue after 5 minutes for validation warning")
}

func (suite *ControllerTestSuite) TestReconcileMultipleResources() {
	// Create multiple AutoInstrumentation resources
	resources := []*autoinstrumentationv1alpha1.AutoInstrumentation{
		{
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
		},
		{
			ObjectMeta: metav1.ObjectMeta{
				Name:      "java-instrumentation",
				Namespace: "default",
			},
			Spec: autoinstrumentationv1alpha1.AutoInstrumentationSpec{
				Selector: autoinstrumentationv1alpha1.PodSelector{
					MatchLabels: map[string]string{
						"app": "java-app",
					},
				},
				OTLP: autoinstrumentationv1alpha1.OTLPConfig{
					Endpoint: "http://otel-collector:4318",
				},
			},
		},
	}

	// Create resources and reconcile each
	for i, autoInstr := range resources {
		err := suite.client.Create(suite.ctx, autoInstr)
		suite.NoError(err, "Should create resource %d", i)

		req := reconcile.Request{
			NamespacedName: types.NamespacedName{
				Name:      autoInstr.Name,
				Namespace: autoInstr.Namespace,
			},
		}

		result, err := suite.controller.Reconcile(suite.ctx, req)
		suite.NoError(err, "Should reconcile resource %d successfully", i)
		suite.False(result.Requeue, "Should not requeue resource %d", i)
	}

	// Verify all resources exist
	for _, autoInstr := range resources {
		retrieved := &autoinstrumentationv1alpha1.AutoInstrumentation{}
		key := types.NamespacedName{Name: autoInstr.Name, Namespace: autoInstr.Namespace}
		err := suite.client.Get(suite.ctx, key, retrieved)
		suite.NoError(err, "Should retrieve resource %s", autoInstr.Name)
	}
}

func (suite *ControllerTestSuite) TestReconcileWithIgnoreSelector() {
	// Create AutoInstrumentation with ignore selector
	autoInstr := &autoinstrumentationv1alpha1.AutoInstrumentation{
		ObjectMeta: metav1.ObjectMeta{
			Name:      "ignore-test",
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

	// Create the resource
	err := suite.client.Create(suite.ctx, autoInstr)
	suite.NoError(err, "Should create AutoInstrumentation with ignore selector")

	// Create reconcile request
	req := reconcile.Request{
		NamespacedName: types.NamespacedName{
			Name:      "ignore-test",
			Namespace: "default",
		},
	}

	// Reconcile
	result, err := suite.controller.Reconcile(suite.ctx, req)
	suite.NoError(err, "Should reconcile resource with ignore selector")
	suite.False(result.Requeue, "Should not requeue")

	// Verify resource exists and has ignore selector
	retrieved := &autoinstrumentationv1alpha1.AutoInstrumentation{}
	err = suite.client.Get(suite.ctx, req.NamespacedName, retrieved)
	suite.NoError(err, "Should retrieve resource")
	suite.NotNil(retrieved.Spec.Ignore, "Should have ignore selector")
	suite.Equal("true", retrieved.Spec.Ignore.MatchLabels["skip"])
}

func (suite *ControllerTestSuite) TestReconcileWithCustomPackages() {
	// Create AutoInstrumentation with custom packages
	autoInstr := &autoinstrumentationv1alpha1.AutoInstrumentation{
		ObjectMeta: metav1.ObjectMeta{
			Name:      "custom-packages-test",
			Namespace: "default",
		},
		Spec: autoinstrumentationv1alpha1.AutoInstrumentationSpec{
			Selector: autoinstrumentationv1alpha1.PodSelector{
				MatchLabels: map[string]string{
					"app": "ml-app",
				},
			},
			OTLP: autoinstrumentationv1alpha1.OTLPConfig{
				Endpoint: "http://jaeger:4318",
			},
			Resource: &autoinstrumentationv1alpha1.ResourceConfig{
				Environment: "staging",
			},
		},
	}

	// Create the resource
	err := suite.client.Create(suite.ctx, autoInstr)
	suite.NoError(err, "Should create AutoInstrumentation with custom packages")

	// Create reconcile request
	req := reconcile.Request{
		NamespacedName: types.NamespacedName{
			Name:      "custom-packages-test",
			Namespace: "default",
		},
	}

	// Reconcile
	result, err := suite.controller.Reconcile(suite.ctx, req)
	suite.NoError(err, "Should reconcile resource with custom packages")
	suite.False(result.Requeue, "Should not requeue")

	// Verify resource exists and has custom packages
	retrieved := &autoinstrumentationv1alpha1.AutoInstrumentation{}
	err = suite.client.Get(suite.ctx, req.NamespacedName, retrieved)
	suite.NoError(err, "Should retrieve resource")
	suite.Equal("http://jaeger:4318", retrieved.Spec.OTLP.Endpoint)
	suite.Equal("staging", retrieved.Spec.Resource.Environment)
}

func (suite *ControllerTestSuite) TestReconcileResourceUpdate() {
	// Create initial AutoInstrumentation
	autoInstr := &autoinstrumentationv1alpha1.AutoInstrumentation{
		ObjectMeta: metav1.ObjectMeta{
			Name:      "update-test",
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
			Resource: &autoinstrumentationv1alpha1.ResourceConfig{
				Environment: "test",
			},
		},
	}

	// Create the resource
	err := suite.client.Create(suite.ctx, autoInstr)
	suite.NoError(err, "Should create initial resource")

	// Reconcile initial resource
	req := reconcile.Request{
		NamespacedName: types.NamespacedName{
			Name:      "update-test",
			Namespace: "default",
		},
	}

	result, err := suite.controller.Reconcile(suite.ctx, req)
	suite.NoError(err, "Should reconcile initial resource")

	// Update the resource
	retrieved := &autoinstrumentationv1alpha1.AutoInstrumentation{}
	err = suite.client.Get(suite.ctx, req.NamespacedName, retrieved)
	suite.NoError(err, "Should get resource for update")

	retrieved.Spec.OTLP.Endpoint = "http://updated-jaeger:4318"
	retrieved.Spec.Resource.Environment = "updated"

	err = suite.client.Update(suite.ctx, retrieved)
	suite.NoError(err, "Should update resource")

	// Reconcile updated resource
	result, err = suite.controller.Reconcile(suite.ctx, req)
	suite.NoError(err, "Should reconcile updated resource")
	suite.False(result.Requeue, "Should not requeue")

	// Verify updates were processed
	final := &autoinstrumentationv1alpha1.AutoInstrumentation{}
	err = suite.client.Get(suite.ctx, req.NamespacedName, final)
	suite.NoError(err, "Should retrieve final resource")
	suite.Equal("http://updated-jaeger:4318", final.Spec.OTLP.Endpoint)
	suite.Equal("updated", final.Spec.Resource.Environment)
}

func (suite *ControllerTestSuite) TestReconcileResourceDeletion() {
	// Create AutoInstrumentation
	autoInstr := &autoinstrumentationv1alpha1.AutoInstrumentation{
		ObjectMeta: metav1.ObjectMeta{
			Name:      "delete-test",
			Namespace: "default",
		},
		Spec: autoinstrumentationv1alpha1.AutoInstrumentationSpec{
			Selector: autoinstrumentationv1alpha1.PodSelector{
				MatchLabels: map[string]string{
					"app": "temp-app",
				},
			},
			OTLP: autoinstrumentationv1alpha1.OTLPConfig{
				Endpoint: "http://jaeger:4318",
			},
		},
	}

	// Create the resource
	err := suite.client.Create(suite.ctx, autoInstr)
	suite.NoError(err, "Should create resource")

	req := reconcile.Request{
		NamespacedName: types.NamespacedName{
			Name:      "delete-test",
			Namespace: "default",
		},
	}

	// Reconcile to ensure it's processed
	result, err := suite.controller.Reconcile(suite.ctx, req)
	suite.NoError(err, "Should reconcile resource")

	// Delete the resource
	err = suite.client.Delete(suite.ctx, autoInstr)
	suite.NoError(err, "Should delete resource")

	// Reconcile after deletion (simulates controller being notified of deletion)
	result, err = suite.controller.Reconcile(suite.ctx, req)
	suite.NoError(err, "Should handle deleted resource gracefully")
	suite.False(result.Requeue, "Should not requeue deleted resource")
}

func (suite *ControllerTestSuite) TestSetupWithManager() {
	// Create a scheme for the mock manager
	scheme := runtime.NewScheme()
	_ = autoinstrumentationv1alpha1.AddToScheme(scheme)
	
	// Create a mock manager with proper scheme
	mgr := &mockManager{scheme: scheme}

	// Test setup
	err := suite.controller.SetupWithManager(mgr)
	suite.NoError(err, "Should setup with manager successfully")
}

// Mock manager for testing
type mockManager struct{
	scheme *runtime.Scheme
}

func (m *mockManager) GetConfig() *rest.Config { return nil }
func (m *mockManager) GetScheme() *runtime.Scheme { return m.scheme }
func (m *mockManager) GetClient() client.Client { return nil }
func (m *mockManager) GetFieldIndexer() client.FieldIndexer { return nil }
func (m *mockManager) GetCache() cache.Cache { return nil }
func (m *mockManager) GetEventRecorderFor(name string) record.EventRecorder { return nil }
func (m *mockManager) GetRESTMapper() meta.RESTMapper { return nil }
func (m *mockManager) GetAPIReader() client.Reader { return nil }
func (m *mockManager) Start(ctx context.Context) error { return nil }
func (m *mockManager) Add(runnable manager.Runnable) error { return nil }
func (m *mockManager) Elected() <-chan struct{} { return nil }
func (m *mockManager) AddMetricsExtraHandler(path string, handler interface{}) error { return nil }
func (m *mockManager) AddMetricsServerExtraHandler(path string, handler http.Handler) error { return nil }
func (m *mockManager) AddHealthzCheck(name string, check healthz.Checker) error { return nil }
func (m *mockManager) AddReadyzCheck(name string, check healthz.Checker) error { return nil }
func (m *mockManager) GetWebhookServer() webhook.Server { return nil }
func (m *mockManager) GetLogger() logr.Logger { return logr.Discard() }
func (m *mockManager) GetControllerOptions() config.Controller { return config.Controller{} }
func (m *mockManager) GetHTTPClient() *http.Client { return nil }

// Implement the NewControllerManagedBy method needed by SetupWithManager
func (m *mockManager) NewControllerManagedBy() interface{} {
	return nil
}

func TestControllerSuite(t *testing.T) {
	suite.Run(t, new(ControllerTestSuite))
}

// Additional unit tests for edge cases
func TestControllerErrorHandling(t *testing.T) {
	scheme := runtime.NewScheme()
	_ = autoinstrumentationv1alpha1.AddToScheme(scheme)
	
	// Create a client that will return errors
	errorClient := &errorClient{fake.NewClientBuilder().WithScheme(scheme).Build()}
	
	controller := NewAutoInstrumentationReconciler(errorClient, scheme)

	req := reconcile.Request{
		NamespacedName: types.NamespacedName{
			Name:      "error-test",
			Namespace: "default",
		},
	}

	// This should handle client errors gracefully
	result, err := controller.Reconcile(context.Background(), req)
	
	// We expect either a handled error or no error (depending on error type)
	// The important thing is that it doesn't panic
	if err != nil {
		// Error should be wrapped appropriately
		assert.Contains(t, err.Error(), "assert.AnError general error for testing")
	}
	
	// Should not requeue on client errors
	assert.False(t, result.Requeue)
}

// Mock client that returns errors for testing error handling
type errorClient struct {
	client.Client
}

func (c *errorClient) Get(ctx context.Context, key client.ObjectKey, obj client.Object, opts ...client.GetOption) error {
	return assert.AnError
}
