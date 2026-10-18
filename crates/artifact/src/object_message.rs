// JSON control messages are objects. Serde's default struct visitor also accepts
// positional arrays, which the published schemas deliberately reject.
macro_rules! object_message {
    ($(#[$doc:meta])* pub struct $name:ident { $( $(#[$attr:meta])* pub $field:ident: $ty:ty, )* }) => {
        $(#[$doc])*
        #[derive(Debug, Clone, PartialEq, Eq, Serialize)]
        #[serde(rename_all = "camelCase", deny_unknown_fields)]
        pub struct $name { $( $(#[$attr])* pub $field: $ty, )* }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                #[derive(Deserialize)]
                #[serde(rename_all = "camelCase", deny_unknown_fields)]
                struct Fields { $( $(#[$attr])* $field: $ty, )* }
                let fields = serde_json::Map::<String, serde_json::Value>::deserialize(deserializer)?;
                let _fields: Fields = serde_json::from_value(serde_json::Value::Object(fields))
                    .map_err(serde::de::Error::custom)?;
                Ok(Self { $( $field: _fields.$field, )* })
            }
        }
    };
}
