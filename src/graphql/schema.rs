use config::Config;
use derive_more::derive::Into;
use indoc::indoc;

use crate::configuration::{Listener, Listeners};

#[derive(Debug, Clone, Into)]
pub struct Schema(pub String);

impl TryFrom<&Config> for Schema {
    type Error = anyhow::Error;

    fn try_from(value: &Config) -> Result<Self, Self::Error> {
        let schema = build_from_config(value)?;
        Ok(schema)
    }
}

fn build_from_config(config: &Config) -> anyhow::Result<Schema> {
    let listeners: Listeners = config.get("listeners")?;
    let entities = listeners.iter().map(entity).collect::<Vec<String>>().join("\n\n");

    let schema_parts: Vec<String> = vec![
        header_link(config.get("link_version")?),
        header_federation_extension(config.get("federation_version")?),
        entities,
        subscriptions(&listeners),
    ];

    let schema = schema_parts.join("\n\n\n");
    Ok(Schema(schema))
}

fn entity(listener: &Listener) -> String {
    format!(
        indoc! {"
        type {} @key(fields: \"{}\") {{
          {}: ID!
        }}"},
        &listener.entity_name, listener.id_key, listener.id_key
    )
}

fn subscription_description(listener: &Listener) -> String {
    listener.description.as_ref().map(|d| d.to_string()).unwrap_or_else(|| {
        format!("Subscription for changes on the {} entity.", listener.entity_name)
    })
}

fn subscriptions(listeners: &Listeners) -> String {
    let operations = listeners
        .iter()
        .map(|l| {
            format!(
                indoc! {"
                \"\"\"
                  {}
                \"\"\"
                  {}({}: ID!): {}
              "},
                subscription_description(l),
                l.operation,
                l.id_key,
                l.entity_name
            )
        })
        .collect::<Vec<String>>()
        .join("\n");

    format!(
        indoc! {"
        type Subscription {{
        {}
        }}"},
        operations
    )
}

fn header_link(link_version: String) -> String {
    format!(
        indoc! {"
      schema
        @link(url: \"https://specs.apollo.dev/link/v{}\")
      {{
        subscription: Subscription
      }}"},
        link_version
    )
}

fn header_federation_extension(federation_version: String) -> String {
    format!(
        indoc! {"
      extend schema
        @link(url: \"https://specs.apollo.dev/federation/v{}\", import: [\"@key\"])"},
        federation_version
    )
}
