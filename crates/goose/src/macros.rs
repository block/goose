#[macro_export]
macro_rules! impl_provider_default {
    ($provider:ty) => {
        impl Default for $provider {
            fn default() -> Self {
                let model = $crate::model::ModelConfig::new_or_fail(
                    &<$provider as $crate::providers::base::Provider>::metadata().default_model,
                );

                <$provider>::from_env(model)
                    .expect(concat!("Failed to initialize ", stringify!($provider)))
            }
        }
    };
}
