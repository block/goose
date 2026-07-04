//! Gosling-custom **agent → client** requests: server-initiated JSON-RPC requests
//! that expect a response from the client (unlike notifications, which are
//! fire-and-forget). This module aggregates their JSON schemas for the ACP
//! schema generator, parallel to `custom_notification_schemas`.
//!
//! To expose a new agent → client request in the generated schema, define its
//! params/response types (deriving `JsonSchema`) next to the feature that sends
//! it, then add one line to [`agent_request_schemas`].

use schemars::{JsonSchema, SchemaGenerator};

use crate::acp::custom_requests::CustomMethodSchema;

fn short_type_name<T>() -> String {
    let full = std::any::type_name::<T>();
    full.rsplit("::").next().unwrap_or(full).to_string()
}

/// Schema descriptor for a single agent → client request. Unlike notification
/// descriptors, request descriptors include both params and response types.
#[allow(dead_code)]
fn agent_request_schema<Req, Resp>(
    generator: &mut SchemaGenerator,
    method: &str,
) -> CustomMethodSchema
where
    Req: JsonSchema,
    Resp: JsonSchema,
{
    CustomMethodSchema {
        method: method.to_string(),
        params_schema: Some(generator.subschema_for::<Req>()),
        params_type_name: Some(short_type_name::<Req>()),
        response_schema: Some(generator.subschema_for::<Resp>()),
        response_type_name: Some(short_type_name::<Resp>()),
    }
}

/// Schemas for every gosling-custom agent → client request. Collected by the ACP
/// schema generator binary.
pub fn agent_request_schemas(_generator: &mut SchemaGenerator) -> Vec<CustomMethodSchema> {
    Vec::new()
}
