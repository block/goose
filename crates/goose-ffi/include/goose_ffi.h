#ifndef GOOSE_FFI_H
#define GOOSE_FFI_H

/* Goose FFI - C interface for the Goose AI agent framework */


#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>
#include <stdint.h>
#include <stdbool.h>

/*
 Result type for async operations

 - succeeded: true if the operation succeeded, false otherwise
 - error_message: Error message if succeeded is false, NULL otherwise
 */
typedef struct goose_AsyncResult {
  bool succeeded;
  char *error_message;
} goose_AsyncResult;

/*
 Opaque pointer to Agent
 */
typedef struct goose_AgentPtr {
  goose_Agent *_0;
} goose_AgentPtr;

/*
 Provider configuration used to initialize an AI provider

 - provider_type: Provider type (0 = Databricks, other values will produce an error)
 - api_key: Provider API key (null for default from environment variables)
 - model_name: Model name to use (null for provider default)
 - host: Provider host URL (null for default from environment variables)
 */
typedef struct goose_ProviderConfigFFI {
  uint32_t provider_type;
  const char *api_key;
  const char *model_name;
  const char *host;
} goose_ProviderConfigFFI;

/*
 Free an async result structure

 This function frees the memory allocated for an AsyncResult structure,
 including any error message it contains.

 # Safety

 The result pointer must be a valid pointer returned by a goose FFI function,
 or NULL.
 */
void goose_free_async_result(struct goose_AsyncResult *result);

/*
 Create a new agent with the given provider configuration

 # Parameters

 - config: Provider configuration

 # Returns

 A new agent pointer, or a null pointer if creation failed

 # Safety

 The config pointer must be valid or NULL. The resulting agent must be freed
 with goose_agent_free when no longer needed.
 */
struct goose_AgentPtr goose_agent_new(const struct goose_ProviderConfigFFI *config);

/*
 Free an agent

 This function frees the memory allocated for an agent.

 # Parameters

 - agent_ptr: Agent pointer returned by goose_agent_new

 # Safety

 The agent_ptr must be a valid pointer returned by goose_agent_new,
 or have a null internal pointer. The agent_ptr must not be used after
 calling this function.
 */
void goose_agent_free(struct goose_AgentPtr agent_ptr);

/*
 Send a message to the agent and get the response

 This function sends a message to the agent and returns the response.

 # Parameters

 - agent_ptr: Agent pointer
 - message: Message to send

 # Returns

 A C string with the agent's response, or NULL on error.
 This string must be freed with goose_free_string when no longer needed.

 # Safety

 The agent_ptr must be a valid pointer returned by goose_agent_new.
 The message must be a valid C string.
 */
char *goose_agent_send_message(struct goose_AgentPtr agent_ptr, const char *message);

/*
 Free a string allocated by goose FFI functions

 This function frees memory allocated for strings returned by goose FFI functions.

 # Parameters

 - s: String to free

 # Safety

 The string must have been allocated by a goose FFI function, or be NULL.
 The string must not be used after calling this function.
 */
void goose_free_string(char *s);

#endif // GOOSE_FFI_H
