/*
Copyright 2024.

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

package v1alpha1

import (
	metav1 "k8s.io/apimachinery/pkg/apis/meta/v1"
)

// EDIT THIS FILE!  This is scaffolding for you to own.
// NOTE: json tags are required.  Any new fields you add must have json:"-" or json:"fieldName" omitempty.

// AutoInstrumentationSpec defines the desired state of AutoInstrumentation
type AutoInstrumentationSpec struct {
	// Selector defines which pods should be instrumented
	// +kubebuilder:validation:Required
	Selector PodSelector `json:"selector"`

	// Ignore defines which pods should be skipped for instrumentation
	// +optional
	Ignore *PodSelector `json:"ignore,omitempty"`

	// Python defines Python-specific instrumentation configuration
	// +optional
	Python *PythonInstrumentation `json:"python,omitempty"`

	// OTLP defines OpenTelemetry Protocol configuration
	// +kubebuilder:validation:Required
	OTLP OTLPConfig `json:"otlp"`

	// Resource defines resource attributes for telemetry
	// +optional
	Resource *ResourceConfig `json:"resource,omitempty"`
}

// PodSelector defines label and expression-based pod selection
type PodSelector struct {
	// MatchLabels is a map of {key,value} pairs
	// +optional
	MatchLabels map[string]string `json:"matchLabels,omitempty"`

	// MatchExpressions is a list of label selector requirements
	// +optional
	MatchExpressions []metav1.LabelSelectorRequirement `json:"matchExpressions,omitempty"`
}

// PythonInstrumentation defines Python-specific instrumentation settings
type PythonInstrumentation struct {
	// Instrumentation defines instrumentation configuration
	// +optional
	Instrumentation *InstrumentationSettings `json:"instrumentation,omitempty"`
}

// EnvVar represents an environment variable to be injected
type EnvVar struct {
	// Name is the environment variable name
	// +kubebuilder:validation:Required
	// +kubebuilder:validation:Pattern:="^[a-zA-Z_][a-zA-Z0-9_]*$"
	Name string `json:"name"`

	// Value is the environment variable value
	// +optional
	Value string `json:"value,omitempty"`

	// ValueFrom specifies a source for the environment variable's value
	// +optional
	ValueFrom *EnvVarSource `json:"valueFrom,omitempty"`
}

// EnvVarSource represents a source for the value of an EnvVar
type EnvVarSource struct {
	// SecretKeyRef selects a key of a secret in the pod's namespace
	// +optional
	SecretKeyRef *SecretKeySelector `json:"secretKeyRef,omitempty"`

	// ConfigMapKeyRef selects a key of a ConfigMap in the pod's namespace
	// +optional
	ConfigMapKeyRef *ConfigMapKeySelector `json:"configMapKeyRef,omitempty"`

	// FieldRef selects a field of the pod
	// +optional
	FieldRef *ObjectFieldSelector `json:"fieldRef,omitempty"`
}

// SecretKeySelector selects a key of a Secret
type SecretKeySelector struct {
	// Name of the secret in the pod's namespace to select from
	// +kubebuilder:validation:Required
	Name string `json:"name"`

	// Key of the secret to select from
	// +kubebuilder:validation:Required
	Key string `json:"key"`

	// Optional specifies whether the Secret or its key must be defined
	// +optional
	Optional *bool `json:"optional,omitempty"`
}

// ConfigMapKeySelector selects a key of a ConfigMap
type ConfigMapKeySelector struct {
	// Name of the ConfigMap in the pod's namespace to select from
	// +kubebuilder:validation:Required
	Name string `json:"name"`

	// Key of the ConfigMap to select from
	// +kubebuilder:validation:Required
	Key string `json:"key"`

	// Optional specifies whether the ConfigMap or its key must be defined
	// +optional
	Optional *bool `json:"optional,omitempty"`
}

// ObjectFieldSelector selects an API field of an object
type ObjectFieldSelector struct {
	// Path of the field to select in the specified API version
	// +kubebuilder:validation:Required
	FieldPath string `json:"fieldPath"`

	// API version of the referent
	// +optional
	APIVersion string `json:"apiVersion,omitempty"`
}

// InstrumentationSettings defines generic instrumentation settings
type InstrumentationSettings struct {
	// Enabled controls whether instrumentation is enabled
	// +kubebuilder:default:=true
	// +optional
	Enabled *bool `json:"enabled,omitempty"`

	// Version specifies the instrumentation version
	// +kubebuilder:default:="latest"
	// +kubebuilder:validation:Pattern:=^(latest|[0-9]+\.[0-9]+\.[0-9]+.*)$
	// +optional
	Version string `json:"version,omitempty"`

	// Provider specifies the instrumentation provider
	// +kubebuilder:validation:Enum:=openlit;openinference;openllmetry;custom
	// +kubebuilder:default:="openlit"
	// +optional
	Provider string `json:"provider,omitempty"`

	// ImagePullPolicy defines the image pull policy for init containers
	// +kubebuilder:validation:Enum:=Always;IfNotPresent;Never
	// +kubebuilder:default:="IfNotPresent"
	// +optional
	ImagePullPolicy string `json:"imagePullPolicy,omitempty"`

	// CustomPackages specifies additional packages to install (comma-separated)
	// +kubebuilder:validation:Pattern:="^[a-zA-Z0-9_\\-\\.,=<>!\\s]*$"
	// +optional
	CustomPackages string `json:"customPackages,omitempty"`

	// CustomInitImage specifies a custom init container image
	// +kubebuilder:validation:Pattern:="^[a-z0-9.-]+(/[a-z0-9._-]+)*:[a-zA-Z0-9._-]+$"
	// +optional
	CustomInitImage string `json:"customInitImage,omitempty"`

	// Env defines environment variables to be injected into instrumented containers
	// +optional
	Env []EnvVar `json:"env,omitempty"`
}

// OTLPConfig defines OpenTelemetry Protocol configuration
type OTLPConfig struct {
	// Endpoint specifies the OTLP endpoint URL
	// +kubebuilder:validation:Required
	// +kubebuilder:validation:Pattern:="^https?://[a-zA-Z0-9.-]+(:[0-9]+)?(/.*)?$"
	Endpoint string `json:"endpoint"`

	// Headers specifies additional headers (key=value format, comma-separated)
	// +kubebuilder:validation:Pattern:="^[a-zA-Z0-9_\\-]+=.*(,[a-zA-Z0-9_\\-]+=.*)*$"
	// +optional
	Headers string `json:"headers,omitempty"`

	// Timeout specifies the timeout in seconds
	// +kubebuilder:validation:Minimum:=1
	// +kubebuilder:validation:Maximum:=300
	// +kubebuilder:default:=30
	// +optional
	Timeout *int32 `json:"timeout,omitempty"`
}

// ResourceConfig defines resource attributes for telemetry
type ResourceConfig struct {
	// Environment specifies the deployment environment
	// +kubebuilder:validation:Pattern:="^[a-zA-Z0-9_\\-]+$"
	// +optional
	Environment string `json:"environment,omitempty"`
}

// AutoInstrumentationStatus defines the observed state of AutoInstrumentation
type AutoInstrumentationStatus struct {
	// Conditions represent the latest available observations of the resource's state
	// +optional
	Conditions []metav1.Condition `json:"conditions,omitempty"`

	// InstrumentedPods tracks which pods have been instrumented
	// +optional
	InstrumentedPods []InstrumentedPod `json:"instrumentedPods,omitempty"`

	// ValidationErrors tracks validation errors
	// +optional
	ValidationErrors []string `json:"validationErrors,omitempty"`

	// LastProcessed tracks the last time this config was processed
	// +optional
	LastProcessed *metav1.Time `json:"lastProcessed,omitempty"`
}

// InstrumentedPod represents a pod that has been instrumented
type InstrumentedPod struct {
	// Name is the pod name
	Name string `json:"name"`

	// Namespace is the pod namespace
	Namespace string `json:"namespace"`

	// InstrumentedAt is when the pod was instrumented
	InstrumentedAt metav1.Time `json:"instrumentedAt"`

	// Provider is the instrumentation provider used
	Provider string `json:"provider"`
}

//+kubebuilder:object:root=true
//+kubebuilder:subresource:status
//+kubebuilder:printcolumn:name="Provider",type=string,JSONPath=`.spec.python.instrumentation.provider`
//+kubebuilder:printcolumn:name="Endpoint",type=string,JSONPath=`.spec.otlp.endpoint`
//+kubebuilder:printcolumn:name="Environment",type=string,JSONPath=`.spec.resource.environment`
//+kubebuilder:printcolumn:name="Age",type=date,JSONPath=`.metadata.creationTimestamp`
//+kubebuilder:resource:shortName=ai

// AutoInstrumentation is the Schema for the autoinstrumentations API
type AutoInstrumentation struct {
	metav1.TypeMeta   `json:",inline"`
	metav1.ObjectMeta `json:"metadata,omitempty"`

	Spec   AutoInstrumentationSpec   `json:"spec,omitempty"`
	Status AutoInstrumentationStatus `json:"status,omitempty"`
}

//+kubebuilder:object:root=true

// AutoInstrumentationList contains a list of AutoInstrumentation
type AutoInstrumentationList struct {
	metav1.TypeMeta `json:",inline"`
	metav1.ListMeta `json:"metadata,omitempty"`
	Items           []AutoInstrumentation `json:"items"`
}

func init() {
	SchemeBuilder.Register(&AutoInstrumentation{}, &AutoInstrumentationList{})
}
