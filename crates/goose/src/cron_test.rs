#[cfg(test)]
mod cron_parsing_tests {
    use crate::scheduler::normalize_cron_expression;
    use tokio_cron_scheduler::Job;

    // Helper: drop the last field if we have 7 so tokio_cron_scheduler (6-field) can parse
    fn to_tokio_spec(spec: &str) -> String {
        let parts: Vec<&str> = spec.split_whitespace().collect();
        if parts.len() == 7 {
            parts[..6].join(" ")
        } else {
            spec.to_string()
        }
    }

    #[test]
    fn test_normalize_cron_expression() {
        // 5-field → 7-field
        assert_eq!(normalize_cron_expression("0 12 * * *"), "0 0 12 * * * *");
        assert_eq!(normalize_cron_expression("*/5 * * * *"), "0 */5 * * * * *");
        assert_eq!(normalize_cron_expression("0 0 * * 1"), "0 0 0 * * 1 *");

        // 6-field → 7-field (append *)
        assert_eq!(normalize_cron_expression("0 0 12 * * *"), "0 0 12 * * * *");
        assert_eq!(
            normalize_cron_expression("*/30 */5 * * * *"),
            "*/30 */5 * * * * *"
        );

        // Weekday expressions (unchanged apart from 7-field format)
        assert_eq!(normalize_cron_expression("0 * * * 1-5"), "0 0 * * * 1-5 *");
        assert_eq!(
            normalize_cron_expression("*/20 * * * 1-5"),
            "0 */20 * * * 1-5 *"
        );
    }

    #[tokio::test]
    async fn test_cron_expression_formats() {
        let samples = [
            "0 0 * * *",   // 5-field
            "0 0 0 * * *", // 6-field
            "*/5 * * * *", // 5-field
        ];
        for expr in samples {
            let norm = normalize_cron_expression(expr);
            let tokio_spec = to_tokio_spec(&norm);
            assert!(
                Job::new_async(&tokio_spec, |_id, _l| Box::pin(async {})).is_ok(),
                "failed to parse {} -> {}",
                expr,
                norm
            );
        }
    }
}
