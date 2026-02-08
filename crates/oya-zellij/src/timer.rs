//! Timer-based auto-refresh for Zellij UI
//!
//! Provides configurable periodic timer events for automatic UI refresh.
//! Uses functional patterns with zero panics and immutable state where possible.

use std::time::Duration;

/// Timer configuration error
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimerError {
    /// Invalid interval duration
    InvalidInterval(String),

    /// Timer not started
    NotStarted,

    /// Timer already started
    AlreadyStarted,
}

impl std::fmt::Display for TimerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidInterval(msg) => write!(f, "Invalid interval: {}", msg),
            Self::NotStarted => write!(f, "Timer not started"),
            Self::AlreadyStarted => write!(f, "Timer already started"),
        }
    }
}

impl std::error::Error for TimerError {}

/// Represents a timer event
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimerEvent {
    event_id: u64,
    tick_number: u64,
    timestamp_ms: u64,
}

impl TimerEvent {
    /// Create a new timer event
    pub fn new(event_id: u64, tick_number: u64, timestamp_ms: u64) -> Self {
        Self {
            event_id,
            tick_number,
            timestamp_ms,
        }
    }

    /// Get the event ID
    pub fn event_id(&self) -> u64 {
        self.event_id
    }

    /// Get the tick number (starts at 1)
    pub fn tick_number(&self) -> u64 {
        self.tick_number
    }

    /// Get the timestamp in milliseconds
    pub fn timestamp_ms(&self) -> u64 {
        self.timestamp_ms
    }
}

/// Timer state
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimerState {
    Idle,
    Running,
    Paused,
    Stopped,
}

/// Timer configuration
#[derive(Debug, Clone, PartialEq)]
pub struct TimerConfig {
    interval_ms: u64,
    max_ticks: Option<u64>,
}

impl TimerConfig {
    /// Create a new timer configuration
    ///
    /// # Arguments
    /// * `interval_ms` - Interval between ticks in milliseconds (minimum 100ms)
    ///
    /// # Returns
    /// * `Ok(TimerConfig)` if the interval is valid
    /// * `Err(TimerError)` if the interval is too short
    pub fn new(interval_ms: u64) -> Result<Self, TimerError> {
        if interval_ms < 100 {
            return Err(TimerError::InvalidInterval(
                "Interval must be at least 100ms".to_string(),
            ));
        }
        Ok(Self {
            interval_ms,
            max_ticks: None,
        })
    }

    /// Set the maximum number of ticks before auto-stop
    pub fn with_max_ticks(mut self, max_ticks: u64) -> Self {
        self.max_ticks = Some(max_ticks);
        self
    }

    /// Get the interval in milliseconds
    pub fn interval_ms(&self) -> u64 {
        self.interval_ms
    }

    /// Get the interval as a Duration
    pub fn interval_duration(&self) -> Duration {
        Duration::from_millis(self.interval_ms)
    }

    /// Get the maximum ticks
    pub fn max_ticks(&self) -> Option<u64> {
        self.max_ticks
    }
}

/// Auto-refresh timer for periodic UI updates
#[derive(Debug, Clone)]
pub struct RefreshTimer {
    config: TimerConfig,
    state: TimerState,
    current_tick: u64,
    event_counter: u64,
}

impl RefreshTimer {
    /// Create a new refresh timer with the given configuration
    pub fn new(config: TimerConfig) -> Self {
        Self {
            config,
            state: TimerState::Idle,
            current_tick: 0,
            event_counter: 0,
        }
    }

    /// Create a new timer with the default 2-second interval
    pub fn default_interval() -> Result<Self, TimerError> {
        Ok(Self::new(TimerConfig::new(2000)?))
    }

    /// Create a new timer with a fast 1-second interval
    pub fn fast_interval() -> Result<Self, TimerError> {
        Ok(Self::new(TimerConfig::new(1000)?))
    }

