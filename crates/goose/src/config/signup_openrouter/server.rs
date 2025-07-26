use anyhow::Result;
use axum::{
    extract::Query,
    http::StatusCode,
    response::{Html, IntoResponse},
    routing::get,
    Router,
};
use serde::Deserialize;
use std::net::SocketAddr;
use tokio::sync::oneshot;

#[derive(Debug, Deserialize)]
struct CallbackQuery {
    code: Option<String>,
    error: Option<String>,
}

/// Run the callback server on localhost:3000
pub async fn run_callback_server(
    code_tx: oneshot::Sender<String>,
    shutdown_rx: oneshot::Receiver<()>,
) -> Result<()> {
    let app = Router::new().route("/", get(handle_callback));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));

    let listener = tokio::net::TcpListener::bind(addr).await?;

    // Wrap the code_tx in an Arc<Mutex> so we can use it in the handler
    let state = std::sync::Arc::new(tokio::sync::Mutex::new(Some(code_tx)));

    axum::serve(listener, app.with_state(state.clone()).into_make_service())
        .with_graceful_shutdown(async move {
            let _ = shutdown_rx.await;
        })
        .await?;

    Ok(())
}

async fn handle_callback(
    Query(params): Query<CallbackQuery>,
    state: axum::extract::State<
        std::sync::Arc<tokio::sync::Mutex<Option<oneshot::Sender<String>>>>,
    >,
) -> impl IntoResponse {
    // Check for error first
    if let Some(error) = params.error {
        return (
            StatusCode::BAD_REQUEST,
            Html(format!(
                r#"
                <!DOCTYPE html>
                <html>
                <head>
                    <title>Authentication Failed</title>
                    <style>
                        body {{
                            font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
                            display: flex;
                            justify-content: center;
                            align-items: center;
                            height: 100vh;
                            margin: 0;
                            background-color: #f5f5f5;
                        }}
                        .container {{
                            text-align: center;
                            padding: 40px;
                            background: white;
                            border-radius: 8px;
                            box-shadow: 0 2px 10px rgba(0,0,0,0.1);
                            max-width: 500px;
                        }}
                        h1 {{
                            color: #d32f2f;
                            margin-bottom: 20px;
                        }}
                        p {{
                            color: #666;
                            line-height: 1.6;
                        }}
                        .error {{
                            background-color: #ffebee;
                            padding: 10px;
                            border-radius: 4px;
                            margin-top: 20px;
                            color: #c62828;
                            font-family: monospace;
                            font-size: 14px;
                        }}
                    </style>
                </head>
                <body>
                    <div class="container">
                        <h1>❌ Authentication Failed</h1>
                        <p>There was an error during the authentication process.</p>
                        <div class="error">{}</div>
                        <p>Please close this tab and try again.</p>
                    </div>
                </body>
                </html>
                "#,
                html_escape::encode_text(&error)
            )),
        );
    }

    // Extract the code
    if let Some(code) = params.code {
        // Send the code through the channel
        let mut tx_guard = state.lock().await;
        if let Some(tx) = tx_guard.take() {
            let _ = tx.send(code);
        }

        return (
            StatusCode::OK,
            Html(r#"
                <!DOCTYPE html>
                <html>
                <head>
                    <title>Authentication Successful</title>
                    <style>
                        body {
                            font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
                            display: flex;
                            justify-content: center;
                            align-items: center;
                            height: 100vh;
                            margin: 0;
                            background-color: #f5f5f5;
                        }
                        .container {
                            text-align: center;
                            padding: 40px;
                            background: white;
                            border-radius: 8px;
                            box-shadow: 0 2px 10px rgba(0,0,0,0.1);
                            max-width: 500px;
                        }
                        h1 {
                            color: #4caf50;
                            margin-bottom: 20px;
                        }
                        p {
                            color: #666;
                            line-height: 1.6;
                        }
                        .checkmark {
                            font-size: 48px;
                            margin-bottom: 20px;
                        }
                    </style>
                </head>
                <body>
                    <div class="container">
                        <div class="checkmark">✅</div>
                        <h1>Authentication Successful!</h1>
                        <p>You have successfully authenticated with OpenRouter.</p>
                        <p>You can now close this tab and return to your terminal.</p>
                    </div>
                </body>
                </html>
            "#.to_string()),
        );
    }

    // No code parameter
    (
        StatusCode::BAD_REQUEST,
        Html(r#"
            <!DOCTYPE html>
            <html>
            <head>
                <title>Invalid Request</title>
                <style>
                    body {
                        font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
                        display: flex;
                        justify-content: center;
                        align-items: center;
                        height: 100vh;
                        margin: 0;
                        background-color: #f5f5f5;
                    }
                    .container {
                        text-align: center;
                        padding: 40px;
                        background: white;
                        border-radius: 8px;
                        box-shadow: 0 2px 10px rgba(0,0,0,0.1);
                        max-width: 500px;
                    }
                    h1 {
                        color: #ff9800;
                        margin-bottom: 20px;
                    }
                    p {
                        color: #666;
                    }
                </style>
            </head>
            <body>
                <div class="container">
                    <h1>⚠️ Invalid Request</h1>
                    <p>This doesn't appear to be a valid authentication callback.</p>
                    <p>Please close this tab and try the authentication process again.</p>
                </div>
            </body>
            </html>
        "#.to_string()),
    )
}
