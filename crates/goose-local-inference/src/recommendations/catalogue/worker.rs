use super::{build::verify_memory_fit, MemoryForecast};
use crate::recommendations::forecast::{self, CachedInspection};
use anyhow::{ensure, Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine};
use eredu::api::{
    ArtifactMetadata, CheckpointMetadata, GgufCompanionMetadata, GgufHeader, MetadataProvenance,
    SafetensorsHeader,
};
use eredu_core::artifact::GgufCompanionRole;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    process::{Command, Stdio},
};

static WORKERS: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(2);

#[derive(Serialize, Deserialize)]
struct Header {
    member: String,
    bytes: String,
    file_len: u64,
}

// Only immutable metadata crosses the process boundary; workers cannot fetch weights.
#[derive(Serialize, Deserialize)]
struct Input {
    provenance: MetadataProvenance,
    config: Option<String>,
    index: Option<String>,
    headers: Vec<Header>,
    projector: Vec<Header>,
    sidecars: BTreeMap<String, String>,
}

#[derive(Serialize, Deserialize)]
pub(super) enum Outcome {
    Compatible {
        memory: MemoryForecast,
        assumptions: Vec<String>,
        chat_template: bool,
    },
    Incompatible(Vec<String>),
    Unverified(String),
}

impl Input {
    fn encode(metadata: ArtifactMetadata) -> Result<Self> {
        fn gguf(headers: Vec<GgufHeader>) -> Vec<Header> {
            headers
                .into_iter()
                .map(|h| Header {
                    member: h.member,
                    bytes: STANDARD.encode(h.bytes),
                    file_len: h.file_len,
                })
                .collect()
        }
        let (config, index, headers, projector) = match metadata.checkpoint {
            CheckpointMetadata::SafeTensors {
                config,
                index,
                headers,
            } => (
                Some(STANDARD.encode(config)),
                index.map(|bytes| STANDARD.encode(bytes)),
                headers
                    .into_iter()
                    .map(|h| Header {
                        member: h.member,
                        bytes: STANDARD.encode(h.bytes),
                        file_len: h.file_len,
                    })
                    .collect(),
                vec![],
            ),
            CheckpointMetadata::Gguf {
                headers,
                companions,
            } => {
                let mut projector = Vec::new();
                for companion in companions {
                    ensure!(
                        companion.role == GgufCompanionRole::MediaProjector && projector.is_empty(),
                        "Unexpected catalogue companion"
                    );
                    projector = gguf(companion.headers);
                }
                (None, None, gguf(headers), projector)
            }
        };
        Ok(Self {
            provenance: metadata.provenance,
            config,
            index,
            headers,
            projector,
            sidecars: metadata
                .sidecars
                .into_iter()
                .map(|(key, bytes)| (key, STANDARD.encode(bytes)))
                .collect(),
        })
    }

    fn decode(self) -> Result<ArtifactMetadata> {
        fn gguf(headers: Vec<Header>) -> Result<Vec<GgufHeader>> {
            headers
                .into_iter()
                .map(|h| {
                    Ok(GgufHeader {
                        member: h.member,
                        bytes: STANDARD.decode(h.bytes)?,
                        file_len: h.file_len,
                    })
                })
                .collect()
        }
        let checkpoint = match self.config {
            Some(config) => CheckpointMetadata::SafeTensors {
                config: STANDARD.decode(config)?,
                index: self.index.map(|bytes| STANDARD.decode(bytes)).transpose()?,
                headers: self
                    .headers
                    .into_iter()
                    .map(|h| {
                        Ok(SafetensorsHeader {
                            member: h.member,
                            bytes: STANDARD.decode(h.bytes)?,
                            file_len: h.file_len,
                        })
                    })
                    .collect::<Result<_>>()?,
            },
            None => CheckpointMetadata::Gguf {
                headers: gguf(self.headers)?,
                companions: if self.projector.is_empty() {
                    vec![]
                } else {
                    vec![GgufCompanionMetadata {
                        role: GgufCompanionRole::MediaProjector,
                        headers: gguf(self.projector)?,
                    }]
                },
            },
        };
        Ok(ArtifactMetadata {
            provenance: self.provenance,
            checkpoint,
            sidecars: self
                .sidecars
                .into_iter()
                .map(|(key, bytes)| Ok((key, STANDARD.decode(bytes)?)))
                .collect::<Result<_>>()?,
        })
    }
}

pub(super) async fn inspect(metadata: Result<ArtifactMetadata, Vec<String>>) -> Result<Outcome> {
    let metadata = match metadata {
        Ok(metadata) => metadata,
        Err(reasons) => return Ok(Outcome::Incompatible(reasons)),
    };
    let _permit = WORKERS.acquire().await?;
    tokio::task::spawn_blocking(move || {
        let input = Input::encode(metadata)?;
        let mut child = Command::new(std::env::current_exe()?)
            .arg("--inspect-metadata")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()?;
        let written =
            serde_json::to_writer(child.stdin.take().context("Missing worker stdin")?, &input);
        drop(input);
        let output = child.wait_with_output()?;
        if !output.status.success() {
            return Err(std::io::Error::other(format!(
                "Catalogue inspection worker exited with {}",
                output.status
            ))
            .into());
        }
        written?;
        Ok(serde_json::from_slice(&output.stdout).map_err(std::io::Error::other)?)
    })
    .await?
}

/// Internal CLI worker: no network, devices, weights or persistent inspection state.
pub fn run_worker() -> Result<()> {
    let input: Input = serde_json::from_reader(std::io::stdin().lock())?;
    let outcome = match forecast::inspect_metadata(Ok(input.decode()?)) {
        Ok(CachedInspection::Compatible(forecast, chat_template)) => {
            ensure!(
                forecast.request.domains.len() == 1
                    && forecast.request.domains[0].domain == eredu::api::MemoryDomain::Unified
                    && forecast.request.domains[0].already_resident_bytes == 0,
                "Catalogue requires a cold unified-memory forecast"
            );
            let peak = &forecast.estimate.domains[0].overall_peak;
            let memory = MemoryForecast {
                lower_bytes: peak.lower_bytes,
                upper_bytes: peak.upper_bytes,
                uncertainties: forecast.estimate.uncertainties.clone(),
            };
            verify_memory_fit(&forecast.request, &memory)?;
            Outcome::Compatible {
                memory,
                assumptions: forecast.estimate.assumptions.clone(),
                chat_template,
            }
        }
        Ok(CachedInspection::Incompatible(reasons)) => Outcome::Incompatible(reasons),
        Err(error) => Outcome::Unverified(format!("{error:#}")),
    };
    serde_json::to_writer(std::io::stdout().lock(), &outcome)?;
    Ok(())
}
