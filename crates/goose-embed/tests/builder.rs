//! Smoke tests for the builder validation surface.
//!
//! These tests intentionally do not touch the network or instantiate a real
//! provider — they just check that the builder rejects obviously-broken
//! configurations with helpful error messages.

use goose_embed::prelude::*;

#[tokio::test]
async fn build_without_provider_or_recipe_errors() {
    let result = Goose::builder().build().await;
    let err = match result {
        Ok(_) => panic!("expected builder to reject missing provider"),
        Err(e) => e,
    };
    let msg = format!("{err:#}");
    assert!(
        msg.contains("provider"),
        "error should mention provider, got: {msg}"
    );
}

#[tokio::test]
async fn build_without_model_errors() {
    let recipe = Recipe {
        version: "1.0.0".to_string(),
        title: "no model".to_string(),
        description: "recipe without a model setting".to_string(),
        instructions: Some("hello".to_string()),
        prompt: None,
        extensions: None,
        settings: None,
        activities: None,
        author: None,
        parameters: None,
        response: None,
        sub_recipes: None,
        retry: None,
    };
    let result = Goose::builder().recipe(recipe).build().await;
    let err = match result {
        Ok(_) => panic!("expected builder to reject missing model"),
        Err(e) => e,
    };
    let msg = format!("{err:#}");
    assert!(
        msg.contains("provider") || msg.contains("model"),
        "error should mention provider or model, got: {msg}"
    );
}
