// This file provides direct access to provider pooling functionality

package uniffi.goose_llm

/**
 * Initialize the provider pool with default configuration
 */
fun initProviderPool() {
    uniffiRustCall { _status ->
        UniffiLib.INSTANCE.uniffi_goose_llm_fn_func_init_provider_pool(_status)
    }
}

/**
 * Configure the provider pool with custom settings
 * 
 * @param maxSize Maximum number of providers in the pool
 * @param maxIdleSec Maximum idle time in seconds before a provider is removed
 * @param maxLifetimeSec Maximum lifetime in seconds for a provider
 * @param maxUses Maximum number of uses for a provider
 */
fun configureProviderPool(
    maxSize: Int,
    maxIdleSec: Long,
    maxLifetimeSec: Long,
    maxUses: Int
) {
    uniffiRustCall { _status ->
        UniffiLib.INSTANCE.uniffi_goose_llm_fn_func_configure_provider_pool(
            maxSize.toUInt().toInt(),  // Convert to match Rust's u32
            maxIdleSec,
            maxLifetimeSec,
            maxUses.toUInt().toInt(),  // Convert to match Rust's u32
            _status
        )
    }
}

/**
 * Get statistics about the provider pool
 * 
 * @return A string representation of the pool statistics
 */
fun getPoolStats(): String {
    return FfiConverterString.lift(
        uniffiRustCall { _status ->
            UniffiLib.INSTANCE.uniffi_goose_llm_fn_func_get_pool_stats(_status)
        }
    )
}

/**
 * Extension function to create a completion request with pool options
 *
 * @param usePool Whether to use the connection pool
 */
fun createCompletionRequest(
    providerName: String,
    providerConfig: Value,
    modelConfig: ModelConfig,
    systemPreamble: String,
    messages: List<Message>,
    extensions: List<ExtensionConfig>,
    usePool: Boolean? = null
): CompletionRequest {
    return FfiConverterTypeCompletionRequest.lift(
        uniffiRustCall { _status ->
            UniffiLib.INSTANCE.uniffi_goose_llm_fn_func_create_completion_request_with_pool(
                FfiConverterString.lower(providerName),
                FfiConverterTypeValue.lower(providerConfig),
                FfiConverterTypeModelConfig.lower(modelConfig),
                FfiConverterString.lower(systemPreamble),
                FfiConverterSequenceTypeMessage.lower(messages),
                FfiConverterSequenceTypeExtensionConfig.lower(extensions),
                // Convert Boolean? to Byte? for FFI
                if (usePool == null) null else if (usePool) 1.toByte() else 0.toByte(),
                _status
            )
        }
    )
}