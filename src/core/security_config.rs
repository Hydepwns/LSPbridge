/// Security-focused configuration with secure defaults for LSPbridge
///
/// This module provides enterprise-grade security configurations with
/// secure-by-default settings for production environments.
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

/// Comprehensive security configuration for LSPbridge
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Rate limiting configuration
    pub rate_limiting: RateLimitSecurityConfig,
    
    /// Input validation and sanitization settings
    pub input_validation: InputValidationConfig,
    
    /// Privacy and data protection settings
    pub privacy: PrivacySecurityConfig,
    
    /// File system access security
    pub file_access: FileAccessConfig,
    
    /// Network and API security
    pub network: NetworkSecurityConfig,
    
    /// Resource limits to prevent DoS attacks
    pub resource_limits: ResourceLimitsConfig,
    
    /// Logging and audit settings
    pub audit: AuditConfig,
}

/// Rate limiting security configuration with secure defaults
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitSecurityConfig {
    /// Enable rate limiting (should always be true in production)
    pub enabled: bool,
    
    /// Default requests per minute for unauthenticated clients
    pub default_requests_per_minute: u32,
    
    /// Strict requests per minute for suspicious clients
    pub strict_requests_per_minute: u32,
    
    /// Maximum clients to track (prevents memory exhaustion)
    pub max_tracked_clients: usize,
    
    /// Global rate limit (total requests per minute across all clients)
    pub global_requests_per_minute: u32,
    
    /// Time window for burst detection
    pub burst_window_seconds: u64,
    
    /// Maximum burst requests allowed
    pub max_burst_requests: u32,
    
    /// Enable IP-based blocking for repeat offenders
    pub enable_ip_blocking: bool,
    
    /// Duration to block IPs after repeated violations
    pub ip_block_duration_minutes: u64,
}

impl Default for RateLimitSecurityConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            default_requests_per_minute: 60,      // Conservative default
            strict_requests_per_minute: 30,       // Very conservative for suspicious clients
            max_tracked_clients: 5000,            // Prevent memory exhaustion
            global_requests_per_minute: 3000,     // Total system capacity
            burst_window_seconds: 10,             // 10-second burst window
            max_burst_requests: 20,               // Max burst before throttling
            enable_ip_blocking: true,             // Enable automatic blocking
            ip_block_duration_minutes: 60,       // Block for 1 hour
        }
    }
}

/// Input validation and sanitization configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputValidationConfig {
    /// Maximum length for query strings (prevents ReDoS attacks)
    pub max_query_length: usize,
    
    /// Maximum length for file paths
    pub max_path_length: usize,
    
    /// Maximum length for user-provided regex patterns
    pub max_regex_length: usize,
    
    /// Enable strict regex validation to prevent ReDoS
    pub strict_regex_validation: bool,
    
    /// Maximum complexity for user regex patterns
    pub max_regex_complexity_score: u32,
    
    /// Enable path traversal protection
    pub enable_path_traversal_protection: bool,
    
    /// Enable null byte detection in inputs
    pub enable_null_byte_detection: bool,
    
    /// Maximum file size for analysis (MB)
    pub max_file_size_mb: usize,
    
    /// Timeout for input validation (milliseconds)
    pub validation_timeout_ms: u64,
}

impl Default for InputValidationConfig {
    fn default() -> Self {
        Self {
            max_query_length: 8192,               // Generous but bounded
            max_path_length: 4096,                // Standard filesystem limit
            max_regex_length: 1024,               // Prevent complex regex DoS
            strict_regex_validation: true,        // Always validate regex
            max_regex_complexity_score: 100,      // Prevent exponential regex
            enable_path_traversal_protection: true, // Always protect against ../
            enable_null_byte_detection: true,     // Detect null byte attacks
            max_file_size_mb: 50,                 // Reasonable file size limit
            validation_timeout_ms: 1000,          // 1 second validation timeout
        }
    }
}

