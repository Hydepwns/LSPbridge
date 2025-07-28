/// Demonstration of the builder pattern macros
///
/// This file shows how to use the new builder macros to reduce constructor
/// duplication and improve API ergonomics.
use lsp_bridge::{parser_analyzer, simple_builder};

// Example 1: Simple configuration struct using simple_builder
simple_builder! {
    /// Configuration for context ranking
    #[derive(Debug, Clone)]
    pub struct ContextRankerConfig {
        /// Maximum tokens allowed for context
        pub max_tokens: usize = 2000,
        /// Weight for function context
        pub function_weight: f32 = 1.0,
        /// Weight for class context
        pub class_weight: f32 = 0.8,
        /// Weight for import context
        pub import_weight: f32 = 0.6,
        /// Enable aggressive optimization
        pub aggressive_optimization: bool = false,
    }
}

// Example 2: Parser-based analyzer using parser_analyzer macro
// This replaces the complex constructor pattern seen in ContextExtractor
parser_analyzer! {
    /// Semantic context extractor with multiple language support
    /// Note: Debug trait is implemented automatically by the macro
    pub struct MultiLanguageAnalyzer {
        parsers: {
            typescript => tree_sitter_typescript::language_typescript(),
            rust => tree_sitter_rust::language(),
            python => tree_sitter_python::language()
        },
        max_depth: u32 = 3,
        include_dependencies: bool = true,
        cache_enabled: bool = true,
    }
}

// Example 3: Before and after comparison
mod before_refactor {
    use anyhow::Result;
    use tree_sitter::Parser;

    pub struct OldStyleAnalyzer {
        typescript_parser: Parser,
        rust_parser: Parser,
        python_parser: Parser,
        max_depth: u32,
        include_dependencies: bool,
    }

    impl OldStyleAnalyzer {
        pub fn new() -> Result<Self> {
            let mut typescript_parser = Parser::new();
            typescript_parser.set_language(tree_sitter_typescript::language_typescript())?;

            let mut rust_parser = Parser::new();
            rust_parser.set_language(tree_sitter_rust::language())?;

            let mut python_parser = Parser::new();
            python_parser.set_language(tree_sitter_python::language())?;

            Ok(Self {
                typescript_parser,
                rust_parser,
                python_parser,
                max_depth: 3,
                include_dependencies: true,
            })
        }

        pub fn with_max_depth(mut self, depth: u32) -> Self {
            self.max_depth = depth;
            self
        }

        pub fn with_dependencies(mut self, enabled: bool) -> Self {
            self.include_dependencies = enabled;
            self
        }
    }
}

mod after_refactor {
    use super::MultiLanguageAnalyzer;

    // Usage is now much simpler - the macro handles all the parser setup
    pub fn create_analyzer() -> anyhow::Result<MultiLanguageAnalyzer> {
        MultiLanguageAnalyzer::new()
    }

    // Additional configuration can be done through the struct fields
    pub fn create_custom_analyzer() -> anyhow::Result<MultiLanguageAnalyzer> {
        let mut analyzer = MultiLanguageAnalyzer::new()?;
        analyzer.max_depth = 5;
        analyzer.include_dependencies = false;
        Ok(analyzer)
    }
}

