//! Runtime helpers shared by generated Rustify programs.

pub type JsonValue = serde_json::Value;

pub mod json {
    use super::JsonValue;

    pub fn parse(input: String) -> Result<JsonValue, String> {
        serde_json::from_str(&input).map_err(|error| error.to_string())
    }

    pub fn stringify(value: JsonValue) -> Result<String, String> {
        serde_json::to_string(&value).map_err(|error| error.to_string())
    }

    pub fn stringify_pretty(value: JsonValue) -> Result<String, String> {
        serde_json::to_string_pretty(&value).map_err(|error| error.to_string())
    }
}

pub mod async_runtime {
    use std::time::Duration;

    pub async fn sleep(milliseconds: f64) {
        let milliseconds = if milliseconds.is_finite() {
            milliseconds.max(0.0)
        } else {
            0.0
        };
        futures_timer::Delay::new(Duration::from_secs_f64(milliseconds / 1_000.0)).await;
    }
}

pub fn console_log<T: std::fmt::Debug>(value: T) {
    println!("{value:?}");
}

pub fn js_truthy(value: bool) -> bool {
    value
}

#[cfg(test)]
mod tests {
    use super::{async_runtime, json};

    #[test]
    fn parses_and_stringifies_json_safely() {
        let value = json::parse("{\"name\":\"Rustify\"}".to_owned()).unwrap();
        assert_eq!(value["name"], "Rustify");
        assert_eq!(json::stringify(value).unwrap(), "{\"name\":\"Rustify\"}");
    }

    #[test]
    fn returns_json_errors_as_strings() {
        assert!(json::parse("{invalid}".to_owned()).is_err());
    }

    #[test]
    fn async_sleep_future_completes() {
        futures_lite::future::block_on(async_runtime::sleep(1.0));
    }
}
