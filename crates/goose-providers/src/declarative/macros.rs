macro_rules! expose_declarative_provider {
    ($module:ident, $definition:expr) => {
        pub mod $module {
            use anyhow::Result;

            use crate::{
                api_client::TlsConfig,
                base::Provider,
                declarative::{from_json, KeyResolver},
            };

            pub const JSON: &str = include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/src/declarative/definitions/",
                $definition,
                ".json"
            ));

            pub fn create(
                tls_config: Option<TlsConfig>,
                key_resolver: impl KeyResolver,
            ) -> Result<Box<dyn Provider>> {
                from_json(JSON, tls_config, key_resolver)
            }
        }
    };
}

macro_rules! expose_declarative_providers {
    ($($module:ident),+ $(,)?) => {
        $(expose_declarative_provider!($module, stringify!($module));)+

        pub fn fixed_provider_configs() -> anyhow::Result<Vec<DeclarativeProviderConfig>> {
            [$($module::JSON),+]
                .into_iter()
                .map(config_from_json)
                .collect()
        }
    };
}
