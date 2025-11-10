#[macro_export]
macro_rules! impl_provider_default {
    ($provider:ty) => {
        impl Default for $provider {
            fn default() -> Self {
                let model = $crate::model::ModelConfig::new(
                    &<$provider as $crate::providers::base::Provider>::metadata().default_model,
                )
                .expect(concat!(
                    "Failed to create model config for ",
                    stringify!($provider)
                ));

                <$provider>::from_env(model)
                    .expect(concat!("Failed to initialize ", stringify!($provider)))
            }
        }
    };
}
