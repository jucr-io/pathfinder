use std::collections::HashMap;

use apollo_parser::cst::{self, CstNode, Definition, Selection};

#[derive(Debug, Clone, PartialEq)]
pub struct SubscriptionOperation {
    pub name: String,
    pub arguments: Arguments,
}

pub type Arguments = HashMap<String, String>;

impl SubscriptionOperation {
    pub fn from_query(query: &str, variables: Option<serde_json::Value>) -> Option<Self> {
        let parsed = apollo_parser::Parser::new(query).parse();
        tracing::debug! { event = "parsed_graphql_query", query, ?variables, parsed=?&parsed };

        // this is super deep but everything is optional and quite weirdly chained in the AST
        // models
        for definition in parsed.document().definitions() {
            let operation =
                if let Definition::OperationDefinition(operation_definition) = definition {
                    operation_definition
                } else {
                    continue;
                };

            if let Some(selection_set) = operation.selection_set().map(|s| s.selections()) {
                for selection in selection_set {
                    if let Selection::Field(field) = selection {
                        let operation_name = if let Some(name) = field.name() {
                            name.text().to_string()
                        } else {
                            continue;
                        };

                        let mut arguments = Arguments::new();
                        if let Some(args) = field.arguments().map(|a| a.arguments()) {
                            // Apply map/filter on the input only when we need it.
                            // If there are no arguments we would also not need to process the
                            // input.
                            let input_variables =
                                if let Some(serde_json::Value::Object(ref map)) = variables {
                                    map.iter()
                                        .filter_map(|(k, v)| {
                                            if let serde_json::Value::String(value) = v {
                                                Some((k.clone(), value.clone()))
                                            } else {
                                                None
                                            }
                                        })
                                        .collect::<HashMap<String, String>>()
                                } else {
                                    HashMap::new()
                                };

                            for arg in args {
                                let arg_name = if let Some(name) = arg.name() {
                                    name.text().to_string()
                                } else {
                                    continue;
                                };

                                let value = if let Some(value) = arg.value() {
                                    value
                                } else {
                                    continue;
                                };

                                match value {
                                    cst::Value::StringValue(s) => {
                                        let value = s.syntax().text().to_string().replace('"', "");
                                        arguments.insert(arg_name, value);
                                    }
                                    cst::Value::Variable(v) => {
                                        if let Some(key) = v.name().map(|n| n.text().to_string()) {
                                            if let Some(value) = input_variables.get(&key) {
                                                arguments.insert(arg_name, value.to_string());
                                            }
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }

                        return Some(Self { name: operation_name, arguments });
                    }
                }
            } else {
                continue;
            };
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_subscription_operation_from_query() {
        let cases = [
            (
                r#"
                subscription ChargingSessionChanged($chargingSessionChangedId: ID!) {
                  chargingSessionChanged(id: $chargingSessionChangedId) {
                    id
                    status
                    startedAt
                  }
                }
                "#,
                serde_json::json!({ "chargingSessionChangedId": "id1" }),
                Some(SubscriptionOperation {
                    name: "chargingSessionChanged".to_string(),
                    arguments: vec![("id".to_string(), "id1".to_string())].into_iter().collect(),
                }),
            ),
            (
                r#"
                subscription ChargingSessionChanged($id: ID!) {
                  chargingSessionChanged(id: $id) {
                    id
                    status
                    startedAt
                  }
                }
                "#,
                serde_json::json!({ "id": "id1" }),
                Some(SubscriptionOperation {
                    name: "chargingSessionChanged".to_string(),
                    arguments: vec![("id".to_string(), "id1".to_string())].into_iter().collect(),
                }),
            ),
            (
                r#"
                subscription ChargingSessionChanged($id: ID!) {
                  chargingSessionChanged(id: "123id") {
                    id
                    status
                    startedAt
                  }
                }
                "#,
                serde_json::json!({ "id": "id1" }),
                Some(SubscriptionOperation {
                    name: "chargingSessionChanged".to_string(),
                    arguments: vec![("id".to_string(), "123id".to_string())].into_iter().collect(),
                }),
            ),
            (
                r#"
                subscription ChargingSessionChanged {
                  chargingSessionChanged(id: "123id") {
                    id
                    status
                    startedAt
                  }
                }
                "#,
                serde_json::json!({}),
                Some(SubscriptionOperation {
                    name: "chargingSessionChanged".to_string(),
                    arguments: vec![("id".to_string(), "123id".to_string())].into_iter().collect(),
                }),
            ),
            (
                r#"
                subscription {
                  chargingSessionChanged(id: "123id") {
                    id
                    status
                    startedAt
                  }
                }
                "#,
                serde_json::json!({}),
                Some(SubscriptionOperation {
                    name: "chargingSessionChanged".to_string(),
                    arguments: vec![("id".to_string(), "123id".to_string())].into_iter().collect(),
                }),
            ),
            (
                r#"
                subscription {}
                "#,
                serde_json::json!({}),
                None,
            ),
        ];

        for (query, variables, result) in cases.iter() {
            let operation = SubscriptionOperation::from_query(query, Some(variables.clone()));
            assert_eq!(operation, *result);
        }

        assert!(true);
    }
}
