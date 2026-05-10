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
        eprintln!(
            "usage: cargo run -p xtask -- <check|check-project|dependency-audit|list-components|web-build|web-serve>"
        );
        return ExitCode::FAILURE;
    };

    match task.as_str() {
        "check" => run("cargo", &["test"]),
        "check-project" => check_project(&args[1..]),
        "dependency-audit" => dependency_audit(),
        "list-components" => list_components(&args[1..]),
        "web-build" => web_build(&args[1..]),
        "web-serve" => web_serve(&args[1..]),
        _ => {
            eprintln!("unknown xtask command: {task}");
            ExitCode::FAILURE
        }
    }
}

fn check_project(args: &[String]) -> ExitCode {
    let Some(example) = parse_example(args) else {
        eprintln!("usage: cargo run -p xtask -- check-project --example <name>");
        return ExitCode::FAILURE;
    };

    match validate_example_project(&example) {
        Ok(report) => {
            println!("project check ok: examples/{example}");
            println!(
                "registered components: {}",
                format_list(&report.registered_components)
            );
            println!("scene components: {}", format_list(&report.used_components));
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("project check failed: {error}");
            ExitCode::FAILURE
        }
    }
}

fn list_components(args: &[String]) -> ExitCode {
    let Some(example) = parse_example(args) else {
        eprintln!("usage: cargo run -p xtask -- list-components --example <name>");
        return ExitCode::FAILURE;
    };

    match registered_components_from_project(&PathBuf::from("examples").join(&example)) {
        Ok(components) => {
            for component in components {
                println!("{component}");
            }
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("component listing failed: {error}");
            ExitCode::FAILURE
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ProjectValidationReport {
    registered_components: Vec<String>,
    used_components: Vec<String>,
}

fn validate_example_project(example: &str) -> Result<ProjectValidationReport, String> {
    let example_dir = PathBuf::from("examples").join(example);
    if !example_dir.join("Cargo.toml").is_file() {
        return Err(format!("example '{example}' not found"));
    }
    if !example_dir.join("Seishin.toml").is_file() {
        return Err(format!("examples/{example} is missing Seishin.toml"));
    }

    let registered_components = registered_components_from_project(&example_dir)?;
    let used_components = custom_component_types_from_project(&example_dir)?;
    let missing = missing_components(&registered_components, &used_components);

    if !missing.is_empty() {
        return Err(format!(
            "unregistered components: {}. registered components: {}",
            format_list(&missing),
            format_list(&registered_components)
        ));
    }

    Ok(ProjectValidationReport {
        registered_components,
        used_components,
    })
}

fn registered_components_from_project(example_dir: &Path) -> Result<Vec<String>, String> {
    let main_path = example_dir.join("src").join("main.rs");
    let main_source = fs::read_to_string(&main_path)
        .map_err(|error| format!("failed to read {}: {error}", main_path.display()))?;
    let components_dir = example_dir.join("src").join("components");

    registered_components_from_sources(&main_source, |module| {
        let module_path = components_dir.join(format!("{module}.rs"));
        fs::read_to_string(module_path).ok()
    })
}

fn custom_component_types_from_project(example_dir: &Path) -> Result<Vec<String>, String> {
    let mut components = std::collections::BTreeSet::new();
    collect_custom_component_types(&example_dir.join("resources"), &mut components)?;
    Ok(components.into_iter().collect())
}

fn collect_custom_component_types(
    root: &Path,
    output: &mut std::collections::BTreeSet<String>,
) -> Result<(), String> {
    if !root.exists() {
        return Ok(());
    }

    for entry in fs::read_dir(root).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let path = entry.path();
        if path.is_dir() {
            collect_custom_component_types(&path, output)?;
            continue;
        }

        if path.extension().and_then(|extension| extension.to_str()) != Some("toml") {
            continue;
        }

        let source = fs::read_to_string(&path)
            .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
        output.extend(custom_component_types_from_toml_source(&source));
    }

    Ok(())
}

fn registered_components_from_sources<F, S>(
    main_source: &str,
    mut module_source: F,
) -> Result<Vec<String>, String>
where
    F: FnMut(&str) -> Option<S>,
    S: AsRef<str>,
{
    let modules = component_modules_from_main_source(main_source);
    let mut components = Vec::new();

    for module in modules {
        let source = module_source(&module)
            .ok_or_else(|| format!("component module '{module}' was registered but not found"))?;
        let names = component_names_from_module_source(source.as_ref());
        if names.is_empty() {
            return Err(format!(
                "component module '{module}' does not expose a registered component name"
            ));
        }
        extend_unique(&mut components, names);
    }

    Ok(components)
}

fn component_modules_from_main_source(source: &str) -> Vec<String> {
    let marker = ".add_component(";
    let component_marker = "components::";
    let mut modules = Vec::new();
    let mut remaining = source;

    while let Some(index) = remaining.find(marker) {
        let after_call = &remaining[index + marker.len()..];
        if let Some(component_index) = after_call.find(component_marker) {
            let after_components = &after_call[component_index + component_marker.len()..];
            let module = take_rust_ident(after_components);
            if !module.is_empty() && !modules.contains(&module) {
                modules.push(module);
            }
        }
        if after_call.is_empty() {
            break;
        }
        remaining = &after_call[1..];
    }

    modules
}

fn component_names_from_module_source(source: &str) -> Vec<String> {
    let mut names = Vec::new();
    for marker in [
        "component_factory(",
        "register_component_factory(",
        "register_component(",
    ] {
        extend_unique(&mut names, quoted_strings_after(source, marker));
    }
    names
}

fn custom_component_types_from_toml_source(source: &str) -> Vec<String> {
    let mut components = std::collections::BTreeSet::new();
    let mut in_custom_component_section = false;

    for raw_line in source.lines() {
        let line = raw_line
            .split_once('#')
            .map_or(raw_line, |(line, _)| line)
            .trim();
        if line.is_empty() {
            continue;
        }

        if line.starts_with('[') && line.ends_with(']') {
            let section = line.trim_matches(['[', ']']);
            in_custom_component_section =
                section.starts_with("components.") || section.contains(".components.");
            continue;
        }

        if !in_custom_component_section {
            continue;
        }

        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        if key.trim() != "type" {
            continue;
        }
        if let Some(component) = parse_quoted_string(value.trim()) {
            components.insert(component);
        }
    }

    components.into_iter().collect()
}

fn missing_components(registered: &[String], used: &[String]) -> Vec<String> {
    let registered = registered
        .iter()
        .map(String::as_str)
        .collect::<std::collections::BTreeSet<_>>();
    used.iter()
        .filter(|component| !registered.contains(component.as_str()))
        .cloned()
        .collect()
}

fn quoted_strings_after(source: &str, marker: &str) -> Vec<String> {
    let mut values = Vec::new();
    let mut remaining = source;

    while let Some(index) = remaining.find(marker) {
        let after_marker = &remaining[index + marker.len()..];
        let Some(value_start) = after_marker.find('"') else {
            if after_marker.is_empty() {
                break;
            }
            remaining = &after_marker[1..];
            continue;
        };
        let after_quote = &after_marker[value_start + 1..];
        let Some(value_end) = after_quote.find('"') else {
            if after_marker.is_empty() {
                break;
            }
            remaining = &after_marker[1..];
            continue;
        };
        let value = after_quote[..value_end].to_string();
        if !values.contains(&value) {
            values.push(value);
        }
        remaining = &after_quote[value_end + 1..];
    }

    values
}

fn take_rust_ident(source: &str) -> String {
    source
        .chars()
        .take_while(|character| character.is_ascii_alphanumeric() || *character == '_')
        .collect()
}

fn extend_unique(output: &mut Vec<String>, values: Vec<String>) {
    for value in values {
        if !output.contains(&value) {
            output.push(value);
        }
    }
}

fn format_list(values: &[String]) -> String {
    if values.is_empty() {
        "none".to_string()
    } else {
        values.join(", ")
    }
}

fn dependency_audit() -> ExitCode {
    println!("{}", render_dependency_audit());
    ExitCode::SUCCESS
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DependencyAuditEntry {
    name: &'static str,
    category: &'static str,
    recommendation: &'static str,
    notes: &'static str,
}

fn dependency_audit_entries() -> &'static [DependencyAuditEntry] {
    &[
        DependencyAuditEntry {
            name: "bytemuck",
            category: "backend",
            recommendation: "keep",
            notes: "Needed for renderer vertex/uniform POD casts; gated by render backend.",
        },
        DependencyAuditEntry {
            name: "image",
            category: "convenience",
            recommendation: "keep optional",
            notes:
                "PNG is the only raster decode format; disabled by no-default builds and still covered by loader tests.",
        },
        DependencyAuditEntry {
            name: "js-sys",
            category: "backend",
            recommendation: "keep web-only",
            notes: "Used for browser byte buffers during web asset preload.",
        },
        DependencyAuditEntry {
            name: "kira",
            category: "backend",
            recommendation: "keep optional native-only",
            notes: "Provides desktop audio playback; target-gated away from wasm.",
        },
        DependencyAuditEntry {
            name: "raw-window-handle",
            category: "backend",
            recommendation: "keep",
            notes: "Required to pass native window/display handles into the renderer backend.",
        },
        DependencyAuditEntry {
            name: "serde",
            category: "serialization/config",
            recommendation: "keep",
            notes: "Core scene/resource/config serialization path.",
        },
        DependencyAuditEntry {
            name: "serde_json",
            category: "tooling",
            recommendation: "keep",
            notes: "Used for generated web manifests in xtask/runtime preload.",
        },
        DependencyAuditEntry {
            name: "toml",
            category: "serialization/config",
            recommendation: "keep",
            notes: "Current data-driven scene/resource format.",
        },
        DependencyAuditEntry {
            name: "tracing",
            category: "tooling",
            recommendation: "keep optional",
            notes: "Facade logging hook; behind the logging feature.",
        },
        DependencyAuditEntry {
            name: "tracing-subscriber",
            category: "tooling",
            recommendation: "keep optional",
            notes: "Default desktop logging setup; behind the logging feature.",
        },
        DependencyAuditEntry {
            name: "wasm-bindgen",
            category: "backend",
            recommendation: "keep web-only",
            notes: "Required for wasm startup and browser API bindings.",
        },
        DependencyAuditEntry {
            name: "wasm-bindgen-futures",
            category: "backend",
            recommendation: "keep web-only",
            notes: "Used for async browser fetch/preload bridging.",
        },
        DependencyAuditEntry {
            name: "web-sys",
            category: "backend",
            recommendation: "keep web-only",
            notes: "Browser window/canvas/audio/fetch APIs.",
        },
        DependencyAuditEntry {
            name: "wgpu",
            category: "backend",
            recommendation: "keep optional",
            notes: "Primary renderer backend; gated by render-wgpu.",
        },
        DependencyAuditEntry {
            name: "winit",
            category: "backend",
            recommendation: "keep optional",
            notes: "Desktop/web window and event loop integration; gated by runtime features.",
        },
    ]
}

fn render_dependency_audit() -> String {
    let mut output = String::new();
    output.push_str("# Dependency Audit\n\n");
    output.push_str("| dependency | category | recommendation | notes |\n");
    output.push_str("| --- | --- | --- | --- |\n");
    for entry in dependency_audit_entries() {
        output.push_str(&format!(
            "| `{}` | {} | {} | {} |\n",
            entry.name, entry.category, entry.recommendation, entry.notes
        ));
    }
    output.push_str("\nFeature checks:\n");
    output
        .push_str("- `seishin_no_default_wasm` keeps a no-default build in the workspace tests.\n");
    output.push_str(
        "- Desktop runtime initialization uses an internal no-dependency `block_on` helper.\n",
    );
    output.push_str("- `seishin_audio/kira-backend` is optional and native-target gated.\n");
    output.push_str(
        "- `seishin_audio/web` carries browser audio bindings separately from native audio.\n",
    );
    output.push_str("- CI builds the wasm example once on Linux while desktop validation runs on Linux, Windows, and macOS.\n");
    output.push_str("\nImage format policy:\n");
    output.push_str("- Seishin engine asset loading supports PNG raster textures through the optional `seishin_assets/png` feature.\n");
    output.push_str("- No-default builds do not decode raster images; they can still read raw asset bytes for custom pipelines.\n");
    output.push_str("- An internal PNG replacement is deferred until the project can justify owning PNG chunk, filter, and zlib/deflate maintenance.\n");
    output
}

#[cfg(test)]
fn dependency_names_from_manifest_source(source: &str) -> Vec<String> {
    let mut names = std::collections::BTreeSet::new();
    let mut in_dependencies = false;

    for line in source.lines() {
        let line = line.split_once('#').map_or(line, |(line, _)| line).trim();
        if line.is_empty() {
            continue;
        }

        if line.starts_with('[') && line.ends_with(']') {
            let section = line.trim_matches(['[', ']']);
            in_dependencies = section == "dependencies" || section.ends_with(".dependencies");
            continue;
        }

        if !in_dependencies {
            continue;
        }

        let Some((name, value)) = line.split_once('=') else {
            continue;
        };
        let name = name.trim().trim_matches('"');
        if name.starts_with("seishin") || value.contains("path") {
            continue;
        }
        names.insert(name.to_string());
    }

    names.into_iter().collect()
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
    let source = fs::read_to_string(&manifest_path).map_err(|error| error.to_string())?;

    package_name_from_manifest_source(&source)
        .ok_or_else(|| format!("package name not found in {}", manifest_path.display()))
}

fn package_name_from_manifest_source(source: &str) -> Option<String> {
    let mut in_package_section = false;

    for line in source.lines() {
        let line = line.split_once('#').map_or(line, |(line, _)| line).trim();
        if line.is_empty() {
            continue;
        }

        if line.starts_with('[') && line.ends_with(']') {
            in_package_section = line == "[package]";
            continue;
        }

        if !in_package_section {
            continue;
        }

        let Some((key, value)) = line.split_once('=') else {
            continue;
        };

        if key.trim() == "name" {
            return parse_quoted_string(value.trim());
        }
    }

    None
}

fn parse_quoted_string(value: &str) -> Option<String> {
    let value = value.strip_prefix('"')?;
    let end = value.find('"')?;

    Some(value[..end].to_string())
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

#[cfg(test)]
mod tests {
    use super::{
        dependency_audit_entries, dependency_names_from_manifest_source,
        package_name_from_manifest_source,
    };

    #[test]
    fn package_name_is_read_from_package_section_only() {
        let manifest = r#"
            [workspace]
            members = ["examples/basic_2d"]

            [dependencies]
            name = "not-a-package"

            [package]
            version = "0.1.0"
            name = "seishin_basic_2d"
        "#;

        assert_eq!(
            package_name_from_manifest_source(manifest),
            Some("seishin_basic_2d".to_string())
        );
    }

    #[test]
    fn dependency_audit_covers_direct_external_dependencies() {
        let root_manifest = include_str!("../../../Cargo.toml");
        let assets_manifest = include_str!("../../../crates/seishin_assets/Cargo.toml");
        let dependencies = dependency_names_from_manifest_source(root_manifest)
            .into_iter()
            .chain(dependency_names_from_manifest_source(assets_manifest))
            .collect::<std::collections::BTreeSet<_>>();
        let audited = dependency_audit_entries()
            .iter()
            .map(|entry| entry.name.to_string())
            .collect::<std::collections::BTreeSet<_>>();
        let missing = dependencies
            .difference(&audited)
            .cloned()
            .collect::<Vec<_>>();

        assert_eq!(missing, Vec::<String>::new());
    }

    #[test]
    fn dependency_audit_records_image_format_policy() {
        let audit = super::render_dependency_audit();

        assert!(audit.contains("Image format policy:"));
        assert!(audit.contains("PNG"));
    }

    #[test]
    fn registered_components_are_read_from_component_modules() {
        let main_source = r#"
            fn main() {
                App::discover_project()
                    .add_component(components::map_bootstrap::new())
                    .add_component(components::player_camera::new())
                    .run();
            }
        "#;
        let mut modules = std::collections::BTreeMap::new();
        modules.insert(
            "map_bootstrap",
            r#"pub fn new() -> impl ComponentDefinition {
                component_factory("MapBootstrap", map_bootstrap_factory)
            }"#,
        );
        modules.insert(
            "player_camera",
            r#"pub fn new() -> impl ComponentDefinition {
                |app: &mut StartupContext| {
                    app.register_component_factory("PlayerCamera", player_camera_factory)?;
                    Ok(())
                }
            }"#,
        );

        assert_eq!(
            super::registered_components_from_sources(main_source, |name| modules
                .get(name)
                .copied()),
            Ok(vec!["MapBootstrap".to_string(), "PlayerCamera".to_string()])
        );
    }

    #[test]
    fn custom_component_types_are_read_only_from_component_sections() {
        let source = r#"
            [input.actions.move]
            type = "axis2d"

            [components.player_controller]
            type = "PlayerController"

            [entities.components.camera]
            type = "PlayerCamera"
        "#;

        assert_eq!(
            super::custom_component_types_from_toml_source(source),
            vec!["PlayerCamera".to_string(), "PlayerController".to_string()]
        );
    }

    #[test]
    fn project_validation_reports_unregistered_scene_components() {
        let registered = vec!["PlayerController".to_string()];
        let used = vec![
            "PlayerCamera".to_string(),
            "PlayerController".to_string(),
            "WandererController".to_string(),
        ];

        assert_eq!(
            super::missing_components(&registered, &used),
            vec!["PlayerCamera".to_string(), "WandererController".to_string()]
        );
    }
}
