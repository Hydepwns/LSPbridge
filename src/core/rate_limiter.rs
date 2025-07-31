use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Configuration for rate limiting
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Maximum number of requests per window
    pub max_requests: u32,
    /// Time window for rate limiting
    pub window_duration: Duration,
    /// Maximum number of clients to track (prevents memory exhaustion)
    pub max_clients: usize,
    /// Enable per-IP rate limiting
    pub per_ip_limiting: bool,
    /// Global rate limit (applies to all requests regardless of IP)
    pub global_limit: Option<u32>,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            max_requests: 60,                       // More conservative default
            window_duration: Duration::from_secs(60), // 1 minute window
            max_clients: 5000,                      // Conservative client limit
            per_ip_limiting: true,
            global_limit: Some(3000),               // Conservative global limit
        }
    }
}

impl RateLimitConfig {
    /// Create a strict rate limit configuration for production
    pub fn strict() -> Self {
        Self {
            max_requests: 50,
            window_duration: Duration::from_secs(60),
            max_clients: 5000,
            per_ip_limiting: true,
            global_limit: Some(500),
        }
    }

    /// Create a permissive rate limit configuration for development
    pub fn permissive() -> Self {
        Self {
            max_requests: 1000,
            window_duration: Duration::from_secs(60),
            max_clients: 50000,
            per_ip_limiting: true,
            global_limit: Some(10000),
        }
    }
}

#[derive(Debug)]
struct ClientState {
    requests: Vec<Instant>,
    first_seen: Instant,
}

impl ClientState {
    fn new() -> Self {
        Self {
            requests: Vec::new(),
            first_seen: Instant::now(),
        }
    }

    /// Check if this client is within rate limits
    fn is_within_limits(&mut self, config: &RateLimitConfig) -> bool {
        let now = Instant::now();
        let window_start = now - config.window_duration;

        // Remove old requests outside the window
        self.requests.retain(|&time| time > window_start);

        // Check if within limit
        if self.requests.len() >= config.max_requests as usize {
            return false;
        }

        // Record this request
        self.requests.push(now);
        true
    }

    /// Get the time until this client can make another request
    fn time_until_next_allowed(&self, config: &RateLimitConfig) -> Option<Duration> {
        if self.requests.len() < config.max_requests as usize {
            return None;
        }

        // Find the oldest request in the current window
        let now = Instant::now();
        let window_start = now - config.window_duration;
        
        if let Some(&oldest_in_window) = self.requests.iter().find(|&&time| time > window_start) {
            let next_allowed = oldest_in_window + config.window_duration;
            if next_allowed > now {
                return Some(next_allowed - now);
            }
        }

        None
    }
}

/// Rate limiter that tracks requests per client and globally
pub struct RateLimiter {
    config: RateLimitConfig,
    clients: Arc<RwLock<HashMap<String, ClientState>>>,
    global_requests: Arc<RwLock<Vec<Instant>>>,
    start_time: Instant,
}

impl RateLimiter {
    /// Create a new rate limiter with the given configuration
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            config,
            clients: Arc::new(RwLock::new(HashMap::new())),
            global_requests: Arc::new(RwLock::new(Vec::new())),
            start_time: Instant::now(),
        }
    }

    /// Create a rate limiter with default configuration
    pub fn default() -> Self {
        Self::new(RateLimitConfig::default())
    }

    /// Check if a request should be allowed
    pub async fn check_request(&self, client_id: &str) -> Result<RateLimitResult> {
        // Check global rate limit first if enabled
        if let Some(global_limit) = self.config.global_limit {
            let mut global_requests = self.global_requests.write().await;
            let now = Instant::now();
            let window_start = now - self.config.window_duration;

            // Clean old global requests
            global_requests.retain(|&time| time > window_start);

            if global_requests.len() >= global_limit as usize {
                return Ok(RateLimitResult::GlobalLimitExceeded);
            }

            // Record this global request
            global_requests.push(now);
        }

        // Check per-client rate limit if enabled
        if self.config.per_ip_limiting {
            let mut clients = self.clients.write().await;

            // Prevent memory exhaustion by limiting tracked clients
            if clients.len() >= self.config.max_clients && !clients.contains_key(client_id) {
                // Remove oldest client to make room
                if let Some(oldest_key) = clients
                    .iter()
                    .min_by_key(|(_, state)| state.first_seen)
                    .map(|(key, _)| key.clone())
                {
                    clients.remove(&oldest_key);
                }
            }

            let client_state = clients.entry(client_id.to_string()).or_insert_with(ClientState::new);

            if !client_state.is_within_limits(&self.config) {
                let retry_after = client_state.time_until_next_allowed(&self.config);
                return Ok(RateLimitResult::ClientLimitExceeded { retry_after });
            }
        }

        Ok(RateLimitResult::Allowed)
    }

    /// Get current rate limiting statistics
    pub async fn get_stats(&self) -> RateLimitStats {
        let clients = self.clients.read().await;
        let global_requests = self.global_requests.read().await;
        
        let now = Instant::now();
        let window_start = now - self.config.window_duration;
        
        let active_clients = clients.len();
        let current_global_requests = global_requests
            .iter()
            .filter(|&&time| time > window_start)
            .count();

        RateLimitStats {
            active_clients,
            current_global_requests,
            window_duration_secs: self.config.window_duration.as_secs(),
            max_requests_per_client: self.config.max_requests,
            global_limit: self.config.global_limit,
            uptime_secs: self.start_time.elapsed().as_secs(),
        }
    }

    /// Clear all rate limiting state (useful for testing)
    pub async fn reset(&self) {
        let mut clients = self.clients.write().await;
        let mut global_requests = self.global_requests.write().await;
        clients.clear();
        global_requests.clear();
    }
}

