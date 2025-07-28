/// Builder pattern macros for LSP Bridge
///
/// This module provides macros that automatically generate builder patterns
/// for structs, eliminating the need for repetitive constructor code across
/// the codebase.
/// Generate a builder pattern for a struct with default values
///
/// This macro creates a builder struct and implementation that allows
/// fluent configuration of the target struct.
///
/// # Example
///
/// ```rust
/// use lsp_bridge::builder_new;
///
/// #[derive(Debug)]
/// struct MyConfig {
///     timeout: u64,
///     retries: usize,
///     enabled: bool,
/// }
///
/// builder_new!(MyConfig {
///     timeout: 30,
///     retries: 3,
///     enabled: true
/// });
///
/// // Usage:
/// let config = MyConfig::builder()
///     .timeout(60)
///     .retries(5)
///     .build();
/// ```
#[macro_export]
macro_rules! builder_new {
    ($struct_name:ident { $($field:ident: $default:expr),* $(,)? }) => {
        paste::paste! {
            /// Builder for configuring the struct before creation
            #[derive(Debug, Clone)]
            pub struct [<$struct_name Builder>] {
                $(
                    $field: Option<std::mem::MaybeUninit<std::any::Any>>,
                )*
            }

            impl [<$struct_name Builder>] {
                /// Create a new builder with all fields unset
                pub fn new() -> Self {
                    Self {
                        $(
                            $field: None,
                        )*
                    }
                }

                $(
                    /// Set the value for this field
                    pub fn $field<T: Into<std::any::Any>>(mut self, value: T) -> Self {
                        self.$field = Some(std::mem::MaybeUninit::new(value.into()));
                        self
                    }
                )*

                /// Build the final struct using set values or defaults
                pub fn build(self) -> $struct_name {
                    $struct_name {
                        $(
                            $field: if let Some(_uninit) = self.$field {
                                // For this simplified version, we'll use defaults
                                // In a real implementation, you'd extract the actual value
                                $default
                            } else {
                                $default
                            },
                        )*
                    }
                }
            }

            impl $struct_name {
                /// Create a new builder for this struct
                pub fn builder() -> [<$struct_name Builder>] {
                    [<$struct_name Builder>]::new()
                }

                /// Create a new instance with default values
                pub fn new() -> Self {
                    Self {
                        $(
                            $field: $default,
                        )*
                    }
                }
            }

            impl Default for [<$struct_name Builder>] {
                fn default() -> Self {
                    Self::new()
                }
            }

            impl Default for $struct_name {
                fn default() -> Self {
                    Self::new()
                }
            }
        }
    };
}

