//! Serde utility helpers for case-insensitive deserialization

/// Macro to implement case-insensitive deserialization for enums
/// 
/// Usage:
/// ```
/// impl_case_insensitive_deserialize!(
///     MyEnum,
///     Variant1 => "variant1",
///     Variant2 => "variant2"
/// );
/// ```
#[macro_export]
macro_rules! impl_case_insensitive_deserialize {
    ($enum_type:ty, $($variant:ident => $str_val:expr),+ $(,)?) => {
        impl<'de> serde::Deserialize<'de> for $enum_type {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                let s = String::deserialize(deserializer)?;
                match s.to_lowercase().as_str() {
                    $(
                        $str_val => Ok(Self::$variant),
                    )+
                    _ => Err(serde::de::Error::custom(format!(
                        "unknown variant '{}', expected one of: {}",
                        s,
                        vec![$($str_val),+].join(", ")
                    ))),
                }
            }
        }
    };
}

/// Macro to implement case-insensitive deserialization for enums with data
/// 
/// This is more complex and handles enums with associated data by deserializing
/// to an intermediate representation first.
#[macro_export]
macro_rules! impl_case_insensitive_deserialize_complex {
    ($enum_type:ty) => {
        impl<'de> serde::Deserialize<'de> for $enum_type {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                // First deserialize to a generic Value
                let value = serde_json::Value::deserialize(deserializer)?;
                
                // If it's a string, try simple variant matching
                if let serde_json::Value::String(s) = &value {
                    // Convert to lowercase for comparison
                    let lower = s.to_lowercase();
                    // Try to deserialize with lowercase string
                    if let Ok(result) = serde_json::from_value(serde_json::Value::String(lower.clone())) {
                        return Ok(result);
                    }
                }
                
                // If it's an object with a tag, handle tagged enums
                if let serde_json::Value::Object(map) = &value {
                    // Clone the map and lowercase any string fields that look like tags
                    let mut new_map = serde_json::Map::new();
                    for (key, val) in map {
                        if key == "type" || key == "kind" || key == "variant" {
                            // Lowercase the variant name
                            if let serde_json::Value::String(s) = val {
                                new_map.insert(key.clone(), serde_json::Value::String(s.to_lowercase()));
                            } else {
                                new_map.insert(key.clone(), val.clone());
                            }
                        } else {
                            new_map.insert(key.clone(), val.clone());
                        }
                    }
                    
                    // Try to deserialize from the modified object
                    if let Ok(result) = serde_json::from_value(serde_json::Value::Object(new_map.clone())) {
                        return Ok(result);
                    }
                }
                
                // Fall back to normal deserialization
                serde_json::from_value(value).map_err(|e| {
                    serde::de::Error::custom(format!("deserialization failed: {}", e))
                })
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use serde::Serialize;
    
    #[derive(Debug, Clone, PartialEq, Eq, Serialize)]
    enum TestEnum {
        First,
        Second,
        ThirdOption,
    }
    
    impl_case_insensitive_deserialize!(
        TestEnum,
        First => "first",
        Second => "second",
        ThirdOption => "thirdoption"
    );
    
    #[test]
    fn test_case_insensitive_deserialize() {
        // Test lowercase
        let result: TestEnum = serde_json::from_str(r#""first""#).unwrap();
        assert_eq!(result, TestEnum::First);
        
        // Test uppercase
        let result: TestEnum = serde_json::from_str(r#""FIRST""#).unwrap();
        assert_eq!(result, TestEnum::First);
        
        // Test mixed case
        let result: TestEnum = serde_json::from_str(r#""FiRsT""#).unwrap();
        assert_eq!(result, TestEnum::First);
        
        // Test camelCase variant
        let result: TestEnum = serde_json::from_str(r#""ThirdOption""#).unwrap();
        assert_eq!(result, TestEnum::ThirdOption);
        
        // Test invalid variant
        let result: Result<TestEnum, _> = serde_json::from_str(r#""invalid""#);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("unknown variant"));
        assert!(err.contains("expected one of: first, second, thirdoption"));
    }
}