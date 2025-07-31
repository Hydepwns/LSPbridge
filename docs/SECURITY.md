# Security Configuration Guide

LSPbridge now includes comprehensive security configurations with secure-by-default settings. This document outlines the security features and best practices.

## 🔒 Security Overview

### Current Security Status: ✅ PRODUCTION READY

All critical security vulnerabilities have been resolved and secure defaults implemented:

- ✅ **Rate Limiting**: Enterprise-grade with per-client and global limits
- ✅ **Input Validation**: Comprehensive validation with ReDoS protection  
- ✅ **Privacy Protection**: Configurable PII filtering and data sanitization
- ✅ **Resource Limits**: DoS protection through resource constraints
- ✅ **File Access Control**: Path traversal protection and sandboxing
- ✅ **Network Security**: TLS support and request validation
- ✅ **Audit Logging**: Security event logging and monitoring

## 🛡️ Security Configuration Profiles

LSPbridge provides three pre-configured security profiles:

### Production Profile (Recommended)
```rust
let config = UnifiedConfig::production();
```

**Features:**
- Strict rate limiting (30 requests/minute per client)
- Maximum input validation and sanitization
- Mandatory PII detection and filtering
- Resource limits prevent DoS attacks
- Comprehensive audit logging
- Circuit breakers for error recovery

**Use Cases:** Production deployments, customer-facing services

### Development Profile
```rust
let config = UnifiedConfig::development();
```

**Features:**
- Relaxed rate limiting (300 requests/minute per client)
- Larger resource limits for debugging
- Optional PII filtering for test data
- Experimental features enabled
- Less aggressive timeouts

**Use Cases:** Local development, debugging sessions

### Testing Profile
```rust
let config = UnifiedConfig::testing();
```

**Features:**
- Memory-only caching (no persistence)
- Predictable sequential processing
- Minimal resource usage
- Fast timeouts for quick test execution
- Disabled external integrations

**Use Cases:** CI/CD pipelines, automated testing

## 🔧 Security Configuration Details

### Rate Limiting

Prevents abuse and DoS attacks:

```rust
pub struct RateLimitSecurityConfig {
    pub enabled: bool,                          // Always true in production
    pub default_requests_per_minute: u32,       // 60 (balanced), 30 (strict)
    pub strict_requests_per_minute: u32,        // 30 (balanced), 10 (strict)
    pub global_requests_per_minute: u32,        // 3000 (balanced), 1000 (strict)
    pub max_burst_requests: u32,                // 20 (balanced), 10 (strict)
    pub enable_ip_blocking: bool,               // Auto-block repeat offenders
    pub ip_block_duration_minutes: u64,        // 60 (balanced), 240 (strict)
}
```

### Input Validation

Prevents injection attacks and ReDoS:

```rust
pub struct InputValidationConfig {
    pub max_query_length: usize,                // 8192 (balanced), 4096 (strict)
    pub max_regex_length: usize,                // 1024 (balanced), 512 (strict)
    pub strict_regex_validation: bool,          // Always true
    pub max_regex_complexity_score: u32,        // 100 (balanced), 50 (strict)
    pub enable_path_traversal_protection: bool, // Always true
    pub enable_null_byte_detection: bool,       // Always true
    pub max_file_size_mb: usize,                // 50 (balanced), 25 (strict)
}
```

### Privacy Protection

Configurable data filtering and anonymization:

```rust
pub struct PrivacySecurityConfig {
    pub default_privacy_level: PrivacyLevel,    // Balanced or Strict
    pub enable_pii_detection: bool,             // Always true in production
    pub mandatory_filter_patterns: Vec<String>, // Email, phone, credit card, etc.
    pub enable_workspace_isolation: bool,       // Prevent cross-workspace leaks
    pub max_data_retention_days: u32,           // 90 (balanced), 30 (strict)
    pub enable_data_anonymization: bool,        // Always true
    pub secure_temp_files: bool,                // Always true
}
```

### Resource Limits

Prevent resource exhaustion attacks:

```rust
pub struct ResourceLimitsConfig {
    pub max_memory_mb: usize,                   // 512 (balanced), 256 (strict)
    pub max_cpu_percent: f64,                   // 80.0 (balanced), 60.0 (strict)
    pub max_concurrent_operations: usize,       // 50 (balanced), 25 (strict)
    pub max_processing_time_seconds: u64,       // 120 (balanced), 60 (strict)
    pub max_cache_size_mb: usize,               // 128 (balanced), 64 (strict)
    pub max_database_connections: u32,          // 20 (balanced), 10 (strict)
}
```

## 🚨 Security Best Practices

### 1. Configuration Management

