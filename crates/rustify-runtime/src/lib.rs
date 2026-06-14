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

/// Invoca una función de JavaScript/TypeScript usando Node.js mediante IPC/JSON de forma síncrona
pub fn call_js_fallback<T: serde::de::DeserializeOwned>(
    entry_path: &str,
    func_name: &str,
    args: &[serde_json::Value],
) -> Result<T, String> {
    use std::io::Write;
    use std::process::{Command, Stdio};

    // Script en Node.js que carga dinámicamente el módulo y ejecuta la función especificada
    let node_script = r#"
import { stdin, stdout } from 'node:process';
import readline from 'node:readline';
const rl = readline.createInterface({ input: stdin, output: stdout, terminal: false });
rl.on('line', async (line) => {
  try {
    const { entry, func, args } = JSON.parse(line);
    const mod = await import('./' + entry);
    const fn = mod[func];
    if (typeof fn !== 'function') throw new Error('Function not found: ' + func);
    const result = await fn(...args);
    console.log(JSON.stringify({ success: true, result }));
  } catch (err) {
    console.log(JSON.stringify({ success: false, error: err.message }));
  }
  process.exit(0);
});
"#;

    let mut child = Command::new("node")
        .args(["--experimental-transform-types", "-e", node_script])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|e| format!("No se pudo iniciar Node.js: {}", e))?;

    let payload = serde_json::json!({
        "entry": entry_path,
        "func": func_name,
        "args": args
    });

    let payload_str = serde_json::to_string(&payload).unwrap();

    {
        let stdin = child
            .stdin
            .as_mut()
            .ok_or("No se pudo obtener stdin del proceso Node.js")?;
        writeln!(stdin, "{}", payload_str).map_err(|e| e.to_string())?;
    }

    let output = child.wait_with_output().map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err("El proceso Node.js falló".to_owned());
    }

    let response_str = String::from_utf8_lossy(&output.stdout);
    let response: serde_json::Value = serde_json::from_str(response_str.trim()).map_err(|e| {
        format!(
            "Respuesta JSON inválida de Node.js: {} (recibido: '{}')",
            e, response_str
        )
    })?;

    if response["success"].as_bool().unwrap_or(false) {
        let result = serde_json::from_value(response["result"].clone())
            .map_err(|e| format!("No se pudo deserializar el resultado: {}", e))?;
        Ok(result)
    } else {
        Err(response["error"]
            .as_str()
            .unwrap_or("Error desconocido en V8/Node.js")
            .to_owned())
    }
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
