// Example of AI/ML API authorization-code + PKCE authentication.
//
// Run with: cargo run --example aimlapi_auth
//
// Requires AIMLAPI_PARTNER_ID. To exercise a non-production environment, also
// set AIMLAPI_APP_URL (the API host) and AIMLAPI_WEB_URL (the consent screen).

use goose::config::signup_aimlapi::AimlapiAuth;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing AI/ML API PKCE flow...\n");

    let mut auth_flow = AimlapiAuth::new()?;

    println!("Starting authentication flow...");
    println!("This will:");
    println!("1. Register an authorization request carrying the PKCE challenge");
    println!("2. Open your browser to the consent screen");
    println!("3. Wait for the redirect back to the loopback listener");
    println!("4. Exchange the one-time code plus the verifier for an api-key\n");

    match auth_flow.complete_flow().await {
        Ok(api_key) => {
            println!("\nAuthentication successful.");
            println!(
                "API key received: {}...",
                &api_key.chars().take(10).collect::<String>()
            );
            println!("\nYou can now use this key with the aimlapi provider.");
        }
        Err(e) => {
            eprintln!("\nAuthentication failed: {}", e);
            eprintln!("Error details: {:?}", e);
        }
    }

    Ok(())
}
