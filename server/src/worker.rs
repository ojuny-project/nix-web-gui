use std::error::Error;
use std::io::{BufRead, Write};

use serde_json::Value;

use crate::rpc;

pub fn run_stdio() -> Result<(), Box<dyn Error>> {
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    let mut session = None;

    for line in stdin.lock().lines().map_while(Result::ok) {
        if line.trim().is_empty() {
            continue;
        }
        let response = match serde_json::from_str::<Value>(&line) {
            Ok(request) => rpc::handle_with_progress(&mut session, &request, &mut |note| {
                let _ = writeln!(stdout, "{note}");
                let _ = stdout.flush();
            }),
            Err(error) => serde_json::json!({
                "jsonrpc": "2.0",
                "id": Value::Null,
                "error": {
                    "code": -32700,
                    "message": format!("parse error: {error}"),
                },
            }),
        };
        writeln!(stdout, "{response}")?;
        stdout.flush()?;
    }

    Ok(())
}
