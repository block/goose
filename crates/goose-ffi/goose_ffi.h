#ifndef GOOSE_FFI_H
#define GOOSE_FFI_H

#include <stdint.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

/**
 * Initialize the Goose agent.
 * Must be called before any other functions.
 * 
 * @param token The authentication token for the Databricks API
 * @return true if initialization was successful, false otherwise
 */
bool goose_initialize(const char* token);

/**
 * Send a message to the Goose agent and get a response.
 * 
 * @param message The message to send
 * @param token The authentication token for the Databricks API
 * @return A JSON string with the response. Must be freed with goose_free_string
 */
char* goose_send_message(const char* message, const char* token);

/**
 * Free a string returned by goose_send_message.
 * 
 * @param str The string to free
 */
void goose_free_string(char* str);

/**
 * Shut down the Goose agent.
 * 
 * @return true if shutdown was successful, false otherwise
 */
bool goose_shutdown(void);

#ifdef __cplusplus
}
#endif

#endif /* GOOSE_FFI_H */