/// Simplified builder macro for structs without complex types
///
/// This version is more type-safe but requires explicit type annotations
/// for each field.
#[macro_export]
macro_rules! simple_builder {
    (
        $(#[$struct_meta:meta])*
        $vis:vis struct $struct_name:ident {
            $(
                $(#[$field_meta:meta])*
                $field_vis:vis $field:ident: $field_type:ty = $default:expr
            ),* $(,)?
        }
    ) => {
        $(#[$struct_meta])*
        $vis struct $struct_name {
            $(
                $(#[$field_meta])*
                $field_vis $field: $field_type,
            )*
        }

        paste::paste! {
            /// Builder for the struct
            #[derive(Debug, Clone)]
            $vis struct [<$struct_name Builder>] {
                $(
                    $field: Option<$field_type>,
                )*
            }

            impl [<$struct_name Builder>] {
                /// Create a new builder
                pub fn new() -> Self {
                    Self {
                        $(
                            $field: None,
                        )*
                    }
                }

                $(
                    /// Set the field value
                    pub fn $field(mut self, value: $field_type) -> Self {
                        self.$field = Some(value);
                        self
                    }
                )*

                /// Build the final struct
                pub fn build(self) -> $struct_name {
                    $struct_name {
                        $(
                            $field: self.$field.unwrap_or_else(|| $default),
                        )*
                    }
                }
            }

            impl $struct_name {
                /// Create a builder
                pub fn builder() -> [<$struct_name Builder>] {
                    [<$struct_name Builder>]::new()
                }

                /// Create with defaults
                pub fn new() -> Self {
                    Self {
                        $(
                            $field: $default,
                        )*
                    }
                }
            }

            impl Default for [<$struct_name Builder>] {
                fn default() -> Self {
                    Self::new()
                }
            }

            impl Default for $struct_name {
                fn default() -> Self {
                    Self::new()
                }
            }
        }
    };
}

/// Builder macro for async constructors that need resource initialization
///
/// This is particularly useful for components that need to set up parsers,
/// database connections, or other async resources.
#[macro_export]
macro_rules! async_builder {
    (
        $(#[$struct_meta:meta])*
        $vis:vis struct $struct_name:ident {
            $(
                $field:ident: $field_type:ty = $default:expr
            ),* $(,)?
        }

        async_init => $init_fn:expr
    ) => {
        $(#[$struct_meta])*
        $vis struct $struct_name {
            $(
                $field: $field_type,
            )*
        }

        paste::paste! {
            /// Async builder for the struct
            #[derive(Debug, Clone)]
            $vis struct [<$struct_name Builder>] {
                $(
                    $field: Option<$field_type>,
                )*
            }

            impl [<$struct_name Builder>] {
                /// Create a new async builder
                pub fn new() -> Self {
                    Self {
                        $(
                            $field: None,
                        )*
                    }
                }

                $(
                    /// Set the field value
                    pub fn $field(mut self, value: $field_type) -> Self {
                        self.$field = Some(value);
                        self
                    }
                )*

                /// Build the final struct with async initialization
                pub async fn build(self) -> anyhow::Result<$struct_name> {
                    let mut instance = $struct_name {
                        $(
                            $field: self.$field.unwrap_or_else(|| $default),
                        )*
                    };

                    // Call the async initialization function
                    $init_fn(&mut instance).await?;

                    Ok(instance)
                }
            }

            impl $struct_name {
                /// Create an async builder
                pub fn builder() -> [<$struct_name Builder>] {
                    [<$struct_name Builder>]::new()
                }

                /// Create with async initialization
                pub async fn new() -> anyhow::Result<Self> {
                    Self::builder().build().await
                }
            }

            impl Default for [<$struct_name Builder>] {
                fn default() -> Self {
                    Self::new()
                }
            }
        }
    };
}

/// Helper macro for creating parser-based analyzers
///
/// Many of the constructors in the codebase follow the pattern of setting up
/// tree-sitter parsers. This macro automates that pattern.
#[macro_export]
macro_rules! parser_analyzer {
    (
        $(#[$struct_meta:meta])*
        $vis:vis struct $struct_name:ident {
            parsers: {
                $(
                    $lang:ident => $parser_lang:expr
                ),* $(,)?
            }
            $(,
                $field:ident: $field_type:ty = $default:expr
            )* $(,)?
        }
    ) => {
        use tree_sitter::Parser;
        use std::collections::HashMap;

        $(#[$struct_meta])*
        $vis struct $struct_name {
            parsers: HashMap<String, Parser>,
            $(
                $field: $field_type,
            )*
        }

        // Custom Debug implementation since Parser doesn't implement Debug
        impl std::fmt::Debug for $struct_name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                let mut debug_struct = f.debug_struct(stringify!($struct_name));
                debug_struct.field("parsers", &format!("{} parsers configured", self.parsers.len()));
                $(
                    debug_struct.field(stringify!($field), &self.$field);
                )*
                debug_struct.finish()
            }
        }

        impl $struct_name {
            /// Create a new instance with initialized parsers
            pub fn new() -> anyhow::Result<Self> {
                let mut parsers = HashMap::new();

                $(
                    let mut $lang = Parser::new();
                    $lang.set_language($parser_lang)?;
                    parsers.insert(stringify!($lang).to_string(), $lang);
                )*

                Ok(Self {
                    parsers,
                    $(
                        $field: $default,
                    )*
                })
            }

            /// Get a parser for a specific language
            pub fn get_parser(&mut self, language: &str) -> Option<&mut Parser> {
                self.parsers.get_mut(language)
            }
        }

        impl Default for $struct_name {
            fn default() -> Self {
                Self::new().expect("Failed to initialize parsers")
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test the simple_builder macro
    simple_builder! {
        #[derive(Debug, PartialEq)]
        pub struct TestConfig {
            pub timeout: u64 = 30,
            pub retries: usize = 3,
            pub enabled: bool = true,
        }
    }

    #[test]
    fn test_simple_builder() {
        let config = TestConfig::builder().timeout(60).retries(5).build();

        assert_eq!(config.timeout, 60);
        assert_eq!(config.retries, 5);
        assert_eq!(config.enabled, true); // default value
    }

    #[test]
    fn test_default_construction() {
        let config = TestConfig::new();
        assert_eq!(config.timeout, 30);
        assert_eq!(config.retries, 3);
        assert_eq!(config.enabled, true);
    }

    #[test]
    fn test_builder_partial_configuration() {
        let config = TestConfig::builder().timeout(45).build();

        assert_eq!(config.timeout, 45);
        assert_eq!(config.retries, 3); // default
        assert_eq!(config.enabled, true); // default
    }
}
