use std::collections::HashMap;

use async_trait::async_trait;
use bytes::Buf;
use prost::encoding::{DecodeContext, WireType};

use crate::{
    configuration,
    ports::data_serde::{DataSerde, ValueMap},
};

/// MAGIC_BYTE(1) | REGISTRY_ID(4) | MESSAGE_INDEXES(1), PAYLOAD(*)
/// For us, the message indexes can only be 1 byte long - we dont do multi-message proto (best
/// practice!).
/// Reference: https://docs.confluent.io/platform/current/schema-registry/fundamentals/serdes-develop/index.html#wire-format
const WIRE_PAYLOAD_OFFSET: usize = 6;

#[derive(Clone)]
pub struct ProtobufDataSerde {
    mapping: configuration::ProtobufMapping,
    wire_extraction_enabled: bool,
}

impl ProtobufDataSerde {
    pub fn new(
        mapping: configuration::ProtobufMapping,
        wire_extraction_enabled: bool,
    ) -> anyhow::Result<Self> {
        if !mapping.0.is_empty() {
            return Ok(ProtobufDataSerde { mapping, wire_extraction_enabled });
        }
        anyhow::bail!("Protobuf mapping cannot be undefined or empty");
    }
}

#[async_trait]
impl DataSerde for ProtobufDataSerde {
    async fn extract_values(&self, data: Vec<u8>) -> anyhow::Result<ValueMap> {
        // If specified, we expect the content to be in the wire format and therefore extract the
        // values with an offset.
        let data = if self.wire_extraction_enabled {
            data.get(WIRE_PAYLOAD_OFFSET..).map(Vec::from).unwrap_or_default()
        } else {
            data
        };

        let context = DecodeContext::default();
        let mut buffer = bytes::Bytes::from(data);
        let tags: HashMap<u32, String> =
            self.mapping.0.iter().map(|(key, tag)| (*tag, key.to_owned())).collect();
        let mut result = ValueMap::new();

        while buffer.has_remaining() {
            let (tag, wire_type) = prost::encoding::decode_key(&mut buffer)?;
            if let Some(key) = tags.get(&tag) {
                let value = match wire_type {
                    // int32, int64, uint32, uint64, sint32, sint64, bool, enum
                    WireType::Varint => {
                        let mut value = i64::default();
                        prost::encoding::int64::merge(
                            wire_type,
                            &mut value,
                            &mut buffer,
                            context.clone(),
                        )?;
                        Some(serde_json::json!(value))
                    }
                    // fixed64, sfixed64, double
                    WireType::SixtyFourBit => {
                        let mut value = f64::default();
                        prost::encoding::double::merge(
                            wire_type,
                            &mut value,
                            &mut buffer,
                            context.clone(),
                        )?;
                        Some(serde_json::json!(value))
                    }
                    // fixed32, sfixed32, float
                    WireType::ThirtyTwoBit => {
                        let mut value = f32::default();
                        prost::encoding::float::merge(
                            wire_type,
                            &mut value,
                            &mut buffer,
                            context.clone(),
                        )?;
                        Some(serde_json::json!(value))
                    }
                    // string, bytes, embedded messages, packed repeated fields
                    WireType::LengthDelimited => {
                        let mut value = String::default();
                        prost::encoding::string::merge(
                            wire_type,
                            &mut value,
                            &mut buffer,
                            context.clone(),
                        )?;
                        Some(serde_json::json!(value))
                    }
                    _ => {
                        let _ = prost::encoding::skip_field(
                            wire_type,
                            tag,
                            &mut buffer,
                            context.clone(),
                        );
                        None
                    }
                };

                if let Some(value) = value {
                    result.insert(key.to_owned(), value);
                }
            } else {
                let _ = prost::encoding::skip_field(wire_type, tag, &mut buffer, context.clone());
            }
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, PartialEq, prost::Message)]
    struct TestMessage {
        #[prost(string, tag = "1")]
        pub field_a: ::prost::alloc::string::String,
        #[prost(string, optional, tag = "2")]
        pub field_b: ::core::option::Option<::prost::alloc::string::String>,
        #[prost(int32, tag = "3")]
        pub field_c: i32,
        #[prost(double, tag = "4")]
        pub field_d: f64,
        #[prost(int64, tag = "5")]
        pub field_e: i64,
        #[prost(float, tag = "6")]
        pub field_f: f32,
    }

    #[tokio::test]
    async fn test_extract_values_without_mapping() {
        assert!(ProtobufDataSerde::new(HashMap::new().into(), false).is_err());
    }

    #[tokio::test]
    async fn test_extract_values_full() {
        let mapping = configuration::ProtobufMapping::from(HashMap::from_iter(vec![
            ("fieldA".to_string(), 1),
            ("fieldB".to_string(), 2),
            ("fieldC".to_string(), 3),
            ("fieldD".to_string(), 4),
            ("fieldE".to_string(), 5),
            ("fieldF".to_string(), 6),
        ]));
        let serde = ProtobufDataSerde::new(mapping, false).unwrap();
        let data = TestMessage {
            field_a: "abc".to_string(),
            field_b: Some("def".to_string()),
            field_c: 123,
            field_d: 456.789,
            field_e: 988,
            field_f: 123.456,
        };
        let result = serde.extract_values(prost::Message::encode_to_vec(&data)).await.unwrap();

        assert_eq!(
            result,
            HashMap::from_iter(vec![
                ("fieldA".to_string(), serde_json::json!("abc")),
                ("fieldB".to_string(), serde_json::json!("def")),
                ("fieldC".to_string(), serde_json::json!(123)),
                ("fieldD".to_string(), serde_json::json!(456.789)),
                ("fieldE".to_string(), serde_json::json!(988)),
                ("fieldF".to_string(), serde_json::json!(123.456f32)),
            ])
        );
    }

    #[tokio::test]
    async fn test_extract_values_empty() {
        let mapping = configuration::ProtobufMapping::from(HashMap::from_iter(vec![
            ("fieldA".to_string(), 1),
            ("fieldB".to_string(), 2),
            ("fieldC".to_string(), 3),
            ("fieldD".to_string(), 4),
        ]));
        let serde = ProtobufDataSerde::new(mapping, false).unwrap();
        let data = TestMessage { field_a: "abc".to_string(), ..Default::default() };
        let result = serde.extract_values(prost::Message::encode_to_vec(&data)).await.unwrap();

        assert_eq!(
            result,
            HashMap::from_iter(vec![("fieldA".to_string(), serde_json::json!("abc"))])
        );
    }

    #[tokio::test]
    async fn test_extract_values_full_wire() {
        let mapping = configuration::ProtobufMapping::from(HashMap::from_iter(vec![
            ("fieldA".to_string(), 1),
            ("fieldB".to_string(), 2),
        ]));
        let serde = ProtobufDataSerde::new(mapping, true).unwrap();
        let data = TestMessage {
            field_a: "abc".to_string(),
            field_b: Some("def".to_string()),
            ..Default::default()
        };
        let mut wire_data = Vec::from(&[0x00, 0x01, 0x02, 0x03, 0x04, 0x00]);
        wire_data.extend(prost::Message::encode_to_vec(&data));
        let result = serde.extract_values(wire_data).await.unwrap();

        assert_eq!(
            result,
            HashMap::from_iter(vec![
                ("fieldA".to_string(), serde_json::json!("abc")),
                ("fieldB".to_string(), serde_json::json!("def")),
            ])
        );
    }
}