/// Privacy and data protection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivacySecurityConfig {
    /// Default privacy level (Strict = most secure)
    pub default_privacy_level: PrivacyLevel,
    
    /// Enable automatic PII detection and filtering
    pub enable_pii_detection: bool,
    
    /// Patterns to always filter (email, phone, etc.)
    pub mandatory_filter_patterns: Vec<String>,
    
    /// Enable workspace isolation (prevent cross-workspace data leaks)
    pub enable_workspace_isolation: bool,
    
    /// Maximum data retention period (days)
    pub max_data_retention_days: u32,
    
    /// Enable automatic data anonymization
    pub enable_data_anonymization: bool,
    
    /// Enable secure temporary file handling
    pub secure_temp_files: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PrivacyLevel {
    /// Maximum privacy protection
    Strict,
    /// Balanced privacy (recommended)
    Balanced,
    /// Minimal privacy (internal use only)
    Minimal,
}

impl Default for PrivacySecurityConfig {
    fn default() -> Self {
        Self {
            default_privacy_level: PrivacyLevel::Balanced, // Secure default
            enable_pii_detection: true,                    // Always detect PII
            mandatory_filter_patterns: vec![
                r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,}\b".to_string(), // Email
                r"\b\d{3}-\d{3}-\d{4}\b".to_string(),                                // Phone
                r"\b\d{4}\s\d{4}\s\d{4}\s\d{4}\b".to_string(),                      // Credit card
                r"\b[A-Z]{2}\d{2}\s[A-Z]{4}\s\d{4}\s\d{4}\s\d{4}\s\d{4}\b".to_string(), // IBAN
            ],
            enable_workspace_isolation: true,             // Prevent data leaks
            max_data_retention_days: 90,                  // 3 months max retention
            enable_data_anonymization: true,              // Anonymize by default
            secure_temp_files: true,                      // Secure temp file handling
        }
    }
}

/// File system access security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileAccessConfig {
    /// Enable sandbox mode (restrict file system access)
    pub enable_sandbox: bool,
    
    /// Allowed directories for file access
    pub allowed_directories: Vec<PathBuf>,
    
    /// Denied directories (overrides allowed)
    pub denied_directories: Vec<PathBuf>,
    
    /// Maximum directory traversal depth
    pub max_traversal_depth: u32,
    
    /// Enable symlink following restrictions
    pub restrict_symlink_following: bool,
    
    /// Enable file type restrictions
    pub enable_file_type_restrictions: bool,
    
    /// Allowed file extensions
    pub allowed_extensions: Vec<String>,
    
    /// Maximum concurrent file operations
    pub max_concurrent_file_ops: usize,
}

impl Default for FileAccessConfig {
    fn default() -> Self {
        Self {
            enable_sandbox: false,                 // Don't break existing workflows
            allowed_directories: vec![],           // Empty = current directory only
            denied_directories: vec![
                PathBuf::from("/proc"),
                PathBuf::from("/sys"),
                PathBuf::from("/dev"),
                PathBuf::from("/etc/passwd"),
                PathBuf::from("/etc/shadow"),
            ],
            max_traversal_depth: 10,               // Reasonable depth limit
            restrict_symlink_following: true,      // Prevent symlink attacks
            enable_file_type_restrictions: false,  // Don't break language support
            allowed_extensions: vec![
                ".rs".to_string(), ".ts".to_string(), ".js".to_string(),
                ".py".to_string(), ".go".to_string(), ".java".to_string(),
                ".cpp".to_string(), ".c".to_string(), ".h".to_string(),
                ".json".to_string(), ".toml".to_string(), ".yaml".to_string(),
            ],
            max_concurrent_file_ops: 100,          // Prevent resource exhaustion
        }
    }
}

/// Network and API security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkSecurityConfig {
    /// Enable TLS for all network communication
    pub require_tls: bool,
    
    /// Minimum TLS version
    pub min_tls_version: String,
    
    /// Enable request signing/authentication
    pub enable_request_signing: bool,
    
    /// API key requirements
    pub require_api_key: bool,
    
    /// Enable CORS protection
    pub enable_cors_protection: bool,
    
    /// Allowed origins for CORS
    pub allowed_origins: Vec<String>,
    
    /// Network timeout settings (seconds)
    pub network_timeout_seconds: u64,
    
    /// Maximum request body size (bytes)
    pub max_request_body_size: usize,
    
    /// Enable request logging
    pub enable_request_logging: bool,
}

