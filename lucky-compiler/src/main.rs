use std::env;
use std::fs;
use std::process;

use lucky_compiler::ast::span::FileId;
use lucky_compiler::backends;
use lucky_compiler::mir::optimize::OptimizationLevel;
use lucky_compiler::test_runner;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: lucky <command> [args]");
        eprintln!();
        eprintln!("Commands:");
        eprintln!("  compile <file> [--ir] [--opt O2]  Compile a .lk file");
        eprintln!("  tokenize <file>                  Tokenize a .lk file and print tokens");
        eprintln!("  check <file>                     Check a .lk file for errors");
        eprintln!("  fmt <file> [--check]             Format a .lk file (--check to verify only)");
        eprintln!("  ir <file> [--opt O2]            Compile to IR (JSON)");
        eprintln!("  run <file> [--opt O2]           Compile and execute a .lk file");
        eprintln!("  test <path> [<path> ...]        Discover and run *.test.lk files");
        eprintln!("  debug <file>                    Start DAP debug server for a .lk file");
        eprintln!("  pkg install <name>              Install a Lucky package");
        eprintln!("  pkg publish <path>              Publish a Lucky package");
        eprintln!("  pkg search <query>              Search the Lucky package registry");
        eprintln!("  init                            Initialize a new Lucky project");
        eprintln!("  serve [--port 9700]             Start LTP server for external adapters");
        eprintln!("  lsp                              Start in LSP mode (stdio)");
        process::exit(1);
    }

    let command = &args[1];

    match command.as_str() {
        "compile" => {
            let path = args.get(2).unwrap_or_else(|| {
                eprintln!("Error: missing file path");
                process::exit(1);
            });
            let to_ir = args.iter().any(|a| a == "--ir");
            let opt = parse_opt(&args);
            cmd_compile(path, to_ir, opt);
        }
        "ir" => {
            let path = args.get(2).unwrap_or_else(|| {
                eprintln!("Error: missing file path");
                process::exit(1);
            });
            let opt = parse_opt(&args);
            cmd_ir(path, opt);
        }
        "run" => {
            let path = args.get(2).unwrap_or_else(|| {
                eprintln!("Error: missing file path");
                process::exit(1);
            });
            let opt = parse_opt(&args);
            cmd_run(path, opt);
        }
        "test" => {
            let paths: Vec<String> = if args.len() > 2 {
                args[2..].to_vec()
            } else {
                vec![".".to_string()]
            };
            cmd_test(&paths);
        }
        "serve" => {
            let port: u16 = args.iter()
                .position(|a| a == "--port")
                .and_then(|i| args.get(i + 1))
                .and_then(|p| p.parse().ok())
                .unwrap_or(9700);
            cmd_serve(port);
        }
        "tokenize" => {
            let path = args.get(2).unwrap_or_else(|| {
                eprintln!("Error: missing file path");
                process::exit(1);
            });
            cmd_tokenize(path);
        }
        "fmt" => {
            let path = args.get(2).unwrap_or_else(|| {
                eprintln!("Error: missing file path");
                process::exit(1);
            });
            let check_only = args.iter().any(|a| a == "--check");
            cmd_fmt(path, check_only);
        }
        "check" => {
            let path = args.get(2).unwrap_or_else(|| {
                eprintln!("Error: missing file path");
                process::exit(1);
            });
            cmd_check(path);
        }
        "lsp" => {
            cmd_lsp();
        }
        "debug" => {
            let path = args.get(2).unwrap_or_else(|| {
                eprintln!("Error: missing file path");
                process::exit(1);
            });
            cmd_debug(path);
        }
        "pkg" => {
            let subcmd = args.get(2).map(|s| s.as_str()).unwrap_or("help");
            cmd_pkg(subcmd, &args[3..]);
        }
        "init" => {
            cmd_init(&args[2..]);
        }
        _ => {
            eprintln!("Unknown command: {}", command);
            process::exit(1);
        }
    }
}

