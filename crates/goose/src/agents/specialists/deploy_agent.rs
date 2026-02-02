//! DeployAgent - Specialist agent for deployment and infrastructure

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::{utils, SpecialistAgent, SpecialistConfig, SpecialistContext};
use crate::agents::orchestrator::{AgentRole, TaskResult};

/// Specialist agent focused on deployment and infrastructure
pub struct DeployAgent {
    config: SpecialistConfig,
    #[allow(dead_code)]
    capabilities: DeployCapabilities,
}

/// Deployment capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeployCapabilities {
    /// Supported deployment platforms
    pub platforms: Vec<String>,
    /// Container technologies supported
    pub containers: Vec<String>,
    /// CI/CD tools supported
    pub cicd_tools: Vec<String>,
    /// Infrastructure as Code tools
    pub iac_tools: Vec<String>,
    /// Cloud providers supported
    pub cloud_providers: Vec<String>,
}

impl Default for DeployCapabilities {
    fn default() -> Self {
        Self {
            platforms: vec![
                "docker".to_string(),
                "kubernetes".to_string(),
                "heroku".to_string(),
                "netlify".to_string(),
                "vercel".to_string(),
                "aws".to_string(),
                "gcp".to_string(),
                "azure".to_string(),
            ],
            containers: vec![
                "docker".to_string(),
                "podman".to_string(),
                "containerd".to_string(),
            ],
            cicd_tools: vec![
                "github_actions".to_string(),
                "gitlab_ci".to_string(),
                "jenkins".to_string(),
                "circleci".to_string(),
                "travis".to_string(),
            ],
            iac_tools: vec![
                "terraform".to_string(),
                "cloudformation".to_string(),
                "pulumi".to_string(),
                "ansible".to_string(),
                "helm".to_string(),
            ],
            cloud_providers: vec![
                "aws".to_string(),
                "gcp".to_string(),
                "azure".to_string(),
                "digitalocean".to_string(),
                "linode".to_string(),
            ],
        }
    }
}

impl DeployAgent {
    /// Create a new DeployAgent with configuration
    pub fn new(config: SpecialistConfig) -> Self {
        Self {
            config,
            capabilities: DeployCapabilities::default(),
        }
    }

    /// Analyze deployment requirements from context
    fn analyze_deploy_requirements(&self, context: &SpecialistContext) -> DeployRequirements {
        let language = context
            .language
            .clone()
            .or_else(|| utils::detect_language(&context.target_files));
        let framework = context
            .framework
            .clone()
            .or_else(|| utils::detect_framework(&context.target_files, language.as_deref()));

        let platform = self.detect_platform(&context.metadata, &language, &framework);
        let deployment_type = self.determine_deployment_type(&context.task);
        let environment = context
            .metadata
            .get("environment")
            .and_then(|v| v.as_str())
            .unwrap_or("staging")
            .to_string();

        DeployRequirements {
            language,
            framework,
            platform,
            deployment_type,
            environment,
            requires_database: context
                .metadata
                .get("requires_database")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            requires_secrets: context
                .metadata
                .get("requires_secrets")
                .and_then(|v| v.as_bool())
                .unwrap_or(true),
            scaling_requirements: context
                .metadata
                .get("scaling")
                .and_then(|v| v.as_str())
                .unwrap_or("basic")
                .to_string(),
            monitoring_required: context
                .metadata
                .get("monitoring")
                .and_then(|v| v.as_bool())
                .unwrap_or(true),
        }
    }

    /// Detect deployment platform
    fn detect_platform(
        &self,
        metadata: &HashMap<String, serde_json::Value>,
        language: &Option<String>,
        framework: &Option<String>,
    ) -> Option<String> {
        // Check explicit platform specification
        if let Some(platform) = metadata.get("platform").and_then(|v| v.as_str()) {
            return Some(platform.to_string());
        }

        // Detect based on language/framework
        match (language.as_deref(), framework.as_deref()) {
            (_, Some("nextjs")) | (_, Some("react")) => Some("vercel".to_string()),
            (Some("javascript") | Some("typescript"), _) => Some("netlify".to_string()),
            (Some("python"), Some("django") | Some("flask")) => Some("heroku".to_string()),
            (Some("rust"), _) => Some("docker".to_string()),
            (Some("go"), _) => Some("kubernetes".to_string()),
            _ => Some("docker".to_string()),
        }
    }