    /// Create a new timer with a slow 5-second interval
    pub fn slow_interval() -> Result<Self, TimerError> {
        Ok(Self::new(TimerConfig::new(5000)?))
    }

    /// Get the current timer state
    pub fn state(&self) -> &TimerState {
        &self.state
    }

    /// Get the current tick count
    pub fn current_tick(&self) -> u64 {
        self.current_tick
    }

    /// Get the timer configuration
    pub fn config(&self) -> &TimerConfig {
        &self.config
    }

    /// Check if the timer is running
    pub fn is_running(&self) -> bool {
        matches!(self.state, TimerState::Running)
    }

    /// Check if the timer should stop based on max_ticks
    pub fn should_stop(&self) -> bool {
        if let Some(max) = self.config.max_ticks() {
            self.current_tick >= max
        } else {
            false
        }
    }

    /// Start the timer
    ///
    /// Returns a new timer in the Running state.
    pub fn start(mut self) -> Result<Self, TimerError> {
        if matches!(self.state, TimerState::Running) {
            return Err(TimerError::AlreadyStarted);
        }
        self.state = TimerState::Running;
        self.current_tick = 0;
        Ok(self)
    }

    /// Pause the timer
    pub fn pause(mut self) -> Result<Self, TimerError> {
        if !matches!(self.state, TimerState::Running) {
            return Err(TimerError::NotStarted);
        }
        self.state = TimerState::Paused;
        Ok(self)
    }

    /// Resume the timer
    pub fn resume(mut self) -> Result<Self, TimerError> {
        if !matches!(self.state, TimerState::Paused) {
            return Err(TimerError::NotStarted);
        }
        self.state = TimerState::Running;
        Ok(self)
    }

    /// Stop the timer
    pub fn stop(mut self) -> Self {
        self.state = TimerState::Stopped;
        self.current_tick = 0;
        self
    }

    /// Reset the timer to idle state
    pub fn reset(mut self) -> Self {
        self.state = TimerState::Idle;
        self.current_tick = 0;
        self
    }

    /// Generate the next tick event
    ///
    /// This simulates a timer tick and returns the next timer event.
    /// In a real async environment, this would be triggered by the actual timer.
    pub fn tick(mut self) -> Result<(Self, TimerEvent), TimerError> {
        if !matches!(self.state, TimerState::Running) {
            return Err(TimerError::NotStarted);
        }

        self.current_tick += 1;
        self.event_counter += 1;

        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or_else(|_| 0, |d| d.as_millis() as u64);

        let event = TimerEvent::new(self.event_counter, self.current_tick, timestamp_ms);

        // Auto-stop if we've reached max ticks
        if self.should_stop() {
            self.state = TimerState::Stopped;
        }

        Ok((self, event))
    }

    /// Check if a tick is due based on elapsed time
    ///
    /// This is useful for manual tick checking in a polling loop.
    pub fn is_tick_due(&self, last_tick_ms: u64) -> bool {
        use std::time::{SystemTime, UNIX_EPOCH};
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or_else(|_| 0, |d| d.as_millis() as u64);

        now_ms.saturating_sub(last_tick_ms) >= self.config.interval_ms
    }
}

impl Default for RefreshTimer {
    fn default() -> Self {
        // Fallback if 2000ms is invalid (should never happen)
        Self::new(TimerConfig::new(2000).unwrap_or(TimerConfig {
            interval_ms: 2000,
            max_ticks: None,
        }))
    }
}