impl Default for NetworkSecurityConfig {
    fn default() -> Self {
        Self {
            require_tls: false,                    // Don't break existing setups
            min_tls_version: "1.2".to_string(),    // Minimum secure TLS
            enable_request_signing: false,         // Optional for now
            require_api_key: false,                // Optional for now
            enable_cors_protection: true,          // Always enable CORS protection
            allowed_origins: vec!["localhost".to_string()], // Local only by default
            network_timeout_seconds: 30,           // Reasonable timeout
            max_request_body_size: 10 * 1024 * 1024, // 10MB max request
            enable_request_logging: true,          // Log for security monitoring
        }
    }
}

/// Resource limits to prevent DoS attacks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimitsConfig {
    /// Maximum memory usage (MB)
    pub max_memory_mb: usize,
    
    /// Maximum CPU usage percentage
    pub max_cpu_percent: f64,
    
    /// Maximum number of concurrent operations
    pub max_concurrent_operations: usize,
    
    /// Maximum processing time per request (seconds)
    pub max_processing_time_seconds: u64,
    
    /// Maximum cache size (MB)
    pub max_cache_size_mb: usize,
    
    /// Maximum database connections
    pub max_database_connections: u32,
    
    /// Enable resource monitoring
    pub enable_resource_monitoring: bool,
    
    /// Resource check interval (seconds)
    pub resource_check_interval_seconds: u64,
}

impl Default for ResourceLimitsConfig {
    fn default() -> Self {
        Self {
            max_memory_mb: 512,                    // Reasonable memory limit
            max_cpu_percent: 80.0,                 // Leave room for system
            max_concurrent_operations: 50,         // Prevent resource exhaustion
            max_processing_time_seconds: 120,      // 2 minute max processing
            max_cache_size_mb: 128,                // Reasonable cache limit
            max_database_connections: 20,          // Conservative DB limit
            enable_resource_monitoring: true,      // Always monitor resources
            resource_check_interval_seconds: 30,   // Check every 30 seconds
        }
    }
}

/// Audit and logging configuration for security monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditConfig {
    /// Enable security audit logging
    pub enable_audit_logging: bool,
    
    /// Audit log file path
    pub audit_log_path: PathBuf,
    
    /// Enable request/response logging
    pub log_requests: bool,
    
    /// Enable error logging
    pub log_security_errors: bool,
    
    /// Enable performance monitoring
    pub log_performance_metrics: bool,
    
    /// Log retention period (days)
    pub log_retention_days: u32,
    
    /// Enable structured logging (JSON format)
    pub structured_logging: bool,
    
    /// Enable sensitive data masking in logs
    pub mask_sensitive_data: bool,
}

impl Default for AuditConfig {
    fn default() -> Self {
        Self {
            enable_audit_logging: true,            // Always enable audit logging
            audit_log_path: PathBuf::from("logs/security-audit.log"),
            log_requests: true,                    // Log for security analysis
            log_security_errors: true,             // Always log security errors
            log_performance_metrics: true,         // Monitor for DoS attempts
            log_retention_days: 30,                // 30-day retention
            structured_logging: true,              // Enable structured logs
            mask_sensitive_data: true,             // Always mask sensitive data
        }
    }
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            rate_limiting: RateLimitSecurityConfig::default(),
            input_validation: InputValidationConfig::default(),
            privacy: PrivacySecurityConfig::default(),
            file_access: FileAccessConfig::default(),
            network: NetworkSecurityConfig::default(),
            resource_limits: ResourceLimitsConfig::default(),
            audit: AuditConfig::default(),
        }
    }
}