    /// Determine deployment type from task description
    fn determine_deployment_type(&self, task: &str) -> DeploymentType {
        let task_lower = task.to_lowercase();

        if task_lower.contains("production") || task_lower.contains("prod") {
            DeploymentType::Production
        } else if task_lower.contains("staging") {
            DeploymentType::Staging
        } else if task_lower.contains("dev") || task_lower.contains("development") {
            DeploymentType::Development
        } else if task_lower.contains("test") {
            DeploymentType::Testing
        } else {
            DeploymentType::Staging
        }
    }

    /// Generate deployment artifacts
    async fn generate_deployment_artifacts(
        &self,
        context: &SpecialistContext,
        requirements: &DeployRequirements,
    ) -> Result<Vec<DeploymentArtifact>> {
        let mut artifacts = Vec::new();

        match requirements.platform.as_deref() {
            Some("docker") => {
                artifacts.extend(
                    self.generate_docker_deployment(context, requirements)
                        .await?,
                );
            }
            Some("kubernetes") => {
                artifacts.extend(self.generate_k8s_deployment(context, requirements).await?);
            }
            Some("heroku") => {
                artifacts.extend(
                    self.generate_heroku_deployment(context, requirements)
                        .await?,
                );
            }
            Some("netlify") | Some("vercel") => {
                artifacts.extend(
                    self.generate_static_deployment(context, requirements)
                        .await?,
                );
            }
            _ => {
                artifacts.push(DeploymentArtifact {
                    path: format!("{}/deploy.sh", context.working_dir),
                    content: format!(
                        "#!/bin/bash\n# Deployment script for: {}\necho 'Deploy to {:?}'\n",
                        context.task, requirements.platform
                    ),
                    artifact_type: "script".to_string(),
                });
            }
        }

        // Add CI/CD configuration
        artifacts.extend(self.generate_cicd_config(context, requirements).await?);

        Ok(artifacts)
    }

    /// Generate Docker deployment artifacts
    async fn generate_docker_deployment(
        &self,
        context: &SpecialistContext,
        requirements: &DeployRequirements,
    ) -> Result<Vec<DeploymentArtifact>> {
        let mut artifacts = Vec::new();

        // Generate Dockerfile
        let dockerfile_content = match requirements.language.as_deref() {
            Some("rust") => self.generate_rust_dockerfile(requirements),
            Some("python") => self.generate_python_dockerfile(requirements),
            Some("javascript") | Some("typescript") => self.generate_node_dockerfile(requirements),
            Some("go") => self.generate_go_dockerfile(requirements),
            _ => self.generate_generic_dockerfile(requirements),
        };

        artifacts.push(DeploymentArtifact {
            path: format!("{}/Dockerfile", context.working_dir),
            content: dockerfile_content,
            artifact_type: "dockerfile".to_string(),
        });

        // Generate docker-compose.yml
        let compose_content = format!(
            r#"version: '3.8'

services:
  app:
    build: .
    ports:
      - "8080:8080"
    environment:
      - ENV={}
      - PORT=8080
    {}
    restart: unless-stopped

{}
networks:
  default:
    driver: bridge
"#,
            requirements.environment,
            if requirements.requires_secrets {
                "env_file:\n      - .env"
            } else {
                ""
            },
            if requirements.requires_database {
                r#"  db:
    image: postgres:15
    environment:
      POSTGRES_DB: appdb
      POSTGRES_USER: user
      POSTGRES_PASSWORD: password
    volumes:
      - postgres_data:/var/lib/postgresql/data
    ports:
      - "5432:5432"

volumes:
  postgres_data:"#
            } else {
                ""
            }
        );

        artifacts.push(DeploymentArtifact {
            path: format!("{}/docker-compose.yml", context.working_dir),
            content: compose_content,
            artifact_type: "compose".to_string(),
        });

        // Generate .dockerignore
        let dockerignore_content = r#"node_modules
npm-debug.log
Dockerfile
.dockerignore
.git
.gitignore
README.md
.env
.nyc_output
coverage
.cache
.pytest_cache
__pycache__
target/debug
target/doc
"#;

        artifacts.push(DeploymentArtifact {
            path: format!("{}/.dockerignore", context.working_dir),
            content: dockerignore_content.to_string(),
            artifact_type: "config".to_string(),
        });

        Ok(artifacts)
    }

