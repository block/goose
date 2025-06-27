#[cfg(test)]
mod cron_parsing_tests {
    use crate::scheduler::normalize_cron_expression;
    use tokio_cron_scheduler::Job;

    #[test]
    fn test_normalize_cron_expression() {
        // Test 5-field to 6-field conversion
        assert_eq!(normalize_cron_expression("0 12 * * *"), "0 0 12 * * *");
        assert_eq!(normalize_cron_expression("*/5 * * * *"), "0 */5 * * * *");
        assert_eq!(normalize_cron_expression("0 0 * * 1"), "0 0 0 * * 1");

        // Test 6-field expressions (should remain unchanged)
        assert_eq!(normalize_cron_expression("0 0 12 * * *"), "0 0 12 * * *");
        assert_eq!(
            normalize_cron_expression("*/30 */5 * * * *"),
            "*/30 */5 * * * *"
        );

        // Test invalid expressions (should remain unchanged but warn)
        assert_eq!(normalize_cron_expression("* * *"), "* * *");
        assert_eq!(normalize_cron_expression("* * * * * * *"), "* * * * * * *");
        assert_eq!(normalize_cron_expression(""), "");
    }

    #[tokio::test]
    async fn test_cron_expression_formats() {
        // Test different cron formats to see which ones work
        let test_expressions = vec![
            ("0 0 * * *", "5-field: every day at midnight"),
            ("0 0 0 * * *", "6-field: every day at midnight"),
            ("* * * * *", "5-field: every minute"),
            ("* * * * * *", "6-field: every second"),
            ("0 */5 * * *", "5-field: every 5 minutes"),
            ("0 0 */5 * * *", "6-field: every 5 minutes"),
            ("0 0 12 * * *", "6-field: every day at noon"),
            ("0 12 * * *", "5-field: every day at noon"),
        ];

        for (expr, desc) in test_expressions {
            println!("Testing cron expression: '{}' ({})", expr, desc);
            let expr_owned = expr.to_string();

            // Test with normalization
            let normalized = normalize_cron_expression(expr);
            println!("  Normalized to: '{}'", normalized);

            match Job::new_async(&normalized, move |_uuid, _l| {
                let expr_clone = expr_owned.clone();
                Box::pin(async move {
                    println!("Job executed for: {}", expr_clone);
                })
            }) {
                Ok(_) => println!("  ✅ Successfully parsed normalized: '{}'", normalized),
                Err(e) => println!("  ❌ Failed to parse normalized '{}': {}", normalized, e),
            }
            println!();
        }
    }
}
