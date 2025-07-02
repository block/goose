#[cfg(test)]
mod tests {
    use crate::oauth::ServiceConfig;

    #[test]
    fn test_canonical_resource_uri_generation() {
        let config = ServiceConfig {
            oauth_host: "https://example.com".to_string(),
            redirect_uri: "http://localhost:8020".to_string(),
            client_name: "Test Client".to_string(),
            client_uri: "https://test.com".to_string(),
            discovery_path: None,
        };

        // Test basic URL
        let result = config
            .get_canonical_resource_uri("https://mcp.example.com/mcp")
            .unwrap();
        assert_eq!(result, "https://mcp.example.com/mcp");

        // Test URL with port
        let result = config
            .get_canonical_resource_uri("https://mcp.example.com:8443/mcp")
            .unwrap();
        assert_eq!(result, "https://mcp.example.com:8443/mcp");

        // Test URL without path
        let result = config
            .get_canonical_resource_uri("https://mcp.example.com")
            .unwrap();
        assert_eq!(result, "https://mcp.example.com");

        // Test URL with root path
        let result = config
            .get_canonical_resource_uri("https://mcp.example.com/")
            .unwrap();
        assert_eq!(result, "https://mcp.example.com");

        // Test case normalization
        let result = config
            .get_canonical_resource_uri("HTTPS://MCP.EXAMPLE.COM/mcp")
            .unwrap();
        assert_eq!(result, "https://mcp.example.com/mcp");
    }

    #[test]
    fn test_service_config_from_mcp_endpoint() {
        let config = ServiceConfig::from_mcp_endpoint("https://mcp.example.com/api/mcp").unwrap();

        assert_eq!(config.oauth_host, "https://mcp.example.com");
        assert_eq!(config.redirect_uri, "http://localhost:8020");
        assert_eq!(config.client_name, "Goose MCP Client");
        assert_eq!(config.client_uri, "https://github.com/block/goose");
        assert!(config.discovery_path.is_none());
    }

    #[test]
    fn test_service_config_with_port() {
        let config = ServiceConfig::from_mcp_endpoint("https://mcp.example.com:8443/mcp").unwrap();

        assert_eq!(config.oauth_host, "https://mcp.example.com:8443");
    }

    #[test]
    fn test_service_config_invalid_url() {
        let result = ServiceConfig::from_mcp_endpoint("invalid-url");
        assert!(result.is_err());
    }

    #[test]
    fn test_custom_discovery_path() {
        let config = ServiceConfig::from_mcp_endpoint("https://mcp.example.com/mcp")
            .unwrap()
            .with_custom_discovery("/custom/oauth/discovery".to_string());

        assert_eq!(
            config.discovery_path,
            Some("/custom/oauth/discovery".to_string())
        );
    }
}
