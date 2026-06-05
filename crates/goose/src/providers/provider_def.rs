use anyhow::Result;
use futures::future::BoxFuture;
use std::path::PathBuf;

use super::inventory::{
    default_inventory_configured, default_inventory_identity, InventoryIdentityInput,
};
use crate::config::{Config, ExtensionConfig};
use crate::model::ModelConfig;

use super::base::ProviderMetadata;
use super::mode::GooseProvider;

pub trait ProviderDef: Send + Sync {
    type Provider: GooseProvider + 'static;

    fn metadata() -> ProviderMetadata
    where
        Self: Sized;

    fn from_env(
        model: ModelConfig,
        extensions: Vec<ExtensionConfig>,
    ) -> BoxFuture<'static, Result<Self::Provider>>
    where
        Self: Sized;

    fn from_env_with_working_dir(
        model: ModelConfig,
        extensions: Vec<ExtensionConfig>,
        _working_dir: PathBuf,
    ) -> BoxFuture<'static, Result<Self::Provider>>
    where
        Self: Sized,
    {
        Self::from_env(model, extensions)
    }

    fn supports_inventory_refresh() -> bool
    where
        Self: Sized,
    {
        false
    }

    fn inventory_identity() -> Result<InventoryIdentityInput>
    where
        Self: Sized,
    {
        let metadata = Self::metadata();
        Ok(default_inventory_identity(
            &metadata.name,
            &metadata.name,
            &metadata.config_keys,
            Config::global(),
        ))
    }

    fn inventory_configured() -> bool
    where
        Self: Sized,
    {
        let metadata = Self::metadata();
        default_inventory_configured(&metadata.config_keys, Config::global())
    }
}