```rust
// ✅ GOOD: Use appropriate profile for environment
let config = match env::var("ENVIRONMENT") {
    Ok(env) if env == "production" => UnifiedConfig::production(),
    Ok(env) if env == "testing" => UnifiedConfig::testing(),
    _ => UnifiedConfig::development(),
};

// ❌ BAD: Using development config in production
let config = UnifiedConfig::development(); // Insecure for production!
```

### 2. Input Validation

```rust
// ✅ GOOD: Validate all user inputs
let validator = InputValidator::new(&config.security.input_validation);
validator.validate_query(&user_query)?;
validator.validate_file_path(&file_path)?;

// ❌ BAD: Processing user input without validation
let result = process_query(&user_query); // Potential injection attack!
```

### 3. Rate Limiting

```rust
// ✅ GOOD: Apply rate limiting to all API endpoints
let rate_limiter = RateLimiter::new(config.security.rate_limiting);
if !rate_limiter.check_rate_limit(&client_id).await? {
    return Err(ApiError::RateLimited);
}

// ❌ BAD: No rate limiting on public endpoints
process_request(&request).await // Vulnerable to DoS attacks!
```

### 4. Privacy Protection

```rust
// ✅ GOOD: Filter sensitive data before processing
let privacy_filter = PrivacyFilter::new(&config.security.privacy);
let sanitized_data = privacy_filter.filter(&raw_data)?;

// ❌ BAD: Processing raw data without filtering
let result = analyze_code(&raw_data); // May leak PII!
```

## 🔍 Security Monitoring

### Audit Logging

All security events are logged for monitoring:

```rust
pub struct AuditConfig {
    pub enable_audit_logging: bool,             // Always true
    pub audit_log_path: PathBuf,                // logs/security-audit.log
    pub log_requests: bool,                     // Log all API requests
    pub log_security_errors: bool,              // Log security violations
    pub mask_sensitive_data: bool,              // Mask PII in logs
    pub log_retention_days: u32,                // 30 (default), 90 (strict)
}
```

### Security Metrics

Monitor these metrics for security threats:

- Request rate per client (detect DoS attempts)
- Failed validation attempts (detect injection attempts)
- Resource usage patterns (detect resource exhaustion)
- Error rates by client (detect malicious activity)
- Processing times (detect algorithmic attacks)

## 🚀 Migration Guide

### Upgrading Existing Configurations

```rust
// Before: Legacy configuration
let config = DynamicConfig::default();

// After: Secure unified configuration
let mut config = UnifiedConfig::production();

// Apply custom settings while respecting security limits
config.performance.max_concurrent_files = 
    config.performance.max_concurrent_files.min(
        config.security.resource_limits.max_concurrent_operations
    );

// Validate configuration
config.validate()?;
```

### Environment-Specific Deployment

```bash
# Production deployment
export LSP_BRIDGE_ENVIRONMENT=production
export LSP_BRIDGE_SECURITY_PROFILE=strict

# Development environment  
export LSP_BRIDGE_ENVIRONMENT=development
export LSP_BRIDGE_SECURITY_PROFILE=balanced

# CI/CD testing
export LSP_BRIDGE_ENVIRONMENT=testing
export LSP_BRIDGE_SECURITY_PROFILE=minimal
```

## ⚠️ Security Warnings

### Critical Security Requirements

1. **Never disable security validation in production**
   ```rust
   // ❌ DANGEROUS: Never do this in production
   config.security.input_validation.strict_regex_validation = false;
   ```

2. **Always validate configuration before use**
   ```rust
   // ✅ REQUIRED: Always validate
   config.validate().expect("Invalid security configuration");
   ```

3. **Monitor security logs regularly**
   ```rust
   // ✅ REQUIRED: Set up log monitoring
   let monitor = SecurityMonitor::new(&config.security.audit);
   monitor.start_monitoring().await;
   ```

4. **Use appropriate TLS in production**
   ```rust
   // ✅ REQUIRED for production
   config.security.network.require_tls = true;
   config.security.network.min_tls_version = "1.3".to_string();
   ```

## 📞 Security Contact

For security vulnerabilities or questions:

- Create a security advisory on GitHub
- Follow responsible disclosure practices
- Include detailed reproduction steps
- Provide suggested fixes if possible

## 🔒 Security Compliance

LSPbridge security configuration supports compliance with:

- **GDPR**: Data retention limits and anonymization
- **SOC 2 Type II**: Audit logging and access controls  
- **ISO 27001**: Security monitoring and incident response
- **NIST Cybersecurity Framework**: Risk-based security controls

---

**Remember**: Security is a process, not a destination. Regularly review and update your security configuration as threats evolve.