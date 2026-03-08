#![allow(missing_docs)]

use foundry_compilers_edge::{
    EdgeCompiler, EdgeCompilerInput, EdgeLanguage, EdgeParsedSource, EdgeSettings,
};

use foundry_compilers::artifacts::sources::{Source, Sources};
use foundry_compilers::compilers::{Compiler, CompilerInput};
use foundry_compilers::{CompilationError, CompilerSettings, Language, ParsedSource};

use std::path::Path;

#[test]
fn test_edge_language_extensions() {
    assert!(EdgeLanguage::FILE_EXTENSIONS.contains(&"edge"));
    assert_eq!(format!("{EdgeLanguage}"), "Edge");
}

#[test]
fn test_parsed_source_contract_names() {
    let source = r#"
contract Counter {
    let count: &s u256;
    pub fn get() -> (u256) { return count; }
}
"#;
    let parsed = EdgeParsedSource::parse(source, Path::new("counter.edge")).unwrap();
    assert_eq!(parsed.contract_names(), &["Counter"]);
    assert!(parsed.version_req().is_none());
    assert_eq!(parsed.language(), EdgeLanguage);
}

#[test]
fn test_parsed_source_version_pragma() {
    let source = "// @version ^0.1.0\ncontract Foo {}";
    let parsed = EdgeParsedSource::parse(source, Path::new("foo.edge")).unwrap();
    assert!(parsed.version_req().is_some());
    assert_eq!(parsed.contract_names(), &["Foo"]);
}

#[test]
fn test_parsed_source_multiple_contracts() {
    let source = "contract A {}\ncontract B {}";
    let parsed = EdgeParsedSource::parse(source, Path::new("multi.edge")).unwrap();
    assert_eq!(parsed.contract_names(), &["A", "B"]);
}

#[test]
fn test_parsed_source_no_contracts() {
    let source = "// just a comment";
    let parsed = EdgeParsedSource::parse(source, Path::new("empty.edge")).unwrap();
    assert!(parsed.contract_names().is_empty());
    assert!(parsed.version_req().is_none());
}

#[test]
fn test_compile_counter() {
    let counter_src = include_str!("../../../examples/counter.edge");

    let mut sources = Sources::new();
    sources.insert(
        std::path::PathBuf::from("counter.edge"),
        Source::new(counter_src),
    );

    let settings = EdgeSettings::default();
    let input = EdgeCompilerInput::build(
        sources,
        settings,
        EdgeLanguage,
        semver::Version::new(0, 1, 0),
    );

    let compiler = EdgeCompiler::new(semver::Version::new(0, 1, 0));
    let output = compiler.compile(&input).expect("compile should succeed");

    // Should have no fatal errors
    let errors: Vec<_> = output.errors.iter().filter(|e| e.is_error()).collect();
    assert!(errors.is_empty(), "Expected no errors, got: {errors:?}");

    // Should have at least one contract
    assert!(
        !output.contracts.is_empty(),
        "Expected at least one contract in output"
    );
}

#[test]
fn test_available_versions() {
    let compiler = EdgeCompiler::new(semver::Version::new(0, 1, 18));
    let versions = compiler.available_versions(&EdgeLanguage);
    assert!(!versions.is_empty());
    assert!(versions.iter().any(|v| v.is_installed()));
}

#[test]
fn test_settings_can_use_cached() {
    let s1 = EdgeSettings::default();
    let s2 = EdgeSettings::default();
    assert!(s1.can_use_cached(&s2));
}

#[test]
fn test_compiler_input_build() {
    let mut sources = Sources::new();
    sources.insert(
        std::path::PathBuf::from("test.edge"),
        Source::new("contract Test {}"),
    );

    let settings = EdgeSettings::default();
    let version = semver::Version::new(0, 1, 0);
    let input = EdgeCompilerInput::build(sources, settings, EdgeLanguage, version.clone());

    assert_eq!(input.version(), &version);
    assert_eq!(input.language(), EdgeLanguage);
    assert_eq!(input.sources().count(), 1);
}

#[test]
fn test_compiler_input_strip_prefix() {
    let mut sources = Sources::new();
    sources.insert(
        std::path::PathBuf::from("/home/user/project/src/test.edge"),
        Source::new("contract Test {}"),
    );

    let settings = EdgeSettings::default();
    let version = semver::Version::new(0, 1, 0);
    let mut input = EdgeCompilerInput::build(sources, settings, EdgeLanguage, version);

    input.strip_prefix(Path::new("/home/user/project"));

    let paths: Vec<_> = input.sources().map(|(p, _)| p.to_path_buf()).collect();
    assert_eq!(paths, vec![std::path::PathBuf::from("src/test.edge")]);
}
