package io.github.aaif_goose

import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.flow

public fun Provider.streamFlow(
    model: ProviderModelConfig,
    system: String,
    messages: List<ProviderMessage>,
): Flow<ProviderStreamChunk> = flow {
    val stream = stream(model, system, messages)
    while (true) {
        emit(stream.next() ?: break)
    }
}
