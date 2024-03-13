/// Macro for generating a JSON enum.
macro_rules! json_enum {
    ($item:item) => {
        #[serde_with::serde_as]
        #[serde_with::skip_serializing_none]
        #[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq, Eq)]
        #[serde(untagged)]
        $item
    };
}

/// Macro for generating a JSON enum as a map key.
macro_rules! json_enum_key {
    ($item:item) => {
        #[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq, Eq, Hash)]
        $item
    };
}

/// Macro for generating a JSON struct without a default impl.
macro_rules! json_struct_nodefault {
    ($item:item) => {
        #[serde_with::serde_as]
        #[serde_with::skip_serializing_none]
        #[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq, Eq)]
        $item
    };
}

/// Macro for generating a JSON struct.
macro_rules! json_struct {
    ($item:item) => {
        $crate::json::json_struct_nodefault! {
            #[derive(Default)]
            $item
        }
    };
}

pub(crate) use {json_enum, json_enum_key, json_struct, json_struct_nodefault};