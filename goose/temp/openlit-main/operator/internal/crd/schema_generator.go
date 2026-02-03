package crd

import (
	"reflect"
	"strconv"
	"strings"

	apiextensionsv1 "k8s.io/apiextensions-apiserver/pkg/apis/apiextensions/v1"

	"github.com/openlit/openlit/operator/api/v1alpha1"
)

// SchemaGenerator generates OpenAPI v3 schemas from Go types using reflection
type SchemaGenerator struct{}

// NewSchemaGenerator creates a new schema generator
func NewSchemaGenerator() *SchemaGenerator {
	return &SchemaGenerator{}
}

// GenerateAutoInstrumentationSchema generates the schema for AutoInstrumentation using reflection
func (g *SchemaGenerator) GenerateAutoInstrumentationSchema() *apiextensionsv1.JSONSchemaProps {
	// Generate schema from the actual Go type
	specType := reflect.TypeOf(v1alpha1.AutoInstrumentationSpec{})
	statusType := reflect.TypeOf(v1alpha1.AutoInstrumentationStatus{})

	return &apiextensionsv1.JSONSchemaProps{
		Type:        "object",
		Description: "AutoInstrumentation defines the configuration for automatic instrumentation of applications",
		Required:    []string{"spec"},
		Properties: map[string]apiextensionsv1.JSONSchemaProps{
			"spec":   g.generateSchemaFromType(specType),
			"status": g.generateSchemaFromType(statusType),
			// Note: apiVersion, kind, and metadata are automatically provided by Kubernetes
			// and should NOT be specified in the OpenAPI schema
		},
	}
}

// generateSchemaFromType generates JSON schema from a Go type using reflection
func (g *SchemaGenerator) generateSchemaFromType(t reflect.Type) apiextensionsv1.JSONSchemaProps {
	if t.Kind() == reflect.Ptr {
		t = t.Elem()
	}

	// Special handling for metav1.Time
	if t.PkgPath() == "k8s.io/apimachinery/pkg/apis/meta/v1" && t.Name() == "Time" {
		return apiextensionsv1.JSONSchemaProps{
			Type:   "string",
			Format: "date-time",
		}
	}

	switch t.Kind() {
	case reflect.Struct:
		return g.generateStructSchema(t)
	case reflect.Slice:
		return g.generateSliceSchema(t)
	case reflect.Map:
		return g.generateMapSchema(t)
	case reflect.String:
		return apiextensionsv1.JSONSchemaProps{Type: "string"}
	case reflect.Int, reflect.Int32, reflect.Int64:
		return apiextensionsv1.JSONSchemaProps{Type: "integer"}
	case reflect.Bool:
		return apiextensionsv1.JSONSchemaProps{Type: "boolean"}
	default:
		return apiextensionsv1.JSONSchemaProps{Type: "object"}
	}
}

// generateStructSchema generates schema for struct types
func (g *SchemaGenerator) generateStructSchema(t reflect.Type) apiextensionsv1.JSONSchemaProps {
	properties := make(map[string]apiextensionsv1.JSONSchemaProps)
	var required []string

	for i := 0; i < t.NumField(); i++ {
		field := t.Field(i)

		// Skip unexported fields
		if !field.IsExported() {
			continue
		}

		// Get JSON tag
		jsonTag := field.Tag.Get("json")
		if jsonTag == "" || jsonTag == "-" {
			continue
		}

		// Parse JSON tag
		jsonName := strings.Split(jsonTag, ",")[0]
		if jsonName == "" {
			jsonName = strings.ToLower(field.Name)
		}

		// Check if field is required (no omitempty)
		if !strings.Contains(jsonTag, "omitempty") {
			required = append(required, jsonName)
		}

		// Generate property schema
		fieldSchema := g.generateSchemaFromType(field.Type)

		// Add validation from kubebuilder tags
		g.addValidationFromTags(field, &fieldSchema)

		properties[jsonName] = fieldSchema
	}

	schema := apiextensionsv1.JSONSchemaProps{
		Type:       "object",
		Properties: properties,
	}

	if len(required) > 0 {
		schema.Required = required
	}

	return schema
}

// generateSliceSchema generates schema for slice types
func (g *SchemaGenerator) generateSliceSchema(t reflect.Type) apiextensionsv1.JSONSchemaProps {
	elementType := t.Elem()
	itemSchema := g.generateSchemaFromType(elementType)

	return apiextensionsv1.JSONSchemaProps{
		Type: "array",
		Items: &apiextensionsv1.JSONSchemaPropsOrArray{
			Schema: &itemSchema,
		},
	}
}

// generateMapSchema generates schema for map types
func (g *SchemaGenerator) generateMapSchema(t reflect.Type) apiextensionsv1.JSONSchemaProps {
	valueType := t.Elem()
	valueSchema := g.generateSchemaFromType(valueType)

	return apiextensionsv1.JSONSchemaProps{
		Type: "object",
		AdditionalProperties: &apiextensionsv1.JSONSchemaPropsOrBool{
			Schema: &valueSchema,
		},
	}
}

// addValidationFromTags adds validation rules from kubebuilder tags
func (g *SchemaGenerator) addValidationFromTags(field reflect.StructField, schema *apiextensionsv1.JSONSchemaProps) {
	// Add pattern validation
	if pattern := field.Tag.Get("kubebuilder:validation:Pattern"); pattern != "" {
		// Remove quotes if present
		pattern = strings.Trim(pattern, `"=`)
		schema.Pattern = pattern
	}

	// Add enum validation
	if enum := field.Tag.Get("kubebuilder:validation:Enum"); enum != "" {
		// Parse enum values
		values := strings.Split(enum, ";")
		for _, v := range values {
			v = strings.TrimSpace(v)
			schema.Enum = append(schema.Enum, apiextensionsv1.JSON{Raw: []byte(`"` + v + `"`)})
		}
	}

	// Add minimum/maximum validation
	if min := field.Tag.Get("kubebuilder:validation:Minimum"); min != "" {
		if minVal := parseFloat(min); minVal != nil {
			schema.Minimum = minVal
		}
	}
	if max := field.Tag.Get("kubebuilder:validation:Maximum"); max != "" {
		if maxVal := parseFloat(max); maxVal != nil {
			schema.Maximum = maxVal
		}
	}

	// Add default values
	if defaultVal := field.Tag.Get("kubebuilder:default"); defaultVal != "" {
		// Handle different types of defaults
		switch schema.Type {
		case "string":
			schema.Default = &apiextensionsv1.JSON{Raw: []byte(`"` + defaultVal + `"`)}
		case "boolean":
			schema.Default = &apiextensionsv1.JSON{Raw: []byte(defaultVal)}
		case "integer":
			schema.Default = &apiextensionsv1.JSON{Raw: []byte(defaultVal)}
		}
	}

	// Add description from comment or tag
	if desc := field.Tag.Get("description"); desc != "" {
		schema.Description = desc
	}
}

// parseFloat parses a string to float64 pointer
func parseFloat(s string) *float64 {
	if s == "" {
		return nil
	}

	val, err := strconv.ParseFloat(s, 64)
	if err != nil {
		return nil
	}
	return &val
}
