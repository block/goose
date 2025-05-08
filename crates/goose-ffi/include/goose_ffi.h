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
 Role enum for message participants
 */
enum goose_MessageRole {
  /*
   User message role
   */
  goose_MessageRole_User = 0,
  /*
   Assistant message role
   */
  goose_MessageRole_Assistant = 1,
  /*
   System message role
   */
  goose_MessageRole_System = 2,
};
typedef uint32_t goose_MessageRole;

/*
 Completion response structure

 - content: JSON string containing the completion response
 - succeeded: true if the operation succeeded, false otherwise
 - error_message: Error message if succeeded is false, NULL otherwise
 */
typedef struct goose_CompletionResponseFFI {
  char *content;
  bool succeeded;
  char *error_message;
} goose_CompletionResponseFFI;

/*
 Message structure for agent interactions

 - role: Message role (User, Assistant, or System)
 - content: Text content of the message
 */
typedef struct goose_MessageFFI {
  goose_MessageRole role;
  const char *content;
} goose_MessageFFI;

/*
 Tool definition for use with completion

 - name: Tool name
 - description: Tool description
 - input_schema_json: JSON schema for the tool's input parameters
 */
typedef struct goose_ToolFFI {
  const char *name;
  const char *description;
  const char *input_schema_json;
} goose_ToolFFI;

/*
 Extension definition for use with completion

 - name: Extension name
 - instructions: Optional instructions for the extension (can be NULL)
 - tools: Array of ToolFFI structures
 - tool_count: Number of tools in the array
 */
typedef struct goose_ExtensionFFI {
  const char *name;
  const char *instructions;
  const struct goose_ToolFFI *tools;
  uintptr_t tool_count;
} goose_ExtensionFFI;

/*
 Free a completion response structure

 This function frees the memory allocated for a CompletionResponseFFI structure,
 including any content and error message it contains.

 # Safety

 The response pointer must be a valid pointer returned by a goose FFI function,
 or NULL.
 */
void goose_free_completion_response(struct goose_CompletionResponseFFI *response);

/*
 Perform a completion request

 This function sends a completion request to the specified provider and returns
 the response.

 # Parameters

 - provider: Provider name (e.g., "databricks", "anthropic")
 - model_name: Model name to use
 - host: Provider host URL (NULL for default from environment variables)
 - api_key: Provider API key (NULL for default from environment variables)
 - system_preamble: System preamble text
 - messages: Array of MessageFFI structures
 - message_count: Number of messages in the array
 - extensions: Array of ExtensionFFI structures
 - extension_count: Number of extensions in the array

 # Returns

 A CompletionResponseFFI structure containing the response or error.
 This must be freed with goose_free_completion_response when no longer needed.

 # Safety

 All string parameters must be valid C strings or NULL.
 The messages array must contain valid MessageFFI structures.
 The extensions array must contain valid ExtensionFFI structures.
 */
struct goose_CompletionResponseFFI *goose_completion(const char *provider,
                                                     const char *model_name,
                                                     const char *host,
                                                     const char *api_key,
                                                     const char *system_preamble,
                                                     const struct goose_MessageFFI *messages_ptr,
                                                     uintptr_t message_count,
                                                     const struct goose_ExtensionFFI *extensions_ptr,
                                                     uintptr_t extension_count);

#endif // GOOSE_FFI_H