// Example 4: Advanced builder pattern for complex configurations
simple_builder! {
    /// Comprehensive diagnostic processor configuration
    #[derive(Debug, Clone)]
    pub struct ProcessorConfig {
        /// Processing timeouts
        pub timeout_seconds: u64 = 30,
        /// Parallel processing enabled
        pub parallel_processing: bool = true,
        /// Maximum concurrent files
        pub max_concurrent_files: usize = 1000,
        /// Chunk size for batch processing
        pub chunk_size: usize = 100,
        /// Enable caching
        pub enable_cache: bool = true,
        /// Cache size in MB
        pub cache_size_mb: usize = 100,
        /// Enable git integration
        pub enable_git: bool = true,
        /// Enable metrics collection
        pub enable_metrics: bool = true,
        /// Memory limit in MB
        pub memory_limit_mb: usize = 256,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_ranker_config_builder() {
        let config = ContextRankerConfig::builder()
            .max_tokens(1500)
            .function_weight(1.2)
            .aggressive_optimization(true)
            .build();

        assert_eq!(config.max_tokens, 1500);
        assert_eq!(config.function_weight, 1.2);
        assert_eq!(config.class_weight, 0.8); // default value
        assert_eq!(config.aggressive_optimization, true);
    }

    #[test]
    fn test_context_ranker_defaults() {
        let config = ContextRankerConfig::new();

        assert_eq!(config.max_tokens, 2000);
        assert_eq!(config.function_weight, 1.0);
        assert_eq!(config.class_weight, 0.8);
        assert_eq!(config.import_weight, 0.6);
        assert_eq!(config.aggressive_optimization, false);
    }

    #[test]
    fn test_processor_config_builder() {
        let config = ProcessorConfig::builder()
            .timeout_seconds(45)
            .max_concurrent_files(500)
            .enable_cache(false)
            .memory_limit_mb(512)
            .build();

        assert_eq!(config.timeout_seconds, 45);
        assert_eq!(config.max_concurrent_files, 500);
        assert_eq!(config.enable_cache, false);
        assert_eq!(config.memory_limit_mb, 512);
        // Verify defaults are preserved
        assert_eq!(config.parallel_processing, true);
        assert_eq!(config.chunk_size, 100);
    }

    #[test]
    fn test_multi_language_analyzer_creation() {
        let analyzer = MultiLanguageAnalyzer::new();
        assert!(analyzer.is_ok());

        let analyzer = analyzer.unwrap();
        assert_eq!(analyzer.max_depth, 3);
        assert_eq!(analyzer.include_dependencies, true);
        assert_eq!(analyzer.cache_enabled, true);
    }

    #[test]
    fn test_parser_access() {
        let mut analyzer = MultiLanguageAnalyzer::new().unwrap();

        // Test that parsers are available
        assert!(analyzer.get_parser("typescript").is_some());
        assert!(analyzer.get_parser("rust").is_some());
        assert!(analyzer.get_parser("python").is_some());
        assert!(analyzer.get_parser("unknown").is_none());
    }
}

// Usage examples showing the ergonomic improvements
fn main() -> anyhow::Result<()> {
    println!("=== Builder Pattern Demo ===\n");

    // Example 1: Simple configuration with fluent API
    let ranker_config = ContextRankerConfig::builder()
        .max_tokens(1800)
        .function_weight(1.5)
        .aggressive_optimization(true)
        .build();
    println!("Context Ranker Config: {:#?}\n", ranker_config);

    // Example 2: Complex processor configuration
    let processor_config = ProcessorConfig::builder()
        .timeout_seconds(60)
        .max_concurrent_files(2000)
        .cache_size_mb(200)
        .memory_limit_mb(512)
        .enable_git(false)
        .build();
    println!("Processor Config: {:#?}\n", processor_config);

    // Example 3: Multi-language analyzer
    let mut analyzer = MultiLanguageAnalyzer::new()?;
    analyzer.max_depth = 5;
    analyzer.include_dependencies = false;

    println!("Multi-Language Analyzer created successfully!");
    println!("Available parsers: TypeScript, Rust, Python");
    println!("Max depth: {}", analyzer.max_depth);
    println!("Include dependencies: {}\n", analyzer.include_dependencies);

    // Example 4: Default configurations
    let default_ranker = ContextRankerConfig::new();
    let default_processor = ProcessorConfig::new();

    println!("Default configurations created with sensible defaults");
    println!("Ranker max tokens: {}", default_ranker.max_tokens);
    println!("Processor timeout: {}s", default_processor.timeout_seconds);

    Ok(())
}