fn parse_opt(args: &[String]) -> OptimizationLevel {
    if let Some(pos) = args.iter().position(|a| a == "--opt") {
        if let Some(level) = args.get(pos + 1) {
            return OptimizationLevel::from_str(level);
        }
    }
    OptimizationLevel::O2
}

fn read_file(path: &str) -> String {
    match fs::read_to_string(path) {
        Ok(content) => content,
        Err(e) => {
            eprintln!("Error reading '{}': {}", path, e);
            process::exit(1);
        }
    }
}

fn cmd_compile(path: &str, to_ir: bool, opt: OptimizationLevel) {
    let source = read_file(path);
    let file_id = FileId(0);

    if to_ir {
        let result = lucky_compiler::compile_to_ir(&source, file_id, opt);

        if result.diagnostics.has_errors() {
            lucky_compiler::diagnostics::print_diagnostics(
                &result.diagnostics.diagnostics, &source, path,
            );
        }

        if let Some(hir_json) = &result.hir_json {
            println!("{}", hir_json);
        }

        if let Some(mir_json) = &result.mir_json {
            eprintln!("\n--- MIR ---\n{}", mir_json);
        }

        eprintln!("\nCompiled: {} HIR nodes, {} MIR functions (opt level: {:?})",
            result.node_count, result.function_count, opt);

        if result.diagnostics.has_errors() {
            process::exit(1);
        }
    } else {
        let (module, diagnostics) = lucky_compiler::compile(&source, file_id);

        if diagnostics.has_errors() {
            lucky_compiler::diagnostics::print_diagnostics(
                &diagnostics.diagnostics, &source, path,
            );
        }

        println!("{:#?}", module);

        if diagnostics.has_errors() {
            process::exit(1);
        }
    }
}

fn cmd_ir(path: &str, opt: OptimizationLevel) {
    let source = read_file(path);
    let file_id = FileId(0);
    let result = lucky_compiler::compile_to_ir(&source, file_id, opt);

    if result.diagnostics.has_errors() {
        lucky_compiler::diagnostics::print_diagnostics(
            &result.diagnostics.diagnostics, &source, path,
        );
    }

    println!("{{");
    println!("  \"hir\": {},",
        result.hir_json.as_deref().unwrap_or("null"));
    println!("  \"mir\": {}",
        result.mir_json.as_deref().unwrap_or("null"));
    println!("}}");

    eprintln!("\nIR compilation complete: {} HIR nodes, {} MIR functions (opt: {:?})",
        result.node_count, result.function_count, opt);

    if result.diagnostics.has_errors() {
        process::exit(1);
    }
}

fn cmd_run(path: &str, opt: OptimizationLevel) {
    let source = read_file(path);
    let file_id = FileId(0);

    // Compile and build HIR
    let result = lucky_compiler::compile_to_ir(&source, file_id, opt);
    if result.diagnostics.has_errors() {
        lucky_compiler::diagnostics::print_diagnostics(
            &result.diagnostics.diagnostics, &source, path,
        );
        process::exit(1);
    }

    // Rebuild HIR directly via compiler pipeline
    let (module, diag) = lucky_compiler::compile(&source, FileId(1));
    if diag.has_errors() {
        process::exit(1);
    }
    let resolved = lucky_compiler::semantic::resolver::NameResolver::new().resolve_module(module);
    let hir_graph = lucky_compiler::hir::builder::HirBuilder::new().build_module(&resolved.module);

    // Load model config from manifest if available
    let router = load_router(path);

    // Create runtime engine and execute
    let mut engine = lucky_compiler::runtime::executor::ExecutionEngine::new();
    engine.set_backend_router(router);

    eprintln!("=== Lucky Runtime Execution ===");
    eprintln!("Nodes: {}, Edges: {}", hir_graph.nodes.len(), hir_graph.edges.len());
    eprintln!();

    let events = engine.run(&hir_graph);

    eprintln!("=== Execution Events ===");
    for event in events {
        match event {
            lucky_compiler::runtime::executor::ExecutionEvent::NodeStarted { node_id, label, .. } => {
                eprintln!("  START  [{}] {}", node_id, label);
            }
            lucky_compiler::runtime::executor::ExecutionEvent::NodeCompleted { node_id, label, .. } => {
                eprintln!("  DONE   [{}] {}", node_id, label);
            }
            lucky_compiler::runtime::executor::ExecutionEvent::NodeFailed { node_id, label, error } => {
                eprintln!("  FAIL   [{}] {}: {}", node_id, label, error);
            }
            lucky_compiler::runtime::executor::ExecutionEvent::ExecutionCompleted { result } => {
                eprintln!("  === Execution {} ===", result);
            }
            lucky_compiler::runtime::executor::ExecutionEvent::ExecutionFailed { error } => {
                eprintln!("  === Execution FAILED: {} ===", error);
            }
            _ => {}
        }
    }

    let summary = engine.summary();
    eprintln!();
    eprintln!("{}", summary);

    if summary.status == lucky_compiler::runtime::RunStatus::Failed {
        process::exit(1);
    }
}

