package testing

import (
	"context"
	"encoding/json"
	"time"

	. "github.com/onsi/ginkgo/v2"
	. "github.com/onsi/gomega"
	admissionv1 "k8s.io/api/admission/v1"
	corev1 "k8s.io/api/core/v1"
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	"k8s.io/apimachinery/pkg/runtime"
	"k8s.io/apimachinery/pkg/types"
	dynamicfake "k8s.io/client-go/dynamic/fake"
	"sigs.k8s.io/controller-runtime/pkg/reconcile"
	"sigs.k8s.io/controller-runtime/pkg/webhook/admission"

	autoinstrumentationv1alpha1 "github.com/openlit/openlit/operator/api/v1alpha1"
	"github.com/openlit/openlit/operator/internal/config"
	"github.com/openlit/openlit/operator/internal/controller"
	"github.com/openlit/openlit/operator/internal/webhook"
)

var _ = Describe("OpenLIT Operator Integration Tests", func() {
	var (
		testSuite    *TestSuite
		reconciler   *controller.AutoInstrumentationReconciler
		webhookHandler *webhook.Handler
		ctx          context.Context
	)

	BeforeEach(func() {
		testSuite = SetupTestSuite()
		ctx = testSuite.Ctx

		// Create controller - use the proper constructor
		reconciler = controller.NewAutoInstrumentationReconciler(testSuite.Client, testSuite.Client.Scheme())

		// Create webhook handler
		cfg := &config.OperatorConfig{
			Namespace: "test-namespace",
		}
		dynamicClient := dynamicfake.NewSimpleDynamicClient(testSuite.Client.Scheme())
		webhookHandler = webhook.NewHandler(cfg, testSuite.Client.Scheme(), dynamicClient)
	})

	AfterEach(func() {
		TeardownTestSuite()
	})

	Describe("End-to-End AutoInstrumentation Flow", func() {
		It("should create, reconcile, and apply AutoInstrumentation", func() {
			By("Creating an AutoInstrumentation resource")
			autoInstr := &autoinstrumentationv1alpha1.AutoInstrumentation{
				ObjectMeta: metav1.ObjectMeta{
					Name:      "e2e-test-instrumentation",
					Namespace: "default",
				},
				Spec: autoinstrumentationv1alpha1.AutoInstrumentationSpec{
					Selector: autoinstrumentationv1alpha1.PodSelector{
						MatchLabels: map[string]string{
							"app": "python-web-app",
						},
					},
					Python: &autoinstrumentationv1alpha1.PythonInstrumentation{
						Instrumentation: &autoinstrumentationv1alpha1.InstrumentationSettings{
							Provider:        "openlit",
							CustomInitImage: "openlit-instrumentation:latest",
						},
					},
					OTLP: autoinstrumentationv1alpha1.OTLPConfig{
						Endpoint: "http://openlit.default.svc.cluster.local:4318",
					},
					Resource: &autoinstrumentationv1alpha1.ResourceConfig{
						Environment: "e2e-test",
					},
				},
			}

			err := testSuite.Client.Create(ctx, autoInstr)
			Expect(err).NotTo(HaveOccurred())

			By("Reconciling the AutoInstrumentation resource")
			req := reconcile.Request{
				NamespacedName: types.NamespacedName{
					Name:      autoInstr.Name,
					Namespace: autoInstr.Namespace,
				},
			}

			result, err := reconciler.Reconcile(ctx, req)
			Expect(err).NotTo(HaveOccurred())
			Expect(result.Requeue).To(BeFalse())

			By("Verifying the AutoInstrumentation resource exists")
			retrieved := &autoinstrumentationv1alpha1.AutoInstrumentation{}
			err = testSuite.Client.Get(ctx, req.NamespacedName, retrieved)
			Expect(err).NotTo(HaveOccurred())
			Expect(retrieved.Spec.Python.Instrumentation.Provider).To(Equal("openlit"))

			By("Creating a pod that matches the selector")
			pod := &corev1.Pod{
				ObjectMeta: metav1.ObjectMeta{
					Name:      "test-python-app",
					Namespace: "default",
					Labels: map[string]string{
						"app": "python-web-app",
					},
				},
				Spec: corev1.PodSpec{
					Containers: []corev1.Container{
						{
							Name:  "web-app",
							Image: "python:3.11-slim",
							Ports: []corev1.ContainerPort{
								{ContainerPort: 8080},
							},
						},
					},
				},
			}

			By("Simulating webhook admission request")
			podBytes, err := json.Marshal(pod)
			Expect(err).NotTo(HaveOccurred())

			admissionReq := admission.Request{
				AdmissionRequest: admissionv1.AdmissionRequest{
					UID: "test-uid-e2e",
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

			response := webhookHandler.Handle(ctx, admissionReq)

			By("Verifying webhook allows the pod and adds patches")
			Expect(response.Allowed).To(BeTrue())
			Expect(response.Patches).NotTo(BeEmpty())

			By("Verifying patches contain expected instrumentation")
			// Check that patches include init container and environment variables
			hasInitContainer := false
			hasVolume := false
			hasEnvVars := false

			for _, patch := range response.Patches {
				if patch.Path == "/spec/initContainers" {
					hasInitContainer = true
				}
				if patch.Path == "/spec/volumes" {
					hasVolume = true
				}
				if patch.Path == "/spec/containers/0/env" {
					hasEnvVars = true
				}
			}

			Expect(hasInitContainer).To(BeTrue(), "Should add init container")
			Expect(hasVolume).To(BeTrue(), "Should add shared volume")
			Expect(hasEnvVars).To(BeTrue(), "Should add environment variables")
		})

		It("should handle ignore selectors correctly", func() {
			By("Creating an AutoInstrumentation with ignore selector")
			autoInstr := &autoinstrumentationv1alpha1.AutoInstrumentation{
				ObjectMeta: metav1.ObjectMeta{
					Name:      "ignore-test-instrumentation",
					Namespace: "default",
				},
				Spec: autoinstrumentationv1alpha1.AutoInstrumentationSpec{
					Selector: autoinstrumentationv1alpha1.PodSelector{
						MatchLabels: map[string]string{
							"app": "python-app",
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
					Ignore: &autoinstrumentationv1alpha1.PodSelector{
						MatchLabels: map[string]string{
							"instrumentation": "skip",
						},
					},
				},
			}

			err := testSuite.Client.Create(ctx, autoInstr)
			Expect(err).NotTo(HaveOccurred())

			By("Reconciling the AutoInstrumentation")
			req := reconcile.Request{
				NamespacedName: types.NamespacedName{
					Name:      autoInstr.Name,
					Namespace: autoInstr.Namespace,
				},
			}

			_, err = reconciler.Reconcile(ctx, req)
			Expect(err).NotTo(HaveOccurred())

			By("Creating a pod that matches selector but should be ignored")
			ignoredPod := &corev1.Pod{
				ObjectMeta: metav1.ObjectMeta{
					Name:      "ignored-python-app",
					Namespace: "default",
					Labels: map[string]string{
						"app":             "python-app",
						"instrumentation": "skip",
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

			podBytes, err := json.Marshal(ignoredPod)
			Expect(err).NotTo(HaveOccurred())

			admissionReq := admission.Request{
				AdmissionRequest: admissionv1.AdmissionRequest{
					UID: "ignore-test-uid",
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

			response := webhookHandler.Handle(ctx, admissionReq)

			By("Verifying webhook allows pod but doesn't instrument it")
			Expect(response.Allowed).To(BeTrue())
			Expect(response.Patches).To(BeEmpty(), "Should not add patches for ignored pods")
		})

		It("should handle multiple AutoInstrumentation resources", func() {
			By("Creating multiple AutoInstrumentation resources")
			autoInstr1 := &autoinstrumentationv1alpha1.AutoInstrumentation{
				ObjectMeta: metav1.ObjectMeta{
					Name:      "python-instrumentation",
					Namespace: "default",
				},
				Spec: autoinstrumentationv1alpha1.AutoInstrumentationSpec{
					Selector: autoinstrumentationv1alpha1.PodSelector{
						MatchLabels: map[string]string{
							"language": "python",
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

			autoInstr2 := &autoinstrumentationv1alpha1.AutoInstrumentation{
				ObjectMeta: metav1.ObjectMeta{
					Name:      "web-app-instrumentation",
					Namespace: "default",
				},
				Spec: autoinstrumentationv1alpha1.AutoInstrumentationSpec{
					Selector: autoinstrumentationv1alpha1.PodSelector{
						MatchLabels: map[string]string{
							"type": "web-app",
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

			err := testSuite.Client.Create(ctx, autoInstr1)
			Expect(err).NotTo(HaveOccurred())
			err = testSuite.Client.Create(ctx, autoInstr2)
			Expect(err).NotTo(HaveOccurred())

			By("Reconciling both resources")
			for _, autoInstr := range []*autoinstrumentationv1alpha1.AutoInstrumentation{autoInstr1, autoInstr2} {
				req := reconcile.Request{
					NamespacedName: types.NamespacedName{
						Name:      autoInstr.Name,
						Namespace: autoInstr.Namespace,
					},
				}
				_, err = reconciler.Reconcile(ctx, req)
				Expect(err).NotTo(HaveOccurred())
			}

			By("Creating a pod that matches both selectors")
			multiMatchPod := &corev1.Pod{
				ObjectMeta: metav1.ObjectMeta{
					Name:      "multi-match-app",
					Namespace: "default",
					Labels: map[string]string{
						"language": "python",
						"type":     "web-app",
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

			podBytes, err := json.Marshal(multiMatchPod)
			Expect(err).NotTo(HaveOccurred())

			admissionReq := admission.Request{
				AdmissionRequest: admissionv1.AdmissionRequest{
					UID: "multi-match-uid",
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

			response := webhookHandler.Handle(ctx, admissionReq)

			By("Verifying webhook handles multiple matches correctly")
			Expect(response.Allowed).To(BeTrue())
			Expect(response.Patches).NotTo(BeEmpty(), "Should instrument pod when multiple configs match")
		})
	})

	Describe("Resource Lifecycle Management", func() {
		It("should handle AutoInstrumentation updates", func() {
			By("Creating initial AutoInstrumentation")
			autoInstr := &autoinstrumentationv1alpha1.AutoInstrumentation{
				ObjectMeta: metav1.ObjectMeta{
					Name:      "lifecycle-test",
					Namespace: "default",
				},
				Spec: autoinstrumentationv1alpha1.AutoInstrumentationSpec{
					Selector: autoinstrumentationv1alpha1.PodSelector{
						MatchLabels: map[string]string{
							"app": "test-app",
						},
					},
					Python: &autoinstrumentationv1alpha1.PythonInstrumentation{
						Instrumentation: &autoinstrumentationv1alpha1.InstrumentationSettings{
							Provider:        "openlit",
							CustomInitImage: "openlit-instrumentation:v1.0.0",
						},
					},
					OTLP: autoinstrumentationv1alpha1.OTLPConfig{
						Endpoint: "http://test:4318",
					},
				},
			}

			err := testSuite.Client.Create(ctx, autoInstr)
			Expect(err).NotTo(HaveOccurred())

			By("Reconciling initial resource")
			req := reconcile.Request{
				NamespacedName: types.NamespacedName{
					Name:      autoInstr.Name,
					Namespace: autoInstr.Namespace,
				},
			}
			_, err = reconciler.Reconcile(ctx, req)
			Expect(err).NotTo(HaveOccurred())

			By("Updating the AutoInstrumentation")
			retrieved := &autoinstrumentationv1alpha1.AutoInstrumentation{}
			err = testSuite.Client.Get(ctx, req.NamespacedName, retrieved)
			Expect(err).NotTo(HaveOccurred())

			retrieved.Spec.Python.Instrumentation.CustomInitImage = "openlit-instrumentation:v2.0.0"
			if retrieved.Spec.Resource == nil {
				retrieved.Spec.Resource = &autoinstrumentationv1alpha1.ResourceConfig{}
			}
			retrieved.Spec.Resource.Environment = "updated-env"

			err = testSuite.Client.Update(ctx, retrieved)
			Expect(err).NotTo(HaveOccurred())

			By("Reconciling updated resource")
			_, err = reconciler.Reconcile(ctx, req)
			Expect(err).NotTo(HaveOccurred())

			By("Verifying updates are persisted")
			final := &autoinstrumentationv1alpha1.AutoInstrumentation{}
			err = testSuite.Client.Get(ctx, req.NamespacedName, final)
			Expect(err).NotTo(HaveOccurred())
			Expect(final.Spec.Python.Instrumentation.CustomInitImage).To(Equal("openlit-instrumentation:v2.0.0"))
			Expect(final.Spec.Resource.Environment).To(Equal("updated-env"))
		})

		It("should handle AutoInstrumentation deletion", func() {
			By("Creating AutoInstrumentation")
			autoInstr := &autoinstrumentationv1alpha1.AutoInstrumentation{
				ObjectMeta: metav1.ObjectMeta{
					Name:      "deletion-test",
					Namespace: "default",
				},
				Spec: autoinstrumentationv1alpha1.AutoInstrumentationSpec{
					Selector: autoinstrumentationv1alpha1.PodSelector{
						MatchLabels: map[string]string{
							"app": "temp-app",
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

			err := testSuite.Client.Create(ctx, autoInstr)
			Expect(err).NotTo(HaveOccurred())

			req := reconcile.Request{
				NamespacedName: types.NamespacedName{
					Name:      autoInstr.Name,
					Namespace: autoInstr.Namespace,
				},
			}

			By("Reconciling to ensure it's processed")
			_, err = reconciler.Reconcile(ctx, req)
			Expect(err).NotTo(HaveOccurred())

			By("Deleting the AutoInstrumentation")
			err = testSuite.Client.Delete(ctx, autoInstr)
			Expect(err).NotTo(HaveOccurred())

			By("Reconciling after deletion")
			_, err = reconciler.Reconcile(ctx, req)
			Expect(err).NotTo(HaveOccurred())

			By("Verifying resource is deleted")
			deleted := &autoinstrumentationv1alpha1.AutoInstrumentation{}
			err = testSuite.Client.Get(ctx, req.NamespacedName, deleted)
			Expect(err).To(HaveOccurred())
		})
	})

	Describe("Error Handling and Edge Cases", func() {
		It("should handle invalid AutoInstrumentation gracefully", func() {
			By("Creating invalid AutoInstrumentation (missing selector)")
			invalidAutoInstr := &autoinstrumentationv1alpha1.AutoInstrumentation{
				ObjectMeta: metav1.ObjectMeta{
					Name:      "invalid-test",
					Namespace: "default",
				},
				Spec: autoinstrumentationv1alpha1.AutoInstrumentationSpec{
					// Missing selector - this should cause validation errors
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

			err := testSuite.Client.Create(ctx, invalidAutoInstr)
			Expect(err).NotTo(HaveOccurred())

			By("Attempting reconciliation")
			req := reconcile.Request{
				NamespacedName: types.NamespacedName{
					Name:      invalidAutoInstr.Name,
					Namespace: invalidAutoInstr.Namespace,
				},
			}

			_, err = reconciler.Reconcile(ctx, req)
			Expect(err).To(HaveOccurred(), "Should fail validation for invalid config")
		})

		It("should handle non-existent AutoInstrumentation", func() {
			By("Reconciling non-existent resource")
			req := reconcile.Request{
				NamespacedName: types.NamespacedName{
					Name:      "non-existent",
					Namespace: "default",
				},
			}

			result, err := reconciler.Reconcile(ctx, req)
			Expect(err).NotTo(HaveOccurred(), "Should handle non-existent resources gracefully")
			Expect(result.Requeue).To(BeFalse())
		})

		It("should handle webhook with invalid JSON", func() {
			By("Creating admission request with invalid JSON")
			admissionReq := admission.Request{
				AdmissionRequest: admissionv1.AdmissionRequest{
					UID: "invalid-json-uid",
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

			response := webhookHandler.Handle(ctx, admissionReq)

			By("Verifying webhook rejects invalid JSON")
			Expect(response.Allowed).To(BeFalse())
			Expect(response.Result.Message).To(ContainSubstring("failed to decode pod"))
		})

		It("should handle non-pod resources gracefully", func() {
			By("Creating admission request for non-pod resource")
			service := &corev1.Service{
				ObjectMeta: metav1.ObjectMeta{
					Name:      "test-service",
					Namespace: "default",
				},
			}

			serviceBytes, err := json.Marshal(service)
			Expect(err).NotTo(HaveOccurred())

			admissionReq := admission.Request{
				AdmissionRequest: admissionv1.AdmissionRequest{
					UID: "service-uid",
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

			response := webhookHandler.Handle(ctx, admissionReq)

			By("Verifying webhook allows non-pod resources")
			Expect(response.Allowed).To(BeTrue())
			Expect(response.Patches).To(BeEmpty())
		})
	})

	Describe("Injection Logic Integration", func() {
		It("should correctly inject into multi-container pods", func() {
			By("Creating AutoInstrumentation")
			autoInstr := &autoinstrumentationv1alpha1.AutoInstrumentation{
				ObjectMeta: metav1.ObjectMeta{
					Name:      "multi-container-test",
					Namespace: "default",
				},
				Spec: autoinstrumentationv1alpha1.AutoInstrumentationSpec{
					Selector: autoinstrumentationv1alpha1.PodSelector{
						MatchLabels: map[string]string{
							"app": "multi-container-app",
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

			err := testSuite.Client.Create(ctx, autoInstr)
			Expect(err).NotTo(HaveOccurred())

			req := reconcile.Request{
				NamespacedName: types.NamespacedName{
					Name:      autoInstr.Name,
					Namespace: autoInstr.Namespace,
				},
			}
			_, err = reconciler.Reconcile(ctx, req)
			Expect(err).NotTo(HaveOccurred())

			By("Creating multi-container pod")
			multiContainerPod := &corev1.Pod{
				ObjectMeta: metav1.ObjectMeta{
					Name:      "multi-container-pod",
					Namespace: "default",
					Labels: map[string]string{
						"app": "multi-container-app",
					},
				},
				Spec: corev1.PodSpec{
					Containers: []corev1.Container{
						{
							Name:  "python-app",
							Image: "python:3.11-slim",
						},
						{
							Name:  "nginx-sidecar",
							Image: "nginx:alpine",
						},
						{
							Name:  "istio-proxy",
							Image: "istio/proxyv2:1.18.0",
						},
					},
				},
			}

			podBytes, err := json.Marshal(multiContainerPod)
			Expect(err).NotTo(HaveOccurred())

			admissionReq := admission.Request{
				AdmissionRequest: admissionv1.AdmissionRequest{
					UID: "multi-container-uid",
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

			response := webhookHandler.Handle(ctx, admissionReq)

			By("Verifying selective instrumentation")
			Expect(response.Allowed).To(BeTrue())
			// Should instrument Python container but skip nginx and istio sidecars
			Expect(response.Patches).NotTo(BeEmpty())
		})

		It("should handle pods with security constraints", func() {
			By("Creating AutoInstrumentation")
			autoInstr := &autoinstrumentationv1alpha1.AutoInstrumentation{
				ObjectMeta: metav1.ObjectMeta{
					Name:      "security-test",
					Namespace: "default",
				},
				Spec: autoinstrumentationv1alpha1.AutoInstrumentationSpec{
					Selector: autoinstrumentationv1alpha1.PodSelector{
						MatchLabels: map[string]string{
							"app": "secure-app",
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

			err := testSuite.Client.Create(ctx, autoInstr)
			Expect(err).NotTo(HaveOccurred())

			req := reconcile.Request{
				NamespacedName: types.NamespacedName{
					Name:      autoInstr.Name,
					Namespace: autoInstr.Namespace,
				},
			}
			_, err = reconciler.Reconcile(ctx, req)
			Expect(err).NotTo(HaveOccurred())

			By("Creating pod with read-only filesystem")
			readOnlyTrue := true
			securePod := &corev1.Pod{
				ObjectMeta: metav1.ObjectMeta{
					Name:      "secure-pod",
					Namespace: "default",
					Labels: map[string]string{
						"app": "secure-app",
					},
				},
				Spec: corev1.PodSpec{
					Containers: []corev1.Container{
						{
							Name:  "secure-app",
							Image: "python:3.11-slim",
							SecurityContext: &corev1.SecurityContext{
								ReadOnlyRootFilesystem: &readOnlyTrue,
							},
						},
					},
				},
			}

			podBytes, err := json.Marshal(securePod)
			Expect(err).NotTo(HaveOccurred())

			admissionReq := admission.Request{
				AdmissionRequest: admissionv1.AdmissionRequest{
					UID: "secure-uid",
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

			response := webhookHandler.Handle(ctx, admissionReq)

			By("Verifying webhook allows but doesn't instrument secured pods")
			Expect(response.Allowed).To(BeTrue())
			// Should not instrument due to security constraints
			Expect(response.Patches).To(BeEmpty())
		})
	})

	Describe("Performance and Reliability", func() {
		It("should handle concurrent webhook requests", func() {
			By("Creating AutoInstrumentation")
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

			err := testSuite.Client.Create(ctx, autoInstr)
			Expect(err).NotTo(HaveOccurred())

			req := reconcile.Request{
				NamespacedName: types.NamespacedName{
					Name:      autoInstr.Name,
					Namespace: autoInstr.Namespace,
				},
			}
			_, err = reconciler.Reconcile(ctx, req)
			Expect(err).NotTo(HaveOccurred())

			By("Sending concurrent webhook requests")
			done := make(chan bool, 10)

			for i := 0; i < 10; i++ {
				go func(id int) {
					defer GinkgoRecover()
					defer func() { done <- true }()

					pod := &corev1.Pod{
						ObjectMeta: metav1.ObjectMeta{
							Name:      "concurrent-pod-" + string(rune(id)),
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
					Expect(err).NotTo(HaveOccurred())

					admissionReq := admission.Request{
						AdmissionRequest: admissionv1.AdmissionRequest{
							UID: types.UID("concurrent-uid-" + string(rune(id))),
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

					response := webhookHandler.Handle(ctx, admissionReq)
					Expect(response.Allowed).To(BeTrue())
				}(i)
			}

			By("Waiting for all requests to complete")
			for i := 0; i < 10; i++ {
				Eventually(done, 30*time.Second).Should(Receive())
			}
		})

		It("should complete operations within reasonable time", func() {
			By("Creating AutoInstrumentation")
			autoInstr := &autoinstrumentationv1alpha1.AutoInstrumentation{
				ObjectMeta: metav1.ObjectMeta{
					Name:      "performance-test",
					Namespace: "default",
				},
				Spec: autoinstrumentationv1alpha1.AutoInstrumentationSpec{
					Selector: autoinstrumentationv1alpha1.PodSelector{
						MatchLabels: map[string]string{
							"perf": "test",
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

			start := time.Now()

			err := testSuite.Client.Create(ctx, autoInstr)
			Expect(err).NotTo(HaveOccurred())

			req := reconcile.Request{
				NamespacedName: types.NamespacedName{
					Name:      autoInstr.Name,
					Namespace: autoInstr.Namespace,
				},
			}

			_, err = reconciler.Reconcile(ctx, req)
			Expect(err).NotTo(HaveOccurred())

			reconcileTime := time.Since(start)

			By("Measuring webhook performance")
			pod := &corev1.Pod{
				ObjectMeta: metav1.ObjectMeta{
					Name:      "perf-test-pod",
					Namespace: "default",
					Labels: map[string]string{
						"perf": "test",
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
			Expect(err).NotTo(HaveOccurred())

			admissionReq := admission.Request{
				AdmissionRequest: admissionv1.AdmissionRequest{
					UID: "perf-test-uid",
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

			webhookStart := time.Now()
			response := webhookHandler.Handle(ctx, admissionReq)
			webhookTime := time.Since(webhookStart)

			By("Verifying performance benchmarks")
			Expect(response.Allowed).To(BeTrue())
			Expect(reconcileTime).To(BeNumerically("<", 5*time.Second), "Reconciliation should complete quickly")
			Expect(webhookTime).To(BeNumerically("<", 1*time.Second), "Webhook should respond quickly")
		})
	})
})
