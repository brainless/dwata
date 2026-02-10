use std::{env, fs, path::PathBuf};

fn escape_rust_string(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

fn main() {
    println!("cargo:rerun-if-env-changed=DWATA_DEFAULT_GOOGLE_CLIENT_ID");
    println!("cargo:rerun-if-env-changed=DWATA_DEFAULT_GOOGLE_CLIENT_SECRET");

    let client_id = env::var("DWATA_DEFAULT_GOOGLE_CLIENT_ID").unwrap_or_default();
    let client_secret = env::var("DWATA_DEFAULT_GOOGLE_CLIENT_SECRET").ok();

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR not set"));
    let dest = out_dir.join("dwata_oauth_defaults.rs");

    let client_id_escaped = escape_rust_string(&client_id);
    let client_secret_line = match client_secret {
        Some(secret) if !secret.is_empty() => format!(
            "pub const DEFAULT_GOOGLE_CLIENT_SECRET: Option<&str> = Some(\"{}\");\n",
            escape_rust_string(&secret)
        ),
        _ => "pub const DEFAULT_GOOGLE_CLIENT_SECRET: Option<&str> = None;\n".to_string(),
    };

    let contents = format!(
        "pub const DEFAULT_GOOGLE_CLIENT_ID: &str = \"{}\";\n{}",
        client_id_escaped, client_secret_line
    );

    fs::write(dest, contents).expect("Failed to write OAuth defaults");
}
