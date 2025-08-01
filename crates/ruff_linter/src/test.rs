#![cfg(any(test, fuzzing))]
//! Helper functions for the tests of rule implementations.

use std::borrow::Cow;
use std::path::Path;

#[cfg(not(fuzzing))]
use anyhow::Result;
use itertools::Itertools;
use rustc_hash::FxHashMap;

use ruff_db::diagnostic::Diagnostic;
use ruff_notebook::Notebook;
#[cfg(not(fuzzing))]
use ruff_notebook::NotebookError;
use ruff_python_ast::PySourceType;
use ruff_python_codegen::Stylist;
use ruff_python_index::Indexer;
use ruff_python_parser::{ParseError, ParseOptions};
use ruff_python_trivia::textwrap::dedent;
use ruff_source_file::SourceFileBuilder;

use crate::codes::Rule;
use crate::fix::{FixResult, fix_file};
use crate::linter::check_path;
use crate::message::{Emitter, EmitterContext, TextEmitter, create_syntax_error_diagnostic};
use crate::package::PackageRoot;
use crate::packaging::detect_package_root;
use crate::settings::types::UnsafeFixes;
use crate::settings::{LinterSettings, flags};
use crate::source_kind::SourceKind;
use crate::{Applicability, FixAvailability};
use crate::{Locator, directives};

#[cfg(not(fuzzing))]
pub(crate) fn test_resource_path(path: impl AsRef<Path>) -> std::path::PathBuf {
    Path::new("./resources/test/").join(path)
}

/// Run [`check_path`] on a Python file in the `resources/test/fixtures` directory.
#[cfg(not(fuzzing))]
pub(crate) fn test_path(
    path: impl AsRef<Path>,
    settings: &LinterSettings,
) -> Result<Vec<Diagnostic>> {
    let path = test_resource_path("fixtures").join(path);
    let source_type = PySourceType::from(&path);
    let source_kind = SourceKind::from_path(path.as_ref(), source_type)?.expect("valid source");
    Ok(test_contents(&source_kind, &path, settings).0)
}

#[cfg(not(fuzzing))]
pub(crate) struct TestedNotebook {
    pub(crate) diagnostics: Vec<Diagnostic>,
    pub(crate) source_notebook: Notebook,
    pub(crate) linted_notebook: Notebook,
}

#[cfg(not(fuzzing))]
pub(crate) fn assert_notebook_path(
    path: impl AsRef<Path>,
    expected: impl AsRef<Path>,
    settings: &LinterSettings,
) -> Result<TestedNotebook, NotebookError> {
    let source_notebook = Notebook::from_path(path.as_ref())?;

    let source_kind = SourceKind::ipy_notebook(source_notebook);
    let (messages, transformed) = test_contents(&source_kind, path.as_ref(), settings);
    let expected_notebook = Notebook::from_path(expected.as_ref())?;
    let linted_notebook = transformed.into_owned().expect_ipy_notebook();

    assert_eq!(
        linted_notebook.cell_offsets(),
        expected_notebook.cell_offsets()
    );
    assert_eq!(linted_notebook.index(), expected_notebook.index());
    assert_eq!(
        linted_notebook.source_code(),
        expected_notebook.source_code()
    );

    Ok(TestedNotebook {
        diagnostics: messages,
        source_notebook: source_kind.expect_ipy_notebook(),
        linted_notebook,
    })
}

/// Run [`check_path`] on a snippet of Python code.
pub fn test_snippet(contents: &str, settings: &LinterSettings) -> Vec<Diagnostic> {
    let path = Path::new("<filename>");
    let contents = dedent(contents);
    test_contents(&SourceKind::Python(contents.into_owned()), path, settings).0
}

thread_local! {
    static MAX_ITERATIONS: std::cell::Cell<usize> = const { std::cell::Cell::new(10) };
}

pub fn set_max_iterations(max: usize) {
    MAX_ITERATIONS.with(|iterations| iterations.set(max));
}

pub(crate) fn max_iterations() -> usize {
    MAX_ITERATIONS.with(std::cell::Cell::get)
}