/// Result of a rate limit check
#[derive(Debug, Clone)]
pub enum RateLimitResult {
    /// Request is allowed
    Allowed,
    /// Client has exceeded their rate limit
    ClientLimitExceeded {
        /// Time to wait before next request
        retry_after: Option<Duration>,
    },
    /// Global rate limit has been exceeded
    GlobalLimitExceeded,
}

impl RateLimitResult {
    /// Check if the request should be allowed
    pub fn is_allowed(&self) -> bool {
        matches!(self, RateLimitResult::Allowed)
    }

    /// Get the HTTP status code for this result
    pub fn http_status_code(&self) -> u16 {
        match self {
            RateLimitResult::Allowed => 200,
            RateLimitResult::ClientLimitExceeded { .. } => 429,
            RateLimitResult::GlobalLimitExceeded => 503,
        }
    }

    /// Get the error message for this result
    pub fn error_message(&self) -> Option<String> {
        match self {
            RateLimitResult::Allowed => None,
            RateLimitResult::ClientLimitExceeded { retry_after } => {
                let base_msg = "Rate limit exceeded for this client";
                if let Some(duration) = retry_after {
                    Some(format!("{}, retry after {} seconds", base_msg, duration.as_secs()))
                } else {
                    Some(base_msg.to_string())
                }
            }
            RateLimitResult::GlobalLimitExceeded => {
                Some("Global rate limit exceeded, please try again later".to_string())
            }
        }
    }
}

/// Statistics about current rate limiting state
#[derive(Debug, Clone)]
pub struct RateLimitStats {
    pub active_clients: usize,
    pub current_global_requests: usize,
    pub window_duration_secs: u64,
    pub max_requests_per_client: u32,
    pub global_limit: Option<u32>,
    pub uptime_secs: u64,
}

/// Helper function to extract client identifier from various sources
pub fn extract_client_id(ip: Option<IpAddr>, user_agent: Option<&str>, api_key: Option<&str>) -> String {
    // Prefer API key if available (for authenticated requests)
    if let Some(key) = api_key {
        return format!("api:{}", key);
    }

    // Fall back to IP address
    if let Some(ip) = ip {
        return format!("ip:{}", ip);
    }

    // Use user agent as last resort (not ideal but better than nothing)
    if let Some(ua) = user_agent {
        return format!("ua:{}", ua);
    }

    // Default to anonymous if no identifier available
    "anonymous".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_basic_rate_limiting() {
        let config = RateLimitConfig {
            max_requests: 2,
            window_duration: Duration::from_millis(100),
            max_clients: 100,
            per_ip_limiting: true,
            global_limit: None,
        };

        let limiter = RateLimiter::new(config);

        // First two requests should be allowed
        assert!(limiter.check_request("client1").await.unwrap().is_allowed());
        assert!(limiter.check_request("client1").await.unwrap().is_allowed());

        // Third request should be denied
        assert!(!limiter.check_request("client1").await.unwrap().is_allowed());

        // Wait for window to expire
        sleep(Duration::from_millis(150)).await;

        // Should be allowed again
        assert!(limiter.check_request("client1").await.unwrap().is_allowed());
    }

    #[tokio::test]
    async fn test_global_rate_limiting() {
        let config = RateLimitConfig {
            max_requests: 10,
            window_duration: Duration::from_millis(100),
            max_clients: 100,
            per_ip_limiting: true,
            global_limit: Some(2),
        };

        let limiter = RateLimiter::new(config);

        // First two requests from different clients should be allowed
        assert!(limiter.check_request("client1").await.unwrap().is_allowed());
        assert!(limiter.check_request("client2").await.unwrap().is_allowed());

        // Third request should hit global limit
        let result = limiter.check_request("client3").await.unwrap();
        assert!(!result.is_allowed());
        assert!(matches!(result, RateLimitResult::GlobalLimitExceeded));
    }

    #[tokio::test]
    async fn test_client_isolation() {
        let config = RateLimitConfig {
            max_requests: 1,
            window_duration: Duration::from_millis(100),
            max_clients: 100,
            per_ip_limiting: true,
            global_limit: None,
        };

        let limiter = RateLimiter::new(config);

        // Each client should have independent limits
        assert!(limiter.check_request("client1").await.unwrap().is_allowed());
        assert!(limiter.check_request("client2").await.unwrap().is_allowed());

        // Both should be denied on second request
        assert!(!limiter.check_request("client1").await.unwrap().is_allowed());
        assert!(!limiter.check_request("client2").await.unwrap().is_allowed());
    }
}