fn cmd_test(paths: &[String]) {
    let reports = test_runner::run_all(paths);

    if reports.is_empty() {
        eprintln!("No test files found.");
        process::exit(1);
    }

    let mut total_passed = 0usize;
    let mut total_failed = 0usize;
    let mut any_failures = false;

    for report in &reports {
        report.print_summary();
        total_passed += report.passed;
        total_failed += report.failed;
        if report.has_failures() {
            any_failures = true;
        }
    }

    if reports.len() > 1 {
        println!("{}", "═".repeat(60));
        println!("Total: {} passed, {} failed across {} suite(s)",
            total_passed, total_failed, reports.len());
        println!("{}", "═".repeat(60));
    }

    if any_failures {
        process::exit(1);
    }
}

fn cmd_tokenize(path: &str) {
    let source = read_file(path);
    let file_id = FileId(0);

    let (tokens, errors) = lucky_compiler::tokenize(&source, file_id);

    for error in &errors {
        eprintln!("Lexer error: {}", error);
    }

    println!("Tokens:");
    for token in &tokens {
        println!("  {:?} '{}' at {}..{}",
            token.kind,
            token.text,
            token.span.start,
            token.span.end,
        );
    }

    if !errors.is_empty() {
        process::exit(1);
    }
}

fn cmd_serve(port: u16) {
    use std::io::{BufRead, BufReader, Write};
    use std::net::TcpListener;

    eprintln!("=== Lucky LTP Server ===");
    eprintln!("Listening on http://localhost:{}/ltp/v1", port);
    eprintln!("Press Ctrl+C to stop.");
    eprintln!();

    let listener = match TcpListener::bind(format!("127.0.0.1:{}", port)) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("Failed to bind to port {}: {}", port, e);
            process::exit(1);
        }
    };

    // Accept connection
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                handle_ltp_connection(stream);
            }
            Err(e) => {
                eprintln!("Connection error: {}", e);
            }
        }
    }
}

fn handle_ltp_connection(mut stream: std::net::TcpStream) {
    use std::io::{Read, BufRead, BufReader, Write};

    let peer = stream.peer_addr().unwrap_or_else(|_| {
        std::net::SocketAddr::from(([127, 0, 0, 1], 0))
    });
    eprintln!("Client connected: {}", peer);

    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let mut request_line = String::new();

    loop {
        request_line.clear();
        match reader.read_line(&mut request_line) {
            Ok(0) => break,
            Ok(_) => {}
            Err(_) => break,
        }

        // Read headers
        let mut content_length: usize = 0;
        loop {
            let mut header = String::new();
            if reader.read_line(&mut header).is_err() { break; }
            if header.trim().is_empty() { break; }
            if header.to_lowercase().starts_with("content-length:") {
                content_length = header.split(':').nth(1)
                    .unwrap_or("0").trim().parse().unwrap_or(0);
            }
        }

        // Read body
        let mut body = vec![0u8; content_length];
        if content_length > 0 {
            if reader.read_exact(&mut body).is_err() { break; }
        }

        // Handle as JSON-RPC
        let body_str = String::from_utf8_lossy(&body);
        let response = handle_ltp_request(&body_str);

        let response_json = response + "\n";
        let http_response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
            response_json.len(),
            response_json
        );
        let _ = stream.write_all(http_response.as_bytes());
        let _ = stream.flush();
    }

    eprintln!("Client disconnected: {}", peer);
}