/// A convenient wrapper around [`check_path`], that additionally
/// asserts that fixes converge after a fixed number of iterations.
pub(crate) fn test_contents<'a>(
    source_kind: &'a SourceKind,
    path: &Path,
    settings: &LinterSettings,
) -> (Vec<Diagnostic>, Cow<'a, SourceKind>) {
    let source_type = PySourceType::from(path);
    let target_version = settings.resolve_target_version(path);
    let options =
        ParseOptions::from(source_type).with_target_version(target_version.parser_version());
    let parsed = ruff_python_parser::parse_unchecked(source_kind.source_code(), options.clone())
        .try_into_module()
        .expect("PySourceType always parses into a module");
    let locator = Locator::new(source_kind.source_code());
    let stylist = Stylist::from_tokens(parsed.tokens(), locator.contents());
    let indexer = Indexer::from_tokens(parsed.tokens(), locator.contents());
    let directives = directives::extract_directives(
        parsed.tokens(),
        directives::Flags::from_settings(settings),
        &locator,
        &indexer,
    );
    let messages = check_path(
        path,
        path.parent()
            .and_then(|parent| detect_package_root(parent, &settings.namespace_packages))
            .map(|path| PackageRoot::Root { path }),
        &locator,
        &stylist,
        &indexer,
        &directives,
        settings,
        flags::Noqa::Enabled,
        source_kind,
        source_type,
        &parsed,
        target_version,
    );

    let source_has_errors = parsed.has_invalid_syntax();

    // Detect fixes that don't converge after multiple iterations.
    let mut iterations = 0;

    let mut transformed = Cow::Borrowed(source_kind);

    if messages.iter().any(|message| message.fix().is_some()) {
        let mut messages = messages.clone();

        while let Some(FixResult {
            code: fixed_contents,
            source_map,
            ..
        }) = fix_file(
            &messages,
            &Locator::new(transformed.source_code()),
            UnsafeFixes::Enabled,
        ) {
            if iterations < max_iterations() {
                iterations += 1;
            } else {
                let output = print_diagnostics(messages, path, &transformed);

                panic!(
                    "Failed to converge after {} iterations. This likely \
                     indicates a bug in the implementation of the fix. Last diagnostics:\n{}",
                    max_iterations(),
                    output
                );
            }

            transformed = Cow::Owned(transformed.updated(fixed_contents, &source_map));

            let parsed =
                ruff_python_parser::parse_unchecked(transformed.source_code(), options.clone())
                    .try_into_module()
                    .expect("PySourceType always parses into a module");
            let locator = Locator::new(transformed.source_code());
            let stylist = Stylist::from_tokens(parsed.tokens(), locator.contents());
            let indexer = Indexer::from_tokens(parsed.tokens(), locator.contents());
            let directives = directives::extract_directives(
                parsed.tokens(),
                directives::Flags::from_settings(settings),
                &locator,
                &indexer,
            );

            let fixed_messages = check_path(
                path,
                None,
                &locator,
                &stylist,
                &indexer,
                &directives,
                settings,
                flags::Noqa::Enabled,
                &transformed,
                source_type,
                &parsed,
                target_version,
            );

            if parsed.has_invalid_syntax() && !source_has_errors {
                // Previous fix introduced a syntax error, abort
                let fixes = print_diagnostics(messages, path, source_kind);
                let syntax_errors = print_syntax_errors(parsed.errors(), path, &transformed);

                panic!(
                    "Fixed source has a syntax error where the source document does not. This is a bug in one of the generated fixes:
{syntax_errors}
Last generated fixes:
{fixes}
Source with applied fixes:
{}",
                    transformed.source_code()
                );
            }

            messages = fixed_messages;
        }
    }

    let source_code = SourceFileBuilder::new(
        path.file_name().unwrap().to_string_lossy().as_ref(),
        source_kind.source_code(),
    )
    .finish();

    let messages = messages
        .into_iter()
        .filter_map(|msg| Some((msg.secondary_code()?.to_string(), msg)))
        .map(|(code, mut diagnostic)| {
            let rule = Rule::from_code(&code).unwrap();
            let fixable = diagnostic.fix().is_some_and(|fix| {
                matches!(
                    fix.applicability(),
                    Applicability::Safe | Applicability::Unsafe
                )
            });

            match (fixable, rule.fixable()) {
                (true, FixAvailability::Sometimes | FixAvailability::Always)
                | (false, FixAvailability::None | FixAvailability::Sometimes) => {
                    // Ok
                }
                (true, FixAvailability::None) => {
                    panic!(
                        "Rule {rule:?} is marked as non-fixable but it created a fix.
Change the `Violation::FIX_AVAILABILITY` to either \
`FixAvailability::Sometimes` or `FixAvailability::Always`"
                    );
                }
                (false, FixAvailability::Always) if source_has_errors => {
                    // Ok
                }
                (false, FixAvailability::Always) => {
                    panic!(
                        "\
Rule {rule:?} is marked to always-fixable but the diagnostic has no fix.
Either ensure you always emit a fix or change `Violation::FIX_AVAILABILITY` to either \
`FixAvailability::Sometimes` or `FixAvailability::None`"
                    )
                }
            }

            assert!(
                !(fixable && diagnostic.first_help_text().is_none()),
                "Diagnostic emitted by {rule:?} is fixable but \
                `Violation::fix_title` returns `None`"
            );

            // Not strictly necessary but adds some coverage for this code path by overriding the
            // noqa offset and the source file
            let range = diagnostic.expect_range();
            diagnostic.set_noqa_offset(directives.noqa_line_for.resolve(range.start()));
            if let Some(annotation) = diagnostic.primary_annotation_mut() {
                annotation.set_span(
                    ruff_db::diagnostic::Span::from(source_code.clone()).with_range(range),
                );
            }

            diagnostic
        })
        .chain(parsed.errors().iter().map(|parse_error| {
            create_syntax_error_diagnostic(source_code.clone(), &parse_error.error, parse_error)
        }))
        .sorted_by(Diagnostic::ruff_start_ordering)
        .collect();
    (messages, transformed)
}