    /// Generate Kubernetes deployment
    async fn generate_k8s_deployment(
        &self,
        context: &SpecialistContext,
        requirements: &DeployRequirements,
    ) -> Result<Vec<DeploymentArtifact>> {
        let mut artifacts = Vec::new();

        let k8s_deployment = format!(
            r#"apiVersion: apps/v1
kind: Deployment
metadata:
  name: app-deployment
  labels:
    app: myapp
spec:
  replicas: {}
  selector:
    matchLabels:
      app: myapp
  template:
    metadata:
      labels:
        app: myapp
    spec:
      containers:
      - name: app
        image: myapp:latest
        ports:
        - containerPort: 8080
        env:
        - name: ENV
          value: "{}"
        {}
        resources:
          requests:
            memory: "64Mi"
            cpu: "250m"
          limits:
            memory: "128Mi"
            cpu: "500m"
---
apiVersion: v1
kind: Service
metadata:
  name: app-service
spec:
  selector:
    app: myapp
  ports:
    - protocol: TCP
      port: 80
      targetPort: 8080
  type: LoadBalancer
"#,
            if requirements.scaling_requirements == "high" {
                5
            } else {
                2
            },
            requirements.environment,
            if requirements.requires_secrets {
                "- name: SECRET_KEY\n          valueFrom:\n            secretKeyRef:\n              name: app-secrets\n              key: secret-key"
            } else {
                ""
            }
        );

        artifacts.push(DeploymentArtifact {
            path: format!("{}/k8s/deployment.yaml", context.working_dir),
            content: k8s_deployment,
            artifact_type: "k8s".to_string(),
        });

        // Add secrets if needed
        if requirements.requires_secrets {
            let secrets_config = r#"apiVersion: v1
kind: Secret
metadata:
  name: app-secrets
type: Opaque
stringData:
  secret-key: "your-secret-key-here"
  # Add other secrets as needed
"#;
            artifacts.push(DeploymentArtifact {
                path: format!("{}/k8s/secrets.yaml", context.working_dir),
                content: secrets_config.to_string(),
                artifact_type: "k8s".to_string(),
            });
        }

        Ok(artifacts)
    }

