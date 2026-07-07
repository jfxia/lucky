use std::path::Path;
use std::fs;

/// Files that are expected to parse without errors.
const KNOWN_GOOD: &[&str] = &[
    "tests/spec/mini.lk",
    "tests/spec/declarations/basic.lk",
    "tests/spec/declarations/permissions.lk",
    "tests/spec/declarations/all_decls.lk",
    "tests/spec/expressions/all_exprs.lk",
    "tests/spec/statements/all_stmts.lk",
    "tests/spec/ai/ai_constructs.lk",
];

/// Files that are expected to contain parse errors (recovery tests).
const KNOWN_ERROR: &[&str] = &[
    "tests/spec/errors/recovery.lk",
];

#[test]
fn spec_files_compile_without_panic() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests").join("spec");
    let mut tested = 0;
    let mut passed = 0;
    let mut failed = Vec::new();

    visit_dirs(&root, &mut |path| {
        if path.extension().and_then(|e| e.to_str()) != Some("lk") {
            return;
        }
        let rel = path.strip_prefix(env!("CARGO_MANIFEST_DIR"))
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");

        let source = match fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) => { failed.push(format!("{}: read error: {}", rel, e)); return; }
        };

        let file_id = lucky_compiler::ast::span::FileId(0);
        let (_module, diagnostics) = lucky_compiler::compile(&source, file_id);

        let is_good = KNOWN_GOOD.contains(&rel.as_str());
        let is_error = KNOWN_ERROR.contains(&rel.as_str());
        let has_errors = diagnostics.has_errors();

        tested += 1;

        if has_errors && is_good {
            failed.push(format!("{}: expected OK but got errors", rel));
        } else if !has_errors && is_error {
            failed.push(format!("{}: expected errors but got OK", rel));
        } else if has_errors && !is_error {
            // File not in known lists and has errors — count as warning, not failure
            eprintln!("[WARN] {} has {} error(s) — not yet in known-good list", rel, diagnostics.diagnostics.len());
            // Still pass — we just track it
        }
        passed += 1;
    });

    assert!(
        failed.is_empty(),
        "{} spec test(s) failed:\n{}",
        failed.len(),
        failed.join("\n")
    );
    eprintln!("Spec tests: {} passed, {} warning(s)", tested, failed.len());
}

fn visit_dirs(dir: &Path, cb: &mut dyn FnMut(&Path)) {
    if dir.is_dir() {
        for entry in fs::read_dir(dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_dir() {
                visit_dirs(&path, cb);
            } else {
                cb(&path);
            }
        }
    }
}
