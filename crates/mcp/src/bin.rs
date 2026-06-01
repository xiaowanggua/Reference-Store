fn main() {
    let db_path = std::env::var("REFSTORE_DB")
        .unwrap_or_else(|_| format!("{}/.refstore/refstore.db", std::env::var("HOME").unwrap_or_else(|_| ".".to_string())));
    let db_path = expand_tilde(&db_path);
    let db = refstore_core::Database::open(&db_path).expect("Failed to open database");
    refstore_mcp::run_mcp(db);
}

fn expand_tilde(path: &str) -> std::path::PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        return std::path::PathBuf::from(format!("{}/{}", home, rest));
    }
    std::path::PathBuf::from(path)
}
