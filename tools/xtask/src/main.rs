use std::{
    fs,
    io::{BufRead, BufReader, Write},
    net::{TcpListener, TcpStream},
    path::{Path, PathBuf},
    process::{Command, ExitCode},
};

fn main() -> ExitCode {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let Some(task) = args.first() else {
        eprintln!("usage: cargo run -p xtask -- <check|web-build|web-serve>");
        return ExitCode::FAILURE;
    };

    match task.as_str() {
        "check" => run("cargo", &["test"]),
        "web-build" => web_build(&args[1..]),
        "web-serve" => web_serve(&args[1..]),
        _ => {
            eprintln!("unknown xtask command: {task}");
            ExitCode::FAILURE
        }
    }
}

fn web_build(args: &[String]) -> ExitCode {
    let Some(example) = parse_example(args) else {
        eprintln!("usage: cargo run -p xtask -- web-build --example <name> [--release]");
        return ExitCode::FAILURE;
    };
    let release = args.iter().any(|arg| arg == "--release");

    match build_web_example(&example, release) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("web build failed: {error}");
            ExitCode::FAILURE
        }
    }
}

fn web_serve(args: &[String]) -> ExitCode {
    let Some(example) = parse_example(args) else {
        eprintln!("usage: cargo run -p xtask -- web-serve --example <name> [--release]");
        return ExitCode::FAILURE;
    };
    let release = args.iter().any(|arg| arg == "--release");

    if let Err(error) = build_web_example(&example, release) {
        eprintln!("web build failed: {error}");
        return ExitCode::FAILURE;
    }

    let output_dir = PathBuf::from("target").join("web").join(&example);
    match serve_static(&output_dir, "127.0.0.1:8000") {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("web serve failed: {error}");
            ExitCode::FAILURE
        }
    }
}

fn parse_example(args: &[String]) -> Option<String> {
    args.windows(2)
        .find(|pair| pair[0] == "--example")
        .map(|pair| pair[1].clone())
}

fn build_web_example(example: &str, release: bool) -> Result<(), String> {
    let example_dir = PathBuf::from("examples").join(example);
    if !example_dir.join("Cargo.toml").is_file() {
        return Err(format!("example '{}' not found", example));
    }

    let package = example_package_name(&example_dir)?;
    let profile = if release { "release" } else { "debug" };
    let mut cargo_args = vec![
        "build".to_string(),
        "--target".to_string(),
        "wasm32-unknown-unknown".to_string(),
        "-p".to_string(),
        package.clone(),
    ];
    if release {
        cargo_args.push("--release".to_string());
    }
    run_checked("cargo", &cargo_args)?;

    let output_dir = PathBuf::from("target").join("web").join(example);
    if output_dir.exists() {
        fs::remove_dir_all(&output_dir).map_err(|error| error.to_string())?;
    }
    fs::create_dir_all(&output_dir).map_err(|error| error.to_string())?;

    let wasm_path = PathBuf::from("target")
        .join("wasm32-unknown-unknown")
        .join(profile)
        .join(format!("{}.wasm", package.replace('-', "_")));
    let out_name = package.replace('-', "_");
    run_checked(
        "wasm-bindgen",
        &[
            wasm_path.to_string_lossy().into_owned(),
            "--target".to_string(),
            "web".to_string(),
            "--out-dir".to_string(),
            output_dir.to_string_lossy().into_owned(),
            "--out-name".to_string(),
            out_name.clone(),
        ],
    )
    .map_err(|error| format!("{error}. Install with `cargo install wasm-bindgen-cli`"))?;

    copy_if_exists(&example_dir.join("assets"), &output_dir.join("assets"))?;
    copy_if_exists(
        &example_dir.join("resources"),
        &output_dir.join("resources"),
    )?;
    fs::copy(
        example_dir.join("Seishin.toml"),
        output_dir.join("Seishin.toml"),
    )
    .map_err(|error| error.to_string())?;
    fs::write(output_dir.join("index.html"), web_index_html(&out_name))
        .map_err(|error| error.to_string())?;
    write_web_manifest(&output_dir)?;

    println!("web build written to {}", output_dir.display());
    Ok(())
}

fn write_web_manifest(output_dir: &Path) -> Result<(), String> {
    let mut resources = vec!["Seishin.toml".to_string()];
    collect_manifest_paths(&output_dir.join("resources"), "resources", &mut resources)?;
    resources.sort();
    resources.dedup();

    let mut assets = Vec::new();
    collect_manifest_paths(&output_dir.join("assets"), "assets", &mut assets)?;
    assets.sort();
    assets.dedup();
    let manifest = WebManifest { resources, assets };
    let manifest = serde_json::to_string_pretty(&manifest).map_err(|error| error.to_string())?;
    fs::write(
        output_dir.join("web-manifest.json"),
        format!("{manifest}\n"),
    )
    .map_err(|error| error.to_string())?;

    Ok(())
}

#[derive(serde::Serialize)]
struct WebManifest {
    resources: Vec<String>,
    assets: Vec<String>,
}

fn collect_manifest_paths(
    root: &Path,
    prefix: &str,
    output: &mut Vec<String>,
) -> Result<(), String> {
    if !root.exists() {
        return Ok(());
    }

    collect_manifest_paths_inner(root, root, prefix, output)
}

fn collect_manifest_paths_inner(
    root: &Path,
    current: &Path,
    prefix: &str,
    output: &mut Vec<String>,
) -> Result<(), String> {
    for entry in fs::read_dir(current).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let path = entry.path();
        if path.is_dir() {
            collect_manifest_paths_inner(root, &path, prefix, output)?;
        } else {
            let relative = path.strip_prefix(root).map_err(|error| error.to_string())?;
            output.push(format!("{}/{}", prefix, normalize_path(relative)));
        }
    }

    Ok(())
}