#[cfg(test)]
#[expect(clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn test_timer_config_valid_interval() {
        let config = TimerConfig::new(2000);
        assert!(config.is_ok());
        assert_eq!(config.unwrap().interval_ms(), 2000);
    }

    #[test]
    fn test_timer_config_invalid_interval() {
        let config = TimerConfig::new(50);
        assert!(config.is_err());
        match config {
            Err(TimerError::InvalidInterval(_)) => (),
            _ => panic!("Expected InvalidInterval error"),
        }
    }

    #[test]
    fn test_timer_config_minimum_interval() {
        let config = TimerConfig::new(100);
        assert!(config.is_ok());
        assert_eq!(config.unwrap().interval_ms(), 100);
    }

    #[test]
    fn test_timer_config_with_max_ticks() {
        let config = TimerConfig::new(1000).unwrap();
        let config = config.with_max_ticks(10);
        assert_eq!(config.max_ticks(), Some(10));
    }

    #[test]
    fn test_timer_config_interval_duration() {
        let config = TimerConfig::new(2500).unwrap();
        assert_eq!(config.interval_duration(), Duration::from_millis(2500));
    }

    #[test]
    fn test_refresh_timer_creation() {
        let config = TimerConfig::new(2000).unwrap();
        let timer = RefreshTimer::new(config);
        assert_eq!(timer.state(), &TimerState::Idle);
        assert_eq!(timer.current_tick(), 0);
    }

    #[test]
    fn test_refresh_timer_default_interval() {
        let timer = RefreshTimer::default_interval();
        assert!(timer.is_ok());
        assert_eq!(timer.unwrap().config().interval_ms(), 2000);
    }

    #[test]
    fn test_refresh_timer_fast_interval() {
        let timer = RefreshTimer::fast_interval();
        assert!(timer.is_ok());
        assert_eq!(timer.unwrap().config().interval_ms(), 1000);
    }

    #[test]
    fn test_refresh_timer_slow_interval() {
        let timer = RefreshTimer::slow_interval();
        assert!(timer.is_ok());
        assert_eq!(timer.unwrap().config().interval_ms(), 5000);
    }

    #[test]
    fn test_refresh_timer_start() {
        let timer = RefreshTimer::default_interval().unwrap();
        assert!(!timer.is_running());

        let timer = timer.start().unwrap();
        assert!(timer.is_running());
        assert_eq!(timer.state(), &TimerState::Running);
    }

    #[test]
    fn test_refresh_timer_start_already_running() {
        let timer = RefreshTimer::default_interval().unwrap().start().unwrap();
        let result = timer.start();
        assert!(result.is_err());
        match result {
            Err(TimerError::AlreadyStarted) => (),
            _ => panic!("Expected AlreadyStarted error"),
        }
    }

    #[test]
    fn test_refresh_timer_pause() {
        let timer = RefreshTimer::default_interval().unwrap().start().unwrap();

        let timer = timer.pause().unwrap();
        assert_eq!(timer.state(), &TimerState::Paused);
    }

    #[test]
    fn test_refresh_timer_pause_not_running() {
        let timer = RefreshTimer::default_interval().unwrap();
        let result = timer.pause();
        assert!(result.is_err());
    }

    #[test]
    fn test_refresh_timer_resume() {
        let timer = RefreshTimer::default_interval()
            .unwrap()
            .start()
            .unwrap()
            .pause()
            .unwrap();

        let timer = timer.resume().unwrap();
        assert_eq!(timer.state(), &TimerState::Running);
    }

    #[test]
    fn test_refresh_timer_resume_not_paused() {
        let timer = RefreshTimer::default_interval().unwrap();
        let result = timer.resume();
        assert!(result.is_err());
    }

    #[test]
    fn test_refresh_timer_stop() {
        let timer = RefreshTimer::default_interval()
            .unwrap()
            .start()
            .unwrap()
            .stop();

        assert_eq!(timer.state(), &TimerState::Stopped);
        assert_eq!(timer.current_tick(), 0);
    }

    #[test]
    fn test_refresh_timer_reset() {
        let timer = RefreshTimer::default_interval().unwrap().start().unwrap();

        let timer = timer.reset();
        assert_eq!(timer.state(), &TimerState::Idle);
        assert_eq!(timer.current_tick(), 0);
    }

    #[test]
    fn test_refresh_timer_tick() {
        let timer = RefreshTimer::default_interval().unwrap().start().unwrap();

        let (timer, event) = timer.tick().unwrap();
        assert_eq!(timer.current_tick(), 1);
        assert_eq!(event.tick_number(), 1);
        assert_eq!(event.event_id(), 1);
    }

    #[test]
    fn test_refresh_timer_tick_multiple() {
        let mut timer = RefreshTimer::default_interval().unwrap().start().unwrap();

        for i in 1..=5 {
            let (new_timer, event) = timer.tick().unwrap();
            timer = new_timer;
            assert_eq!(event.tick_number(), i);
            assert_eq!(timer.current_tick(), i);
        }
    }

    #[test]
    fn test_refresh_timer_tick_not_running() {
        let timer = RefreshTimer::default_interval().unwrap();
        let result = timer.tick();
        assert!(result.is_err());
    }

    #[test]
    fn test_refresh_timer_should_stop() {
        let config = TimerConfig::new(1000).unwrap().with_max_ticks(3);
        let timer = RefreshTimer::new(config).start().unwrap();

        assert!(!timer.should_stop());

        let (timer, _) = timer.tick().unwrap();
        assert!(!timer.should_stop());

        let (timer, _) = timer.tick().unwrap();
        assert!(!timer.should_stop());

        let (timer, _) = timer.tick().unwrap();
        assert!(timer.should_stop());
        assert_eq!(timer.state(), &TimerState::Stopped);
    }

    #[test]
    fn test_refresh_timer_no_max_ticks() {
        let mut timer = RefreshTimer::default_interval().unwrap().start().unwrap();

        for _ in 0..100 {
            let (new_timer, _) = timer.tick().unwrap();
            timer = new_timer;
        }

        assert!(!timer.should_stop());
        assert!(timer.is_running());
    }

    #[test]
    fn test_timer_event_creation() {
        let event = TimerEvent::new(1, 5, 1234567890);
        assert_eq!(event.event_id(), 1);
        assert_eq!(event.tick_number(), 5);
        assert_eq!(event.timestamp_ms(), 1234567890);
    }

    #[test]
    fn test_timer_event_id_increment() {
        let timer = RefreshTimer::default_interval().unwrap().start().unwrap();

        let (timer, event1) = timer.tick().unwrap();
        let (timer, event2) = timer.tick().unwrap();
        let (_, event3) = timer.tick().unwrap();

        assert_eq!(event1.event_id(), 1);
        assert_eq!(event2.event_id(), 2);
        assert_eq!(event3.event_id(), 3);
    }

    #[test]
    fn test_refresh_timer_default_trait() {
        let timer = RefreshTimer::default();
        assert_eq!(timer.config().interval_ms(), 2000);
        assert_eq!(timer.state(), &TimerState::Idle);
    }

    #[test]
    fn test_timer_error_display() {
        let err = TimerError::InvalidInterval("test error".to_string());
        let display = format!("{}", err);
        assert!(display.contains("Invalid interval"));
        assert!(display.contains("test error"));

        let err = TimerError::NotStarted;
        assert_eq!(format!("{}", err), "Timer not started");

        let err = TimerError::AlreadyStarted;
        assert_eq!(format!("{}", err), "Timer already started");
    }

    #[test]
    fn test_is_tick_due() {
        let timer = RefreshTimer::default_interval().unwrap();

        // Should be due if more than interval has passed
        assert!(timer.is_tick_due(0));

        // Should not be due if less than interval has passed
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or_else(|_| 0, |d| d.as_millis() as u64);
        assert!(!timer.is_tick_due(now));
    }

    #[test]
    fn test_timer_state_equality() {
        let state1 = TimerState::Idle;
        let state2 = TimerState::Idle;
        let state3 = TimerState::Running;

        assert_eq!(state1, state2);
        assert_ne!(state1, state3);
    }
}
