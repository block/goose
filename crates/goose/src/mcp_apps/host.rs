//! MCP Apps Host Context Provider
//!
//! This module provides the host context that is sent to MCP Apps UI iframes
//! during initialization. The context includes theme, viewport, platform info, etc.

use super::types::{
    DeviceCapabilities, DisplayMode, HostCapabilities, HostContext, HostInfo,
    McpUiInitializeResult, Platform, Theme, Viewport,
};
use super::MIME_TYPE;

/// Protocol version for MCP Apps
/// This should match the SEP-1865 specification version
pub const PROTOCOL_VERSION: &str = "2025-06-18";

/// Builder for creating host context
#[derive(Debug, Default)]
pub struct HostContextBuilder {
    theme: Option<Theme>,
    display_mode: Option<DisplayMode>,
    viewport: Option<Viewport>,
    locale: Option<String>,
    time_zone: Option<String>,
    platform: Option<Platform>,
}

impl HostContextBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn theme(mut self, theme: Theme) -> Self {
        self.theme = Some(theme);
        self
    }

    pub fn display_mode(mut self, mode: DisplayMode) -> Self {
        self.display_mode = Some(mode);
        self
    }

    pub fn viewport(mut self, width: u32, height: u32) -> Self {
        self.viewport = Some(Viewport {
            width,
            height,
            max_width: None,
            max_height: None,
        });
        self
    }

    pub fn viewport_with_max(
        mut self,
        width: u32,
        height: u32,
        max_width: Option<u32>,
        max_height: Option<u32>,
    ) -> Self {
        self.viewport = Some(Viewport {
            width,
            height,
            max_width,
            max_height,
        });
        self
    }

    pub fn locale(mut self, locale: impl Into<String>) -> Self {
        self.locale = Some(locale.into());
        self
    }

    pub fn time_zone(mut self, tz: impl Into<String>) -> Self {
        self.time_zone = Some(tz.into());
        self
    }

    pub fn platform(mut self, platform: Platform) -> Self {
        self.platform = Some(platform);
        self
    }

    pub fn build(self) -> HostContext {
        HostContext {
            theme: self.theme,
            display_mode: self.display_mode,
            available_display_modes: vec![DisplayMode::Inline, DisplayMode::Fullscreen],
            viewport: self.viewport,
            locale: self.locale,
            time_zone: self.time_zone,
            user_agent: Some(format!("goose/{}", env!("CARGO_PKG_VERSION"))),
            platform: self.platform,
            device_capabilities: Some(DeviceCapabilities {
                touch: Some(false),
                hover: Some(true),
            }),
            safe_area_insets: None,
        }
    }
}

/// Create the default host capabilities for Goose
pub fn default_host_capabilities() -> HostCapabilities {
    HostCapabilities {
        mime_types: vec![MIME_TYPE.to_string(), "text/html".to_string()],
        open_link: Some(true),
        message: Some(true),
    }
}

/// Create the host info for Goose
pub fn host_info() -> HostInfo {
    HostInfo {
        name: "goose".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    }
}

/// Create a complete MCP UI initialize result
///
/// This is the response sent to UI iframes when they send an `initialize` request.
pub fn create_initialize_result(context: HostContext) -> McpUiInitializeResult {
    McpUiInitializeResult {
        protocol_version: PROTOCOL_VERSION.to_string(),
        host_capabilities: Some(default_host_capabilities()),
        host_info: Some(host_info()),
        host_context: Some(context),
    }
}

/// Create a default host context for desktop platform
pub fn default_desktop_context() -> HostContext {
    HostContextBuilder::new()
        .theme(Theme::Light)
        .display_mode(DisplayMode::Inline)
        .platform(Platform::Desktop)
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_host_context_builder() {
        let context = HostContextBuilder::new()
            .theme(Theme::Dark)
            .display_mode(DisplayMode::Inline)
            .viewport(400, 300)
            .locale("en-US")
            .time_zone("America/New_York")
            .platform(Platform::Desktop)
            .build();

        assert_eq!(context.theme, Some(Theme::Dark));
        assert_eq!(context.display_mode, Some(DisplayMode::Inline));
        assert_eq!(context.viewport.as_ref().unwrap().width, 400);
        assert_eq!(context.viewport.as_ref().unwrap().height, 300);
        assert_eq!(context.locale, Some("en-US".to_string()));
        assert_eq!(context.time_zone, Some("America/New_York".to_string()));
        assert_eq!(context.platform, Some(Platform::Desktop));
        assert!(context.user_agent.is_some());
    }

    #[test]
    fn test_default_host_capabilities() {
        let caps = default_host_capabilities();
        assert!(caps.mime_types.contains(&MIME_TYPE.to_string()));
        assert!(caps.mime_types.contains(&"text/html".to_string()));
        assert_eq!(caps.open_link, Some(true));
        assert_eq!(caps.message, Some(true));
    }

    #[test]
    fn test_host_info() {
        let info = host_info();
        assert_eq!(info.name, "goose");
        assert!(!info.version.is_empty());
    }

    #[test]
    fn test_create_initialize_result() {
        let context = default_desktop_context();
        let result = create_initialize_result(context);

        assert_eq!(result.protocol_version, PROTOCOL_VERSION);
        assert!(result.host_capabilities.is_some());
        assert!(result.host_info.is_some());
        assert!(result.host_context.is_some());

        let host_info = result.host_info.unwrap();
        assert_eq!(host_info.name, "goose");
    }

    #[test]
    fn test_initialize_result_serialization() {
        let context = HostContextBuilder::new()
            .theme(Theme::Dark)
            .display_mode(DisplayMode::Inline)
            .viewport(400, 300)
            .build();

        let result = create_initialize_result(context);
        let json = serde_json::to_value(&result).unwrap();

        assert_eq!(json["protocolVersion"], PROTOCOL_VERSION);
        assert_eq!(json["hostInfo"]["name"], "goose");
        assert_eq!(json["hostContext"]["theme"], "dark");
        assert_eq!(json["hostContext"]["displayMode"], "inline");
        assert_eq!(json["hostContext"]["viewport"]["width"], 400);
    }
}