    /// Generate Heroku deployment
    async fn generate_heroku_deployment(
        &self,
        context: &SpecialistContext,
        requirements: &DeployRequirements,
    ) -> Result<Vec<DeploymentArtifact>> {
        let mut artifacts = Vec::new();

        // Generate Procfile
        let procfile_content = match requirements.language.as_deref() {
            Some("python") => "web: gunicorn app:app\nworker: python worker.py",
            Some("javascript") | Some("typescript") => "web: npm start",
            Some("rust") => "web: ./target/release/app",
            _ => "web: ./start.sh",
        };

        artifacts.push(DeploymentArtifact {
            path: format!("{}/Procfile", context.working_dir),
            content: procfile_content.to_string(),
            artifact_type: "procfile".to_string(),
        });

        // Generate app.json for Heroku
        let app_json = format!(
            r#"{{
  "name": "{}",
  "description": "Deployed application",
  "image": "heroku/{}",
  "addons": [
    {}
  ],
  "env": {{
    "ENV": {{
      "value": "{}"
    }}
  }},
  "buildpacks": [
    {{
      "url": "{}"
    }}
  ]
}}
"#,
            context.task.replace(" ", "-").to_lowercase(),
            match requirements.language.as_deref() {
                Some("python") => "python",
                Some("javascript") | Some("typescript") => "nodejs",
                _ => "default",
            },
            if requirements.requires_database {
                "\"heroku-postgresql:mini\""
            } else {
                ""
            },
            requirements.environment,
            match requirements.language.as_deref() {
                Some("python") => "heroku/python",
                Some("javascript") | Some("typescript") => "heroku/nodejs",
                _ => "heroku/buildpack-default",
            }
        );

        artifacts.push(DeploymentArtifact {
            path: format!("{}/app.json", context.working_dir),
            content: app_json,
            artifact_type: "config".to_string(),
        });

        Ok(artifacts)
    }

    /// Generate static site deployment
    async fn generate_static_deployment(
        &self,
        context: &SpecialistContext,
        requirements: &DeployRequirements,
    ) -> Result<Vec<DeploymentArtifact>> {
        let mut artifacts = Vec::new();

        // Generate netlify.toml or vercel.json based on platform
        match requirements.platform.as_deref() {
            Some("netlify") => {
                let netlify_config = format!(
                    r#"[build]
  command = "npm run build"
  publish = "dist"

[build.environment]
  NODE_ENV = "{}"

[[redirects]]
  from = "/*"
  to = "/index.html"
  status = 200

[context.production]
  command = "npm run build:prod"

[context.deploy-preview]
  command = "npm run build:preview"
"#,
                    requirements.environment
                );

                artifacts.push(DeploymentArtifact {
                    path: format!("{}/netlify.toml", context.working_dir),
                    content: netlify_config,
                    artifact_type: "config".to_string(),
                });
            }
            Some("vercel") => {
                let vercel_config = format!(
                    r#"{{
  "version": 2,
  "builds": [
    {{
      "src": "package.json",
      "use": "@vercel/static-build",
      "config": {{
        "distDir": "dist"
      }}
    }}
  ],
  "routes": [
    {{
      "src": "/(.*)",
      "dest": "/index.html"
    }}
  ],
  "env": {{
    "NODE_ENV": "{}"
  }}
}}
"#,
                    requirements.environment
                );

                artifacts.push(DeploymentArtifact {
                    path: format!("{}/vercel.json", context.working_dir),
                    content: vercel_config,
                    artifact_type: "config".to_string(),
                });
            }
            _ => {}
        }

        Ok(artifacts)
    }

    /// Generate CI/CD configuration
    async fn generate_cicd_config(
        &self,
        context: &SpecialistContext,
        requirements: &DeployRequirements,
    ) -> Result<Vec<DeploymentArtifact>> {
        let mut artifacts = Vec::new();

        // Generate GitHub Actions workflow
        let github_workflow = format!(
            r#"name: Deploy to {}

on:
  push:
    branches: [ main ]
  pull_request:
    branches: [ main ]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@v3
    
    - name: Setup environment
      uses: actions/setup-{}@v3
      with:
        {}: {}
    
    - name: Install dependencies
      run: {}
    
    - name: Run tests
      run: {}
    
    - name: Build
      run: {}

  deploy:
    needs: test
    runs-on: ubuntu-latest
    if: github.ref == 'refs/heads/main'
    
    steps:
    - uses: actions/checkout@v3
    
    - name: Deploy to {}
      run: |
        {}
"#,
            requirements.environment,
            match requirements.language.as_deref() {
                Some("javascript") | Some("typescript") => "node",
                Some("python") => "python",
                Some("rust") => "rust",
                _ => "generic",
            },
            match requirements.language.as_deref() {
                Some("javascript") | Some("typescript") => "node-version",
                Some("python") => "python-version",
                Some("rust") => "toolchain",
                _ => "version",
            },
            match requirements.language.as_deref() {
                Some("javascript") | Some("typescript") => "18",
                Some("python") => "3.9",
                Some("rust") => "stable",
                _ => "latest",
            },
            match requirements.language.as_deref() {
                Some("javascript") | Some("typescript") => "npm install",
                Some("python") => "pip install -r requirements.txt",
                Some("rust") => "cargo build",
                _ => "echo 'Install dependencies'",
            },
            match requirements.language.as_deref() {
                Some("javascript") | Some("typescript") => "npm test",
                Some("python") => "pytest",
                Some("rust") => "cargo test",
                _ => "echo 'Run tests'",
            },
            match requirements.language.as_deref() {
                Some("javascript") | Some("typescript") => "npm run build",
                Some("python") => "echo 'Build complete'",
                Some("rust") => "cargo build --release",
                _ => "echo 'Build complete'",
            },
            requirements.platform.as_deref().unwrap_or("production"),
            match requirements.platform.as_deref() {
                Some("docker") => "docker build -t app . && docker push $REGISTRY/app",
                Some("heroku") => "git push heroku main",
                Some("netlify") => "netlify deploy --prod",
                Some("vercel") => "vercel --prod",
                _ => "echo 'Deploy application'",
            }
        );

        artifacts.push(DeploymentArtifact {
            path: format!("{}/.github/workflows/deploy.yml", context.working_dir),
            content: github_workflow,
            artifact_type: "cicd".to_string(),
        });

        Ok(artifacts)
    }

    fn generate_rust_dockerfile(&self, requirements: &DeployRequirements) -> String {
        format!(
            r#"# Build stage
FROM rust:1.70 as builder

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src

RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/* ./

EXPOSE 8080

ENV ENV={}

CMD ["./app"]
"#,
            requirements.environment
        )
    }

    fn generate_python_dockerfile(&self, requirements: &DeployRequirements) -> String {
        format!(
            r#"FROM python:3.11-slim

WORKDIR /app

COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt

COPY . .

EXPOSE 8080

ENV ENV={}

CMD ["gunicorn", "--bind", "0.0.0.0:8080", "app:app"]
"#,
            requirements.environment
        )
    }

    fn generate_node_dockerfile(&self, requirements: &DeployRequirements) -> String {
        format!(
            r#"FROM node:18-alpine

WORKDIR /app

COPY package*.json ./
RUN npm ci --only=production

COPY . .

RUN npm run build

EXPOSE 8080

ENV ENV={}

CMD ["npm", "start"]
"#,
            requirements.environment
        )
    }

    fn generate_go_dockerfile(&self, requirements: &DeployRequirements) -> String {
        format!(
            r#"FROM golang:1.21-alpine AS builder

WORKDIR /app
COPY go.mod go.sum ./
RUN go mod download

COPY . .
RUN CGO_ENABLED=0 GOOS=linux go build -o main .

FROM alpine:latest
RUN apk --no-cache add ca-certificates
WORKDIR /root/

COPY --from=builder /app/main .

EXPOSE 8080

ENV ENV={}

CMD ["./main"]
"#,
            requirements.environment
        )
    }

    fn generate_generic_dockerfile(&self, requirements: &DeployRequirements) -> String {
        format!(
            r#"FROM ubuntu:22.04

WORKDIR /app

COPY . .

EXPOSE 8080

ENV ENV={}

CMD ["./start.sh"]
"#,
            requirements.environment
        )
    }
}

#[async_trait]
impl SpecialistAgent for DeployAgent {
    fn role(&self) -> AgentRole {
        AgentRole::Deploy
    }

    fn name(&self) -> &str {
        "DeployAgent"
    }

    async fn can_handle(&self, context: &SpecialistContext) -> bool {
        let task_lower = context.task.to_lowercase();
        let deploy_keywords = [
            "deploy",
            "deployment",
            "infrastructure",
            "docker",
            "kubernetes",
            "ci/cd",
            "pipeline",
        ];

        deploy_keywords
            .iter()
            .any(|keyword| task_lower.contains(keyword))
    }

    async fn execute(&self, context: SpecialistContext) -> Result<TaskResult> {
        let requirements = self.analyze_deploy_requirements(&context);

        tracing::info!(
            "DeployAgent executing: {} (Platform: {:?})",
            context.task,
            requirements.platform
        );

        let deployment_artifacts = self
            .generate_deployment_artifacts(&context, &requirements)
            .await?;

        let mut files_modified = Vec::new();
        let mut artifacts = Vec::new();

        for artifact in &deployment_artifacts {
            files_modified.push(artifact.path.clone());
            artifacts.push(format!(
                "Generated {} artifact: {}",
                artifact.artifact_type,
                artifact.path.rsplit('/').next().unwrap_or("file")
            ));
        }

        let mut metrics = HashMap::new();
        metrics.insert(
            "deployment_artifacts".to_string(),
            serde_json::Value::Number(deployment_artifacts.len().into()),
        );
        metrics.insert(
            "platform".to_string(),
            serde_json::Value::String(
                requirements
                    .platform
                    .clone()
                    .unwrap_or_else(|| "generic".to_string()),
            ),
        );
        metrics.insert(
            "environment".to_string(),
            serde_json::Value::String(requirements.environment.clone()),
        );

        let result = TaskResult {
            success: true,
            output: format!(
                "Generated {} deployment artifacts for: {}",
                deployment_artifacts.len(),
                context.task
            ),
            files_modified,
            artifacts,
            metrics,
        };

        tracing::info!(
            "DeployAgent completed successfully: {} artifacts generated",
            deployment_artifacts.len()
        );
        Ok(result)
    }

    fn config(&self) -> &SpecialistConfig {
        &self.config
    }

    async fn estimate_duration(&self, context: &SpecialistContext) -> std::time::Duration {
        let requirements = self.analyze_deploy_requirements(context);
        let base_time = std::time::Duration::from_secs(600); // 10 minutes base

        let platform_factor = match requirements.platform.as_deref() {
            Some("kubernetes") => 600, // K8s is complex
            Some("docker") => 300,
            Some("heroku") => 180,
            _ => 240,
        };

        let env_factor = match requirements.deployment_type {
            DeploymentType::Production => 300, // Production needs more care
            _ => 120,
        };

        base_time + std::time::Duration::from_secs(platform_factor + env_factor)
    }

    async fn validate_result(&self, result: &TaskResult) -> Result<bool> {
        // Check that deployment artifacts were created
        let deployment_files = result
            .files_modified
            .iter()
            .filter(|f| {
                f.contains("Dockerfile")
                    || f.contains("docker-compose")
                    || f.contains(".yml")
                    || f.contains(".yaml")
                    || f.contains("Procfile")
                    || f.contains("netlify.toml")
                    || f.contains("vercel.json")
            })
            .count();

        if deployment_files == 0 {
            return Ok(false);
        }

        // Check that platform was specified
        if let Some(platform) = result.metrics.get("platform") {
            if platform.as_str().unwrap_or("") == "generic" {
                tracing::warn!("Using generic deployment platform");
            }
        }

        Ok(result.success)
    }
}

/// Deployment requirements analysis
#[derive(Debug, Clone)]
struct DeployRequirements {
    language: Option<String>,
    #[allow(dead_code)]
    framework: Option<String>,
    platform: Option<String>,
    deployment_type: DeploymentType,
    environment: String,
    requires_database: bool,
    requires_secrets: bool,
    scaling_requirements: String,
    #[allow(dead_code)]
    monitoring_required: bool,
}

/// Type of deployment
#[derive(Debug, Clone, PartialEq)]
enum DeploymentType {
    Development,
    Testing,
    Staging,
    Production,
}

/// Generated deployment artifact
#[derive(Debug, Clone)]
struct DeploymentArtifact {
    path: String,
    #[allow(dead_code)]
    content: String,
    artifact_type: String,
}