impl SecurityConfig {
    /// Create a new security configuration with production-ready defaults
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Create a strict security configuration for high-security environments
    pub fn strict() -> Self {
        Self {
            rate_limiting: RateLimitSecurityConfig {
                default_requests_per_minute: 30,      // Very conservative
                strict_requests_per_minute: 10,       // Extremely conservative
                global_requests_per_minute: 1000,     // Lower global limit
                max_burst_requests: 10,               // Lower burst limit
                ip_block_duration_minutes: 240,      // 4-hour blocks
                ..RateLimitSecurityConfig::default()
            },
            input_validation: InputValidationConfig {
                max_query_length: 4096,               // Smaller max query
                max_regex_length: 512,                // More restrictive regex
                max_regex_complexity_score: 50,       // Lower complexity limit
                max_file_size_mb: 25,                 // Smaller file limit
                validation_timeout_ms: 500,           // Faster timeout
                ..InputValidationConfig::default()
            },
            privacy: PrivacySecurityConfig {
                default_privacy_level: PrivacyLevel::Strict, // Maximum privacy
                max_data_retention_days: 30,          // Shorter retention
                ..PrivacySecurityConfig::default()
            },
            file_access: FileAccessConfig {
                enable_sandbox: true,                 // Enable sandbox mode
                max_traversal_depth: 5,               // Shallower traversal
                enable_file_type_restrictions: true,  // Restrict file types
                max_concurrent_file_ops: 50,          // Lower concurrency
                ..FileAccessConfig::default()
            },
            network: NetworkSecurityConfig {
                require_tls: true,                    // Require TLS
                require_api_key: true,                // Require authentication
                network_timeout_seconds: 15,          // Shorter timeout
                max_request_body_size: 5 * 1024 * 1024, // 5MB max request
                ..NetworkSecurityConfig::default()
            },
            resource_limits: ResourceLimitsConfig {
                max_memory_mb: 256,                   // Lower memory limit
                max_cpu_percent: 60.0,                // Lower CPU limit
                max_concurrent_operations: 25,        // Lower concurrency
                max_processing_time_seconds: 60,      // Shorter processing time
                max_cache_size_mb: 64,                // Smaller cache
                max_database_connections: 10,         // Fewer DB connections
                resource_check_interval_seconds: 15,  // More frequent checks
                ..ResourceLimitsConfig::default()
            },
            audit: AuditConfig {
                log_retention_days: 90,               // Longer audit retention
                ..AuditConfig::default()
            },
        }
    }
    
    /// Create a development-friendly security configuration
    pub fn development() -> Self {
        Self {
            rate_limiting: RateLimitSecurityConfig {
                default_requests_per_minute: 300,     // Higher limits for dev
                strict_requests_per_minute: 100,
                global_requests_per_minute: 10000,
                max_burst_requests: 50,
                enable_ip_blocking: false,            // Disable blocking in dev
                ..RateLimitSecurityConfig::default()
            },
            input_validation: InputValidationConfig {
                max_query_length: 16384,              // Larger limits for testing
                max_regex_length: 2048,
                max_file_size_mb: 100,
                ..InputValidationConfig::default()
            },
            privacy: PrivacySecurityConfig {
                default_privacy_level: PrivacyLevel::Minimal, // Less filtering in dev
                enable_pii_detection: false,          // Disable for dev data
                enable_workspace_isolation: false,    // Allow cross-workspace in dev
                ..PrivacySecurityConfig::default()
            },
            network: NetworkSecurityConfig {
                require_tls: false,                   // Optional TLS in dev
                enable_cors_protection: false,       // More permissive CORS
                allowed_origins: vec!["*".to_string()], // Allow all origins
                ..NetworkSecurityConfig::default()
            },
            resource_limits: ResourceLimitsConfig {
                max_memory_mb: 1024,                  // Higher limits for dev
                max_concurrent_operations: 100,
                max_processing_time_seconds: 300,     // Longer processing time
                ..ResourceLimitsConfig::default()
            },
            ..Self::default()
        }
    }
    
