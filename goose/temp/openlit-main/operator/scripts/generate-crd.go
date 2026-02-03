/*
OpenLIT CRD Generator Script

This script generates the CustomResourceDefinition (CRD) YAML for the
AutoInstrumentation resource using the centralized schema definitions.

The generator creates a complete CRD with:
- OpenAPI v3 schema validation
- Comprehensive field documentation
- Kubebuilder validation markers
- Status subresource configuration
- Printer columns for kubectl output

Output: deploy/crd/openlit.io_autoinstrumentations.yaml

Usage:

	go run scripts/generate-crd.go [output-directory]

The generated CRD can be applied directly to Kubernetes clusters or
included in deployment manifests for automated operator installation.
*/
package main

import (
	"fmt"
	"os"

	apiextensionsv1 "k8s.io/apiextensions-apiserver/pkg/apis/apiextensions/v1"
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
	"sigs.k8s.io/yaml"

	"github.com/openlit/openlit/operator/internal/crd"
)

func main() {
	// Create schema generator
	schemaGenerator := crd.NewSchemaGenerator()

	// Generate AutoInstrumentation CRD
	crd := &apiextensionsv1.CustomResourceDefinition{
		TypeMeta: metav1.TypeMeta{
			APIVersion: "apiextensions.k8s.io/v1",
			Kind:       "CustomResourceDefinition",
		},
		ObjectMeta: metav1.ObjectMeta{
			Name: "autoinstrumentations.openlit.io",
			Annotations: map[string]string{
				"controller-gen.kubebuilder.io/version": "generated-by-openlit-operator",
			},
		},
		Spec: apiextensionsv1.CustomResourceDefinitionSpec{
			Group: "openlit.io",
			Scope: apiextensionsv1.NamespaceScoped,
			Names: apiextensionsv1.CustomResourceDefinitionNames{
				Plural:     "autoinstrumentations",
				Singular:   "autoinstrumentation",
				Kind:       "AutoInstrumentation",
				ShortNames: []string{"ai"},
			},
			Versions: []apiextensionsv1.CustomResourceDefinitionVersion{
				{
					Name:    "v1alpha1",
					Served:  true,
					Storage: true,
					Subresources: &apiextensionsv1.CustomResourceSubresources{
						Status: &apiextensionsv1.CustomResourceSubresourceStatus{},
					},
					Schema: &apiextensionsv1.CustomResourceValidation{
						OpenAPIV3Schema: schemaGenerator.GenerateAutoInstrumentationSchema(),
					},
					AdditionalPrinterColumns: []apiextensionsv1.CustomResourceColumnDefinition{
						{
							Name:        "Python-Enabled",
							Type:        "boolean",
							Description: "Python instrumentation enabled",
							JSONPath:    ".spec.python.instrumentation.enabled",
						},
						{
							Name:        "Provider",
							Type:        "string",
							Description: "Instrumentation provider",
							JSONPath:    ".spec.python.instrumentation.provider",
						},
						{
							Name:        "OTLP-Endpoint",
							Type:        "string",
							Description: "OTLP endpoint",
							JSONPath:    ".spec.otlp.endpoint",
						},
						{
							Name:     "Age",
							Type:     "date",
							JSONPath: ".metadata.creationTimestamp",
						},
					},
				},
			},
		},
	}

	// Convert to YAML
	yamlData, err := yaml.Marshal(crd)
	if err != nil {
		fmt.Printf("Error marshaling CRD to YAML: %v\n", err)
		os.Exit(1)
	}

	// Write to file
	outputDir := "deploy"
	if len(os.Args) > 1 {
		outputDir = os.Args[1]
	}

	err = os.MkdirAll(outputDir, 0755)
	if err != nil {
		fmt.Printf("Error creating output directory: %v\n", err)
		os.Exit(1)
	}

	outputFile := fmt.Sprintf("%s/openlit.io_autoinstrumentations.yaml", outputDir)
	err = os.WriteFile(outputFile, yamlData, 0644)
	if err != nil {
		fmt.Printf("Error writing CRD file: %v\n", err)
		os.Exit(1)
	}

	fmt.Printf("✅ Generated CRD: %s\n", outputFile)
	fmt.Printf("📋 CRD Name: %s\n", crd.ObjectMeta.Name)
	fmt.Printf("🔗 API Version: %s/%s\n", crd.Spec.Group, crd.Spec.Versions[0].Name)
	fmt.Printf("📦 Kind: %s\n", crd.Spec.Names.Kind)
}
