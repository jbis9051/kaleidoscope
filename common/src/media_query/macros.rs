use chrono::format::parse;
use std::fmt::Display;
use serde_json::json;
use crate::media_query::JoinableTable;

pub trait DSLType {
    type RustType;

    const VARIANTS: &'static [&'static str];
    fn to_str(&self) -> &'static str;
    fn from_str(s: &str) -> Option<Self>
    where
        Self: Sized;
    fn parse(s: &str) -> Result<Self::RustType, String>;
}

#[macro_export]
macro_rules! dsl_types {
    // handle multiple enums separated by semicolons
    ($($dsl:ident($name:ident, $rust:ty) { $($variant:ident = $str:literal),*, |$lambda:tt| $body:block };)* $(;)?) => {
        $(
            dsl_types!($dsl($name, $rust) { $($variant = $str),*, |$lambda| { $body } });
        )*

        macro_rules! dsl_name_to_type {
            $(($dsl) => {
                $name
            };)*
        }
    };
    ($dsl:ident($name:ident, $rust:ty) { $($variant:ident = $str:literal),*, |$lambda:tt| $body:block }) => {
        #[derive(Debug, Clone, PartialEq)]
        enum $name {
            $($variant),*
        }

        impl DSLType for $name {
            type RustType = $rust;

            // [ "<op>", ... ] }
            const VARIANTS: &'static [&'static str] = &[$($str),*];

            fn to_str(&self) -> &'static str {
                match self {
                    $($name::$variant => $str),*
                }
            }

            fn from_str(s: &str) -> Option<Self> {
                $(
                    if s.starts_with($str) {
                        return Some($name::$variant);
                    }
                )*
                None
            }

            fn parse($lambda: &str) -> Result<Self::RustType, std::string::String> {
                $body
            }
        }
    };
}

// TODO this should return Vec<(String, String, String)> for <variant> <op> <value>, because this is improperly ambiguous currently: variant:!"%foo" == variant:!%"foo"
pub fn parse_filter(query_string: &str) -> Result<Vec<String>, String> {
    let chars = query_string.chars().collect::<Vec<_>>();

    let mut filters = vec![];

    let mut quote = None;
    let mut curr = 0;

    let mut curr_filter = String::new();

    while curr < query_string.len() {
        let c = chars[curr];
        match c {
            _ if c.is_whitespace() => {
                if quote.is_none() {
                    if !curr_filter.is_empty() {
                        filters.push(curr_filter.clone());
                        curr_filter.clear();
                    }
                } else {
                    curr_filter.push(c);
                }
            }
            '\\' => {
                // skip next character
                if curr + 1 >= chars.len() {
                    return Err("unexpected end of string".to_string());
                }

                curr_filter.push(chars[curr + 1]);
                curr += 1;
            }
            '"' | '\'' => {
                if let Some(q) = quote {
                    if q == c {
                        quote = None;
                    } else {
                        curr_filter.push(c);
                    }
                } else {
                    quote = Some(c);
                }
            }
            _ => {
                curr_filter.push(c);
            }
        }
        curr += 1;
    }

    if let Some(q) = quote {
        return Err(format!("unmatched quote: {}", q));
    }

    if !curr_filter.is_empty() {
        filters.push(curr_filter);
    }

    Ok(filters)
}

pub fn format_value(value: &str) -> String {
    let mut out = String::new();
    out.push('\'');
    for c in value.chars() {
        match c {
            '\'' => {
                out.push('\\');
                out.push('\'');
            }
            _ => out.push(c),
        }
    }
    out.push('\'');
    out
}

#[macro_export]
macro_rules! query_dsl {

    (
        $name: ident($enum_name:ident) { 
            $($field:tt($dsl:tt, $variant:ident, [$($table:ident,)*]),)* 
        }
    ) => {
        #[derive(Clone, Default)]
        pub struct $name {
            filters: Vec<$enum_name>
        }

        #[derive(PartialEq, Clone, Debug)]
        pub enum $enum_name {
            $(
                $variant(dsl_name_to_type!($dsl), <dsl_name_to_type!($dsl) as DSLType>::RustType),
            )*
        }

        impl $enum_name {
                // {
                //   "fields": { "<name>": "<dsl_type>", ... },
                //   "dsl_types": { "<dsl>": [ "<op>", ... ] }
                // }
            pub fn describe() -> String {
                    let mut out = String::new();

                    out.push_str("{ \"fields\": {");

                    let fields = [$(concat!("\"", stringify!($field), "\": \"", stringify!($dsl), "\" ")),*];
                    out.push_str(fields.join(", ").as_str());
                    out.push_str("}, \"dsl_types\": {");

                    let mut dsl_types = Vec::new();

                    $(
                        dsl_types.push(format!("\"{}\": {:?}", stringify!($dsl), <dsl_name_to_type!($dsl) as DSLType>::VARIANTS));
                    )*

                    out.push_str(dsl_types.join(", ").as_str());
                    out.push_str("}}");
                    out
            }
            
            pub fn tables(&self) -> Vec<JoinableTable> { 
                match self {
                    $(
                        $enum_name::$variant(_, _) => {
                            vec![$(JoinableTable::$table),*]
                        }
                    )*
                }
            }

        }

        impl std::fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                let mut first = true;


                for filter in &self.filters {
                    if !first {
                        write!(f, " ")?;
                    }
                    first = false;

                    match filter {
                        $(
                            $enum_name::$variant(op, value) => {
                                let mut value = format!("{}", value);

                                if value.contains(char::is_whitespace){
                                    value = format_value(&value);
                                }

                                write!(f, "{}:{}{}", stringify!($field), op.to_str(), value)?;
                            }
                        )*
                    };
                }

                Ok(())
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::Debug::fmt(self, f)
            }
        }

        impl std::str::FromStr for $name {
            type Err = String;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                let filters = parse_filter(s)?;

                let mut out = $name {
                    filters: Vec::new()
                };


                for filter in filters {
                    // <variant>:<op><value>
                    if let Some((key, rest)) = filter.split_once(':') {
                        match key {
                            $(
                                stringify!($field) => {
                                    if let Some(varient) = <dsl_name_to_type!($dsl)>::from_str(rest) {
                                        let len = varient.to_str().len();
                                        out.filters.push($enum_name::$variant(varient, <dsl_name_to_type!($dsl)>::parse(&rest[len..])?));
                                    } else {
                                        return Err(format!("invalid operator for key '{}': {}", key, rest));
                                    }
                                }
                            )*
                            _ => return Err(format!("unexpected key '{}'", key)),
                        }
                    } else {
                        return Err(format!("invalid filter no colon: {}", filter));
                    }
                }

                Ok(out)
            }
        }

        impl Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                struct FormatWrapped<'a, D: 'a> {
                    inner: &'a D,
                }

                impl<D: fmt::Debug> fmt::Display for FormatWrapped<'_, D> {
                    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                        self.inner.fmt(f)
                    }
                }

                serializer.collect_str(&FormatWrapped { inner: &self })
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                struct QueryDsVisitor;

                impl<'de> Visitor<'de> for QueryDsVisitor {
                    type Value = $name;

                    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                        write!(formatter, "a string in the format '<variant>:<op><value> <variant>:<op><value> ...'")
                    }

                    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
                    where
                        E: de::Error,
                    {
                       value.parse().map_err(de::Error::custom)
                    }

                }

                deserializer.deserialize_str(QueryDsVisitor)
            }
        }
    };
}