    /// Validate the security configuration for consistency
    pub fn validate(&self) -> anyhow::Result<()> {
        // Validate rate limiting
        if self.rate_limiting.default_requests_per_minute == 0 {
            anyhow::bail!("Rate limit cannot be zero");
        }
        
        if self.rate_limiting.strict_requests_per_minute > self.rate_limiting.default_requests_per_minute {
            anyhow::bail!("Strict rate limit cannot be higher than default rate limit");
        }
        
        // Validate input validation
        if self.input_validation.max_query_length == 0 {
            anyhow::bail!("Max query length cannot be zero");
        }
        
        if self.input_validation.validation_timeout_ms == 0 {
            anyhow::bail!("Validation timeout cannot be zero");
        }
        
        // Validate resource limits
        if self.resource_limits.max_memory_mb < 64 {
            anyhow::bail!("Memory limit too low: minimum 64MB required");
        }
        
        if self.resource_limits.max_cpu_percent <= 0.0 || self.resource_limits.max_cpu_percent > 100.0 {
            anyhow::bail!("CPU limit must be between 0 and 100 percent");
        }
        
        // Validate privacy settings
        if self.privacy.max_data_retention_days == 0 {
            anyhow::bail!("Data retention period cannot be zero");
        }
        
        Ok(())
    }
    
    /// Apply this security configuration to the unified config
    pub fn apply_to_unified_config(&self, unified: &mut super::config::unified::UnifiedConfig) {
        // Apply rate limiting settings
        // Note: This would require adding rate limiting fields to UnifiedConfig
        
        // Apply resource limits
        unified.memory.max_memory_mb = unified.memory.max_memory_mb.min(self.resource_limits.max_memory_mb);
        unified.performance.max_cpu_usage_percent = unified.performance.max_cpu_usage_percent.min(self.resource_limits.max_cpu_percent);
        unified.performance.max_concurrent_files = unified.performance.max_concurrent_files.min(self.resource_limits.max_concurrent_operations);
        
        // Apply cache limits
        unified.cache.max_size_mb = unified.cache.max_size_mb.min(self.resource_limits.max_cache_size_mb);
        
        // Apply timeout settings
        unified.timeouts.processing_timeout_seconds = unified.timeouts.processing_timeout_seconds.min(self.resource_limits.max_processing_time_seconds);
        unified.timeouts.network_timeout_seconds = unified.timeouts.network_timeout_seconds.min(self.network.network_timeout_seconds);
        
        // Apply file size limits
        unified.performance.file_size_limit_mb = unified.performance.file_size_limit_mb.min(self.input_validation.max_file_size_mb);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_default_security_config() {
        let config = SecurityConfig::new();
        assert!(config.validate().is_ok());
        
        // Test secure defaults
        assert!(config.rate_limiting.enabled);
        assert!(config.input_validation.strict_regex_validation);
        assert!(config.privacy.enable_pii_detection);
        assert!(config.audit.enable_audit_logging);
    }
    
    #[test]
    fn test_strict_security_config() {
        let config = SecurityConfig::strict();
        assert!(config.validate().is_ok());
        
        // Test strict settings
        assert_eq!(config.rate_limiting.default_requests_per_minute, 30);
        assert_eq!(config.input_validation.max_query_length, 4096);
        assert!(matches!(config.privacy.default_privacy_level, PrivacyLevel::Strict));
        assert!(config.file_access.enable_sandbox);
        assert!(config.network.require_tls);
    }
    
    #[test]
    fn test_development_security_config() {
        let config = SecurityConfig::development();
        assert!(config.validate().is_ok());
        
        // Test development-friendly settings
        assert_eq!(config.rate_limiting.default_requests_per_minute, 300);
        assert!(!config.rate_limiting.enable_ip_blocking);
        assert!(!config.privacy.enable_pii_detection);
        assert!(!config.network.require_tls);
    }
    
    #[test]
    fn test_security_config_validation() {
        let mut config = SecurityConfig::new();
        
        // Test invalid rate limit
        config.rate_limiting.default_requests_per_minute = 0;
        assert!(config.validate().is_err());
        
        // Test invalid memory limit
        config.rate_limiting.default_requests_per_minute = 100;
        config.resource_limits.max_memory_mb = 32;
        assert!(config.validate().is_err());
        
        // Test invalid CPU limit
        config.resource_limits.max_memory_mb = 256;
        config.resource_limits.max_cpu_percent = 150.0;
        assert!(config.validate().is_err());
    }
}