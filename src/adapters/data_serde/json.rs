use std::collections::HashMap;

use async_trait::async_trait;

use crate::{
    configuration,
    ports::data_serde::{DataSerde, ValueMap},
};

#[derive(Clone)]
pub struct JsonDataSerde {
    mapping: configuration::JsonMapping,
    strict: bool,
}

impl JsonDataSerde {
    pub fn new(mapping: configuration::JsonMapping, strict: bool) -> anyhow::Result<Self> {
        if strict && mapping.0.is_empty() {
            anyhow::bail!("JSON mapping cannot be undefined or empty when strict mode is enabled");
        }
        Ok(JsonDataSerde { mapping, strict })
    }
}

#[async_trait]
impl DataSerde for JsonDataSerde {
    async fn extract_values(&self, data: Vec<u8>) -> anyhow::Result<ValueMap> {
        if data.is_empty() {
            return Ok(HashMap::new());
        }
        let mut values: ValueMap = serde_json::from_slice(&data)?;

        let result = if self.strict {
            let mut result: ValueMap = HashMap::new();
            for (key, source) in &self.mapping.0 {
                if let Some(value) = values.remove(source) {
                    result.insert(key.clone(), value);
                }
            }
            result
        } else {
            for (key, source) in &self.mapping.0 {
                if let Some(value) = values.remove(source) {
                    values.insert(key.clone(), value);
                }
            }
            values
        };

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use configuration::JsonMapping;

    use super::*;

    #[tokio::test]
    async fn test_extract_values_without_mapping_non_strict() {
        let serde = JsonDataSerde::new(JsonMapping::default(), false).unwrap();
        let data = serde_json::json!({
            "id": "abc",
            "createdAt": 123,
        });
        let result = serde.extract_values(serde_json::to_vec(&data).unwrap()).await.unwrap();

        assert_eq!(result.get("id"), Some(serde_json::Value::String("abc".to_string())).as_ref());
        assert_eq!(
            result.get("createdAt"),
            Some(serde_json::Value::Number(123i64.into())).as_ref()
        );
    }

    #[tokio::test]
    async fn test_extract_values_without_mapping_strict() {
        assert!(JsonDataSerde::new(JsonMapping::default(), false).is_err());
    }

    #[tokio::test]
    async fn test_extract_values_with_full_mapping_non_strict() {
        let mapping = configuration::JsonMapping::from(HashMap::from_iter(vec![
            ("id".to_string(), "account_id".to_string()),
            ("createdAt".to_string(), "created_at".to_string()),
        ]));
        let serde = JsonDataSerde::new(mapping, false).unwrap();
        let data = serde_json::json!({
            "account_id": "abc",
            "created_at": 123,
        });
        let result = serde.extract_values(serde_json::to_vec(&data).unwrap()).await.unwrap();

        assert_eq!(result.get("id"), Some(serde_json::Value::String("abc".to_string())).as_ref());
        assert_eq!(
            result.get("createdAt"),
            Some(serde_json::Value::Number(123i64.into())).as_ref()
        );
    }

    #[tokio::test]
    async fn test_extract_values_with_partial_mapping_non_strict() {
        let mapping = configuration::JsonMapping::from(HashMap::from_iter(vec![(
            "createdAt".to_string(),
            "created_at".to_string(),
        )]));
        let serde = JsonDataSerde::new(mapping, false).unwrap();
        let data = serde_json::json!({
            "account_id": "abc",
            "created_at": 123,
        });
        let result = serde.extract_values(serde_json::to_vec(&data).unwrap()).await.unwrap();

        assert_eq!(
            result.get("account_id"),
            Some(serde_json::Value::String("abc".to_string())).as_ref()
        );
        assert_eq!(
            result.get("createdAt"),
            Some(serde_json::Value::Number(123i64.into())).as_ref()
        );
    }

    #[tokio::test]
    async fn test_extract_values_with_partial_mapping_strict() {
        let mapping = configuration::JsonMapping::from(HashMap::from_iter(vec![(
            "createdAt".to_string(),
            "created_at".to_string(),
        )]));
        let serde = JsonDataSerde::new(mapping, true).unwrap();
        let data = serde_json::json!({
            "account_id": "abc",
            "created_at": 123,
        });
        let result = serde.extract_values(serde_json::to_vec(&data).unwrap()).await.unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result.get("account_id"), None);
        assert_eq!(
            result.get("createdAt"),
            Some(serde_json::Value::Number(123i64.into())).as_ref()
        );
    }

    #[tokio::test]
    async fn test_extract_values_with_empty_data_non_strict() {
        let serde = JsonDataSerde::new(JsonMapping::default(), false).unwrap();
        let result = serde.extract_values(Vec::new()).await.unwrap();

        assert_eq!(result, HashMap::new());
    }
}