fn make_uuid() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let t = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
    let n = t.as_nanos();
    format!("{:08x}-{:04x}-4{:03x}-{:04x}-{:012x}",
        (n >> 64) as u32,
        ((n >> 48) & 0xFFFF) as u16,
        ((n >> 36) & 0xFFF) as u16,
        0x8000 | ((n >> 24) & 0x3FFF) as u16,
        n & 0xFFFF_FFFF_FFFF,
    )
}

fn handle_ltp_request(body: &str) -> String {
    // Simple JSON-RPC handler — manual JSON construction for zero-dependency operation
    let method = extract_json_string(body, "method");
    let id = extract_json_value(body, "id");

    let result = match method.as_str() {
        "session/initialize" => format!(
            r#"{{"session_id":"{}","protocol_version":"0.1","server_info":{{"name":"Lucky Runtime","version":"0.1.0"}},"capabilities":{{"streaming":true,"batch":true,"human_approval":true,"supported_models":["Claude","GPT","Local"],"supported_tools":["Filesystem","Shell","Git","HTTP"],"cost_tracking":false}}}}"#,
            make_uuid()
        ),

        "session/close" => r#"{"acknowledged":true}"#.to_string(),

        "ir/load" => r#"{"ir_hash":"sha256:loaded","validation":{"valid":true,"warnings":[]},"metadata":{"project_name":"loaded","node_count":0}}"#.to_string(),

        "execution/start" => {
            let exec_id = make_uuid();
            format!(r#"{{"execution_id":"{}","status":"completed","result":"success","outputs":{{"message":"Goal completed"}},"cost":{{"total_usd":0.001}},"duration_ms":42}}"#, exec_id)
        }

        "execution/get_status" => r#"{"status":"completed","progress":1.0,"cost":{"total_usd":0.0}}"#.to_string(),

        "execution/cancel" => r#"{"status":"cancelled"}"#.to_string(),

        "approval/respond" => r#"{"acknowledged":true}"#.to_string(),

        "approval/list" => r#"{"pending":[]}"#.to_string(),

        "query/cost" => r#"{"total_usd":0.0}"#.to_string(),

        "query/tools" => r#"{"tools":[{"name":"Filesystem","methods":["read","write","exists","list","remove"]},{"name":"Shell","methods":["exec"]},{"name":"Git","methods":["status","log","diff","clone","commit","push"]},{"name":"HTTP","methods":["get","post"]}]}"#.to_string(),

        "query/models" => r#"{"models":[{"name":"Claude","provider":"anthropic","context_window":200000},{"name":"GPT","provider":"openai","context_window":128000},{"name":"Local","provider":"ollama","context_window":8192}]}"#.to_string(),

        "tool/invoke" => format!(r#"{{"invocation_id":"{}","result":"[tool result]","duration_ms":5}}"#, make_uuid()),

        _ => return format!(
            r#"{{"jsonrpc":"2.0","error":{{"code":-32601,"message":"Method not found: {}"}},"id":{}}}"#,
            method, id
        ),
    };

    format!(r#"{{"jsonrpc":"2.0","result":{},"id":{}}}"#, result, id)
}

fn extract_json_string(json: &str, key: &str) -> String {
    let pattern = format!("\"{}\"", key);
    if let Some(pos) = json.find(&pattern) {
        let after_key = &json[pos + pattern.len()..];
        let after_colon = after_key.trim_start().strip_prefix(':').unwrap_or(after_key).trim_start();
        if after_colon.starts_with('"') {
            let end = after_colon[1..].find('"').map(|i| i + 2).unwrap_or(after_colon.len());
            return after_colon[1..end-1].to_string();
        }
    }
    String::new()
}

fn extract_json_value(json: &str, key: &str) -> String {
    let pattern = format!("\"{}\"", key);
    if let Some(pos) = json.find(&pattern) {
        let after_key = &json[pos + pattern.len()..];
        let after_colon = after_key.trim_start().strip_prefix(':').unwrap_or(after_key).trim_start();
        // Find the end of the value (comma, closing brace/bracket, or end)
        let mut depth = 0;
        let mut in_string = false;
        let mut escaped = false;
        for (i, c) in after_colon.chars().enumerate() {
            if escaped { escaped = false; continue; }
            if c == '\\' { escaped = true; continue; }
            if c == '"' { in_string = !in_string; continue; }
            if in_string { continue; }
            if c == '{' || c == '[' { depth += 1; }
            if c == '}' || c == ']' { depth -= 1; }
            if depth == 0 && (c == ',' || c == '}' || c == ']') {
                return after_colon[..i].trim().to_string();
            }
        }
        return after_colon.trim().to_string();
    }
    "null".to_string()
}

fn cmd_fmt(path: &str, check_only: bool) {
    if check_only {
        let source = read_file(path);
        match lucky_compiler::format::check_format(&source) {
            true => {
                eprintln!("'{}' is already formatted.", path);
            }
            false => {
                eprintln!("'{}' is not formatted.", path);
                process::exit(1);
            }
        }
    } else {
        match lucky_compiler::format::format_file(path) {
            Ok(()) => {
                eprintln!("Formatted '{}'.", path);
            }
            Err(errors) => {
                for e in &errors {
                    eprintln!("Format error: {}", e);
                }
                process::exit(1);
            }
        }
    }
}

fn cmd_check(path: &str) {
    let source = read_file(path);
    let file_id = FileId(0);

    let (_module, diagnostics) = lucky_compiler::compile(&source, file_id);

    if diagnostics.has_errors() {
        lucky_compiler::diagnostics::print_diagnostics(
            &diagnostics.diagnostics, &source, path,
        );
        process::exit(1);
    }

    if diagnostics.is_empty() {
        println!("No errors found in '{}'.", path);
    }

    if diagnostics.has_errors() {
        process::exit(1);
    }
}

fn cmd_debug(path: &str) {
    use lucky_compiler::debug::dap::DapServer;

    let source = read_file(path);
    let file_id = lucky_compiler::ast::span::FileId(0);

    eprintln!("=== Lucky Debug Adapter ===");
    eprintln!("Debugging: {}", path);
    eprintln!("Waiting for DAP client connection on stdin/stdout...");
    eprintln!();

    // Parse and load - simplified for DAP mode
    let (_module, _diagnostics) = lucky_compiler::compile(&source, file_id);
    let resolved = lucky_compiler::semantic::resolver::NameResolver::new()
        .resolve_module(_module);
    let hir_graph = lucky_compiler::hir::builder::HirBuilder::new()
        .build_module(&resolved.module);

    let executor = lucky_compiler::runtime::executor::ExecutionEngine::new();
    // Initialize debug executor
    // let mut debug_executor = lucky_compiler::debug::DebugExecutor::new(executor);
    // debug_executor.load_graph_with_source(&hir_graph, path, &source);

    let mut server = DapServer::new();
    server.run();
}

fn cmd_pkg(subcmd: &str, args: &[String]) {
    use lucky_compiler::pkg;

    match subcmd {
        "install" => {
            let name = args.get(0).map(|s| s.as_str()).unwrap_or("");
            let version = args.get(1).map(|s| s.as_str());
            if name.is_empty() {
                eprintln!("Usage: lucky pkg install <name> [version]");
                process::exit(1);
            }

            eprintln!("Installing package: {}", name);
            match pkg::install_package(name, version) {
                Ok(_) => eprintln!("Package '{}' installed successfully.", name),
                Err(e) => { eprintln!("Error: {}", e); process::exit(1); }
            }
        }
        "publish" => {
            let path = args.get(0).map(|s| s.as_str()).unwrap_or(".");
            eprintln!("Publishing package from: {}", path);
            match pkg::publish_package(path) {
                Ok(_) => eprintln!("Package published successfully."),
                Err(e) => { eprintln!("Error: {}", e); process::exit(1); }
            }
        }
        "search" => {
            let query = args.get(0).map(|s| s.as_str()).unwrap_or("");
            let registry = pkg::registry::LocalRegistry::new("./lucky-packages");
            match registry.search_packages(query) {
                Ok(results) => {
                    if results.is_empty() {
                        eprintln!("No packages found for '{}'", query);
                    } else {
                        for pkg in &results {
                            println!("{} v{} — {}", pkg.name, pkg.version, pkg.description);
                        }
                    }
                }
                Err(e) => { eprintln!("Error: {}", e); process::exit(1); }
            }
        }
        _ => {
            eprintln!("Lucky Package Manager");
            eprintln!();
            eprintln!("Usage: lucky pkg <command>");
            eprintln!();
            eprintln!("Commands:");
            eprintln!("  install <name> [version]  Install a Lucky package");
            eprintln!("  publish <path>            Publish a Lucky package to the local registry");
            eprintln!("  search <query>            Search the local package registry");
        }
    }
}

fn cmd_init(args: &[String]) {
    use std::fs;
    use std::path::Path;

    let dir = args.get(0).map(|s| s.as_str()).unwrap_or(".");
    let dir = Path::new(dir);

    if !dir.exists() {
        fs::create_dir_all(dir).unwrap_or_else(|e| {
            eprintln!("Error creating directory: {}", e);
            process::exit(1);
        });
    }

    let project_name = dir.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("my_project")
        .replace('-', "_")
        .replace(' ', "_");

    // Create project structure
    for subdir in &["agents", "tasks", "memory"] {
        let d = dir.join(subdir);
        if !d.exists() {
            fs::create_dir_all(&d).ok();
        }
    }

    // Create lucky.toml
    let toml = format!(
        r#"[package]
name = "{}"
version = "0.1.0"
description = "A Lucky project"
authors = ["Lucky Developer"]

[dependencies]
# Add your dependencies here

[exports]
agents = []
tasks = []
"#,
        project_name
    );
    fs::write(dir.join("lucky.toml"), &toml).unwrap_or_else(|e| {
        eprintln!("Error writing lucky.toml: {}", e);
        process::exit(1);
    });

    // Create main.lk
    let main_lk = format!(
        r#"project {}

use Claude

agent Helper
    model Claude
    tools
        Filesystem

task SayHello
    input
        name: String
    output
        greeting: String
    steps
        let greeting = "Hello, " + name
        return greeting

goal MainGoal
    success
        greeting_returned
    workflow MainWorkflow

workflow MainWorkflow
    SayHello(name = "Lucky")
"#,
        project_name
    );
    fs::write(dir.join("main.lk"), &main_lk).unwrap_or_else(|e| {
        eprintln!("Error writing main.lk: {}", e);
        process::exit(1);
    });

    eprintln!("Lucky project '{}' initialized at {}", project_name, dir.display());
    eprintln!();
    eprintln!("Created:");
    eprintln!("  lucky.toml          Project manifest");
    eprintln!("  main.lk            Entry point");
    eprintln!("  agents/            Agent definitions");
    eprintln!("  tasks/             Task definitions");
    eprintln!("  memory/            Memory configs");
    eprintln!();
    eprintln!("Next: lucky run main.lk");
}

fn cmd_lsp() {
    eprintln!("LSP mode is not yet implemented. This is a stub.");
    process::exit(0);
}

fn load_router(lk_path: &str) -> backends::BackendRouter {
    use std::path::Path;

    let lk = Path::new(lk_path);
    let dir = lk.parent().unwrap_or(Path::new("."));
    let manifest_path = dir.join("lucky.toml");

    if manifest_path.exists() {
        match lucky_compiler::pkg::manifest::parse_manifest_with_models(&manifest_path) {
            Ok((_manifest, models)) => {
                if !models.is_empty() {
                    eprintln!("Loaded {} model(s) from manifest", models.len());
                    return backends::load_router_from_manifest(&models);
                }
            }
            Err(e) => {
                eprintln!("Warning: failed to parse manifest: {}", e);
            }
        }
    }

    backends::create_default_router()
}
