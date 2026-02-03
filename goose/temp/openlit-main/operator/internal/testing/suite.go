package testing

import (
	"context"
	"path/filepath"
	"testing"
	"time"

	. "github.com/onsi/ginkgo/v2"
	. "github.com/onsi/gomega"
	"k8s.io/client-go/kubernetes/scheme"
	"k8s.io/client-go/rest"
	"sigs.k8s.io/controller-runtime/pkg/client"
	"sigs.k8s.io/controller-runtime/pkg/envtest"
	logf "sigs.k8s.io/controller-runtime/pkg/log"
	"sigs.k8s.io/controller-runtime/pkg/log/zap"

	autoinstrumentationv1alpha1 "github.com/openlit/openlit/operator/api/v1alpha1"
	"github.com/openlit/openlit/operator/internal/observability"
)

var (
	cfg       *rest.Config
	k8sClient client.Client
	testEnv   *envtest.Environment
	ctx       context.Context
	cancel    context.CancelFunc
)

// TestSuite provides a shared test environment for all operator tests
type TestSuite struct {
	Client     client.Client
	Cfg        *rest.Config
	Ctx        context.Context
	Cancel     context.CancelFunc
}

// SetupTestSuite initializes the test environment with a real Kubernetes API server
func SetupTestSuite() *TestSuite {
	logf.SetLogger(zap.New(zap.WriteTo(GinkgoWriter), zap.UseDevMode(true)))

	ctx, cancel = context.WithCancel(context.TODO())

	By("bootstrapping test environment")
	testEnv = &envtest.Environment{
		CRDDirectoryPaths: []string{
			filepath.Join("..", "..", "deploy"),
		},
		ErrorIfCRDPathMissing: true,
	}

	var err error
	cfg, err = testEnv.Start()
	Expect(err).NotTo(HaveOccurred())
	Expect(cfg).NotTo(BeNil())

	err = autoinstrumentationv1alpha1.AddToScheme(scheme.Scheme)
	Expect(err).NotTo(HaveOccurred())

	k8sClient, err = client.New(cfg, client.Options{Scheme: scheme.Scheme})
	Expect(err).NotTo(HaveOccurred())
	Expect(k8sClient).NotTo(BeNil())

	return &TestSuite{
		Client: k8sClient,
		Cfg:    cfg,
		Ctx:    ctx,
		Cancel: cancel,
	}
}

// TeardownTestSuite cleans up the test environment
func TeardownTestSuite() {
	By("tearing down the test environment")
	cancel()
	err := testEnv.Stop()
	Expect(err).NotTo(HaveOccurred())
}

// MockLoggerProvider creates a mock logger provider for testing
func MockLoggerProvider() *observability.LoggerProvider {
	return &observability.LoggerProvider{
		OTLPEnabled:   false,
		OTLPEndpoint:  "",
		ErrorMessage:  "",
	}
}

// WaitForCondition waits for a condition to be true with timeout
func WaitForCondition(condition func() bool, timeout time.Duration, interval time.Duration) bool {
	timeoutTimer := time.After(timeout)
	ticker := time.NewTicker(interval)
	defer ticker.Stop()

	for {
		select {
		case <-timeoutTimer:
			return false
		case <-ticker.C:
			if condition() {
				return true
			}
		}
	}
}

// RunAllTests runs all operator test suites
func RunAllTests(t *testing.T) {
	RegisterFailHandler(Fail)
	RunSpecs(t, "OpenLIT Operator Test Suite")
}