fn example_package_name(example_dir: &Path) -> Result<String, String> {
    let manifest_path = example_dir.join("Cargo.toml");
    let metadata = cargo_metadata::MetadataCommand::new()
        .manifest_path(&manifest_path)
        .no_deps()
        .exec()
        .map_err(|error| format!("failed to read cargo metadata: {error}"))?;
    let manifest_suffix = normalize_path(&manifest_path);

    metadata
        .packages
        .iter()
        .find(|package| {
            normalize_path(package.manifest_path.as_std_path()).ends_with(&manifest_suffix)
        })
        .map(|package| package.name.clone())
        .ok_or_else(|| format!("package name not found in {}", manifest_path.display()))
}

fn normalize_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn copy_if_exists(from: &Path, to: &Path) -> Result<(), String> {
    if !from.exists() {
        return Ok(());
    }

    copy_dir(from, to)
}

fn copy_dir(from: &Path, to: &Path) -> Result<(), String> {
    fs::create_dir_all(to).map_err(|error| error.to_string())?;
    for entry in fs::read_dir(from).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let source = entry.path();
        let target = to.join(entry.file_name());
        if source.is_dir() {
            copy_dir(&source, &target)?;
        } else {
            fs::copy(&source, &target).map_err(|error| error.to_string())?;
        }
    }

    Ok(())
}

fn web_index_html(out_name: &str) -> String {
    format!(
        r#"<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>Seishin Web</title>
    <style>
      html, body {{ margin: 0; min-height: 100%; background: #111; }}
      canvas {{ display: block; margin: auto; outline: none; }}
    </style>
  </head>
  <body>
    <script type="module">
      import init from './{out_name}.js';
      init();
    </script>
  </body>
</html>
"#
    )
}

fn run_checked(program: &str, args: &[String]) -> Result<(), String> {
    let status = Command::new(program)
        .args(args)
        .status()
        .map_err(|error| format!("failed to run {program}: {error}"))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!("{program} exited with status {status}"))
    }
}

fn run(program: &str, args: &[&str]) -> ExitCode {
    match Command::new(program).args(args).status() {
        Ok(status) if status.success() => ExitCode::SUCCESS,
        Ok(status) => ExitCode::from(status.code().unwrap_or(1) as u8),
        Err(error) => {
            eprintln!("failed to run {program}: {error}");
            ExitCode::FAILURE
        }
    }
}

fn serve_static(root: &Path, address: &str) -> Result<(), String> {
    let root = fs::canonicalize(root).map_err(|error| error.to_string())?;
    let listener = TcpListener::bind(address).map_err(|error| error.to_string())?;

    println!("serving {} at http://{address}", root.display());
    println!("press Ctrl+C to stop");

    for stream in listener.incoming() {
        let stream = stream.map_err(|error| error.to_string())?;
        let root = root.clone();

        std::thread::spawn(move || {
            if let Err(error) = handle_static_request(stream, &root) {
                eprintln!("static request failed: {error}");
            }
        });
    }

    Ok(())
}

fn handle_static_request(mut stream: TcpStream, root: &Path) -> Result<(), String> {
    let mut request_line = String::new();
    {
        let mut reader = BufReader::new(&stream);
        reader
            .read_line(&mut request_line)
            .map_err(|error| error.to_string())?;
    }

    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or_default();
    let request_path = parts.next().unwrap_or("/");

    if method != "GET" && method != "HEAD" {
        write_response(&mut stream, "405 Method Not Allowed", "text/plain", b"")?;
        return Ok(());
    }

    let Some(file_path) = resolve_request_path(root, request_path) else {
        write_response(&mut stream, "403 Forbidden", "text/plain", b"forbidden")?;
        return Ok(());
    };

    if !file_path.is_file() {
        write_response(&mut stream, "404 Not Found", "text/plain", b"not found")?;
        return Ok(());
    }

    let bytes = fs::read(&file_path).map_err(|error| error.to_string())?;
    write_response(&mut stream, "200 OK", content_type(&file_path), &bytes)
}

fn resolve_request_path(root: &Path, request_path: &str) -> Option<PathBuf> {
    let path = request_path
        .split_once('?')
        .map_or(request_path, |(path, _)| path);
    let path = path.trim_start_matches('/');
    let path = if path.is_empty() { "index.html" } else { path };
    let mut resolved = root.to_path_buf();

    for segment in path.split('/') {
        if segment.is_empty() || segment == "." {
            continue;
        }
        if segment == ".." || segment.contains('\\') {
            return None;
        }
        resolved.push(segment);
    }

    Some(resolved)
}

fn write_response(
    stream: &mut TcpStream,
    status: &str,
    content_type: &str,
    body: &[u8],
) -> Result<(), String> {
    write!(
        stream,
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    )
    .map_err(|error| error.to_string())?;
    stream.write_all(body).map_err(|error| error.to_string())
}

fn content_type(path: &Path) -> &'static str {
    match path.extension().and_then(|extension| extension.to_str()) {
        Some("html") => "text/html; charset=utf-8",
        Some("js") => "text/javascript; charset=utf-8",
        Some("wasm") => "application/wasm",
        Some("css") => "text/css; charset=utf-8",
        Some("json") => "application/json; charset=utf-8",
        Some("toml") => "text/plain; charset=utf-8",
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("svg") => "image/svg+xml",
        _ => "application/octet-stream",
    }
}
