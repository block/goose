use chrono::Utc;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

use super::download_task::DownloadProgress;

pub struct ProgressTracker {
    downloaded_bytes: Arc<AtomicU64>,
    total_bytes: Arc<AtomicU64>,
    start_time: Instant,
    last_update_time: Arc<RwLock<Instant>>,
    speed_calculator: Arc<RwLock<SpeedCalculator>>,
    resumed_from: u64,
}

impl ProgressTracker {
    pub fn new(resumed_from: u64) -> Self {
        Self {
            downloaded_bytes: Arc::new(AtomicU64::new(resumed_from)),
            total_bytes: Arc::new(AtomicU64::new(0)),
            start_time: Instant::now(),
            last_update_time: Arc::new(RwLock::new(Instant::now())),
            speed_calculator: Arc::new(RwLock::new(SpeedCalculator::new())),
            resumed_from,
        }
    }

    pub async fn update(&self, downloaded: u64, total: Option<u64>) {
        self.downloaded_bytes.store(downloaded, Ordering::Relaxed);
        if let Some(t) = total {
            self.total_bytes.store(t, Ordering::Relaxed);
        }

        *self.last_update_time.write().await = Instant::now();
        self.speed_calculator
            .write()
            .await
            .add_sample(Instant::now(), downloaded);
    }

    pub async fn to_progress(&self) -> DownloadProgress {
        let downloaded = self.downloaded_bytes.load(Ordering::Relaxed);
        let total = self.total_bytes.load(Ordering::Relaxed);
        let total_opt = if total > 0 { Some(total) } else { None };

        let speed = self.speed_calculator.read().await.current_speed();
        let eta = if speed > 0.0 && total > downloaded {
            Some(((total - downloaded) as f64 / speed) as u64)
        } else {
            None
        };

        DownloadProgress {
            downloaded_bytes: downloaded,
            total_bytes: total_opt,
            speed_bytes_per_sec: speed,
            eta_seconds: eta,
            resumed_from_bytes: self.resumed_from,
            last_updated: Utc::now(),
        }
    }
}

pub struct SpeedCalculator {
    samples: VecDeque<(Instant, u64)>,
    window_size: Duration,
}

impl SpeedCalculator {
    pub fn new() -> Self {
        Self {
            samples: VecDeque::with_capacity(50),
            window_size: Duration::from_secs(5),
        }
    }

    pub fn add_sample(&mut self, time: Instant, bytes: u64) {
        self.samples.push_back((time, bytes));

        // Remove old samples outside window
        let cutoff = time - self.window_size;
        while let Some((t, _)) = self.samples.front() {
            if *t < cutoff {
                self.samples.pop_front();
            } else {
                break;
            }
        }
    }

    pub fn current_speed(&self) -> f64 {
        if self.samples.len() < 2 {
            return 0.0;
        }

        let (first_time, first_bytes) = self.samples.front().unwrap();
        let (last_time, last_bytes) = self.samples.back().unwrap();

        let duration = last_time.duration_since(*first_time).as_secs_f64();
        if duration > 0.0 {
            (*last_bytes as i64 - *first_bytes as i64).max(0) as f64 / duration
        } else {
            0.0
        }
    }

    pub fn clear(&mut self) {
        self.samples.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_progress_tracker_basic() {
        let tracker = ProgressTracker::new(0);

        tracker.update(100, Some(1000)).await;
        let progress = tracker.to_progress().await;

        assert_eq!(progress.downloaded_bytes, 100);
        assert_eq!(progress.total_bytes, Some(1000));
        assert_eq!(progress.resumed_from_bytes, 0);
    }

    #[tokio::test]
    async fn test_progress_tracker_with_resume() {
        let tracker = ProgressTracker::new(500);

        tracker.update(750, Some(1000)).await;
        let progress = tracker.to_progress().await;

        assert_eq!(progress.downloaded_bytes, 750);
        assert_eq!(progress.total_bytes, Some(1000));
        assert_eq!(progress.resumed_from_bytes, 500);
    }

    #[test]
    fn test_speed_calculator_no_samples() {
        let calc = SpeedCalculator::new();
        assert_eq!(calc.current_speed(), 0.0);
    }

    #[test]
    fn test_speed_calculator_one_sample() {
        let mut calc = SpeedCalculator::new();
        calc.add_sample(Instant::now(), 100);
        assert_eq!(calc.current_speed(), 0.0);
    }

    #[test]
    fn test_speed_calculator_two_samples() {
        let mut calc = SpeedCalculator::new();
        let start = Instant::now();

        calc.add_sample(start, 0);
        calc.add_sample(start + Duration::from_secs(1), 100);

        let speed = calc.current_speed();
        assert!(speed > 90.0 && speed < 110.0); // Allow some tolerance
    }

    #[test]
    fn test_speed_calculator_window() {
        let mut calc = SpeedCalculator::new();
        let start = Instant::now();

        // Add samples spanning 10 seconds
        for i in 0..11 {
            calc.add_sample(
                start + Duration::from_secs(i),
                i * 100,
            );
        }

        // Only last 5 seconds should be considered (window_size)
        // So samples should be pruned
        assert!(calc.samples.len() <= 6); // 5 seconds + 1
    }

    #[test]
    fn test_speed_calculator_clear() {
        let mut calc = SpeedCalculator::new();
        calc.add_sample(Instant::now(), 100);
        calc.add_sample(Instant::now(), 200);

        assert_eq!(calc.samples.len(), 2);

        calc.clear();
        assert_eq!(calc.samples.len(), 0);
    }
}