fn print_syntax_errors(errors: &[ParseError], path: &Path, source: &SourceKind) -> String {
    let filename = path.file_name().unwrap().to_string_lossy();
    let source_file = SourceFileBuilder::new(filename.as_ref(), source.source_code()).finish();

    let messages: Vec<_> = errors
        .iter()
        .map(|parse_error| {
            create_syntax_error_diagnostic(source_file.clone(), &parse_error.error, parse_error)
        })
        .collect();

    if let Some(notebook) = source.as_ipy_notebook() {
        print_jupyter_messages(&messages, path, notebook)
    } else {
        print_messages(&messages)
    }
}

/// Print the lint diagnostics in `diagnostics`.
fn print_diagnostics(mut diagnostics: Vec<Diagnostic>, path: &Path, source: &SourceKind) -> String {
    diagnostics.retain(|msg| !msg.is_invalid_syntax());

    if let Some(notebook) = source.as_ipy_notebook() {
        print_jupyter_messages(&diagnostics, path, notebook)
    } else {
        print_messages(&diagnostics)
    }
}

pub(crate) fn print_jupyter_messages(
    diagnostics: &[Diagnostic],
    path: &Path,
    notebook: &Notebook,
) -> String {
    let mut output = Vec::new();

    TextEmitter::default()
        .with_show_fix_status(true)
        .with_show_fix_diff(true)
        .with_show_source(true)
        .with_unsafe_fixes(UnsafeFixes::Enabled)
        .emit(
            &mut output,
            diagnostics,
            &EmitterContext::new(&FxHashMap::from_iter([(
                path.file_name().unwrap().to_string_lossy().to_string(),
                notebook.index().clone(),
            )])),
        )
        .unwrap();

    String::from_utf8(output).unwrap()
}

pub(crate) fn print_messages(diagnostics: &[Diagnostic]) -> String {
    let mut output = Vec::new();

    TextEmitter::default()
        .with_show_fix_status(true)
        .with_show_fix_diff(true)
        .with_show_source(true)
        .with_unsafe_fixes(UnsafeFixes::Enabled)
        .emit(
            &mut output,
            diagnostics,
            &EmitterContext::new(&FxHashMap::default()),
        )
        .unwrap();

    String::from_utf8(output).unwrap()
}

#[macro_export]
macro_rules! assert_diagnostics {
    ($value:expr, $path:expr, $notebook:expr) => {{
        insta::with_settings!({ omit_expression => true }, {
            insta::assert_snapshot!(
                $crate::test::print_jupyter_messages(&$value, &$path, &$notebook)
            );
        });
    }};
    ($value:expr, @$snapshot:literal) => {{
        insta::with_settings!({ omit_expression => true }, {
            insta::assert_snapshot!($crate::test::print_messages(&$value), @$snapshot);
        });
    }};
    ($name:expr, $value:expr) => {{
        insta::with_settings!({ omit_expression => true }, {
            insta::assert_snapshot!($name, $crate::test::print_messages(&$value));
        });
    }};
    ($value:expr) => {{
        insta::with_settings!({ omit_expression => true }, {
            insta::assert_snapshot!($crate::test::print_messages(&$value));
        });
    }};
}
