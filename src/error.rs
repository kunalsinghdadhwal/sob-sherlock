pub fn error_json(code: &str, msg: &str) -> String {
    let esc = msg.replace('\\', "\\\\").replace('"', "\\\"");
    format!(
        r#"{{"ok":false,"error":{{"code":"{}","message":"{}"}}}}"#,
        code, esc
    )
}
