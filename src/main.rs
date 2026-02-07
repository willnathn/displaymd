use axum::{
    Router,
    extract::{Path, State},
    response::{Html, Redirect},
    routing::get,
};
use clap::Parser;
use std::io;
use std::{collections::BTreeMap, path::PathBuf, sync::Arc};

#[derive(Parser)]
#[command(name = "displaymd")]
struct Args {
    /// Directory containing markdown files
    #[arg(default_value = ".")]
    dir: PathBuf,

    /// Port to serve on
    #[arg(short, long, default_value = "3000")]
    port: u16,

    /// Home file to show at (relative path)
    #[arg(short = 'H', long, default_value = "README.md")]
    home: String,
}

struct AppState {
    root: PathBuf,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let root = args.dir.canonicalize().expect("invalid path");

    println!("Serving: {}", root.display());

    let state = Arc::new(AppState { root });

    let app = Router::new()
        .route("/", get(index))
        .route("/view/{*path}", get(view))
        .with_state(state);

    let addr = format!("127.0.0.1:{}", args.port);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    println!("http://{addr}");
    axum::serve(listener, app).await.unwrap();
}

async fn index(State(state): State<Arc<AppState>>) -> Redirect {
    let files = collect_md_files(&state.root);
    let first = files.keys().next().cloned().unwrap_or_default();
    Redirect::to(&format!("/view/{first}"))
}

// check a file exists in a subdir of root and
fn file_to_markdown(root: &PathBuf, path: &String) -> io::Result<String> {
    let file_path = match root.join(path).canonicalize() {
        Ok(p) if p.starts_with(root) => p,
        _ => return Err(io::ErrorKind::NotFound.into()),
    };
    std::fs::read_to_string(&file_path)
}

async fn view(State(state): State<Arc<AppState>>, Path(path): Path<String>) -> Html<String> {
    let content = match file_to_markdown(&state.root, &path) {
        Ok(c) => c,
        _ => return Html("<p>File not found<p>".to_string()),
    };
    let files = collect_md_files(&state.root);
    let sidebar = build_sidebar(&files, &path);

    Html(format!(
        r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <title>{path}</title>
    <style>{CSS}</style>
</head>
<body>
    <nav class="sidebar">{sidebar}</nav>
    <main class="content">{content}</main>
</body>
</html>"#
    ))
}

fn collect_md_files(root: &PathBuf) -> BTreeMap<String, String> {
    let mut files = BTreeMap::new();
    collect_recursive(root, root, &mut files);
    files
}

fn collect_recursive(root: &PathBuf, dir: &PathBuf, files: &mut BTreeMap<String, String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_recursive(root, &path, files);
        } else if path.extension().is_some_and(|e| e == "md") {
            let relative = path.strip_prefix(root).unwrap();
            let key = relative.to_string_lossy().to_string();
            let name = path.file_stem().unwrap().to_string_lossy().to_string();
            files.insert(key, name);
        }
    }
}

fn build_sidebar(files: &BTreeMap<String, String>, current: &str) -> String {
    let mut out = String::from("<ul>");
    for (path, name) in files {
        let class = if path == current {
            " class=\"active\""
        } else {
            ""
        };
        out.push_str(&format!(
            r#"<li{class}><a href="/view/{path}">{name}</a></li>"#
        ));
    }
    out.push_str("</ul>");
    out
}

const CSS: &str = include_str!("../static/style.css");
