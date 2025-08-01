use ruff_python_ast::Expr;
use ruff_text_size::TextRange;

use ruff_macros::{ViolationMetadata, derive_message_formats};
use ruff_python_semantic::Modules;

use crate::Violation;
use crate::checkers::ast::Checker;

use crate::rules::flake8_datetimez::helpers;

/// ## What it does
/// Checks for usage of `datetime.datetime.today()`.
///
/// ## Why is this bad?
/// `datetime` objects are "naive" by default, in that they do not include
/// timezone information. "Naive" objects are easy to understand, but ignore
/// some aspects of reality, which can lead to subtle bugs. Timezone-aware
/// `datetime` objects are preferred, as they represent a specific moment in
/// time, unlike "naive" objects.
///
/// `datetime.datetime.today()` creates a "naive" object; instead, use
/// `datetime.datetime.now(tz=...)` to create a timezone-aware object.
///
/// ## Example
/// ```python
/// import datetime
///
/// datetime.datetime.today()
/// ```
///
/// Use instead:
/// ```python
/// import datetime
///
/// datetime.datetime.now(tz=datetime.timezone.utc)
/// ```
///
/// Or, for Python 3.11 and later:
/// ```python
/// import datetime
///
/// datetime.datetime.now(tz=datetime.UTC)
/// ```
#[derive(ViolationMetadata)]
pub(crate) struct CallDatetimeToday;

impl Violation for CallDatetimeToday {
    #[derive_message_formats]
    fn message(&self) -> String {
        "`datetime.datetime.today()` used".to_string()
    }

    fn fix_title(&self) -> Option<String> {
        Some("Use `datetime.datetime.now(tz=...)` instead".to_string())
    }
}

/// DTZ002
pub(crate) fn call_datetime_today(checker: &Checker, func: &Expr, location: TextRange) {
    if !checker.semantic().seen_module(Modules::DATETIME) {
        return;
    }

    if !checker
        .semantic()
        .resolve_qualified_name(func)
        .is_some_and(|qualified_name| {
            matches!(qualified_name.segments(), ["datetime", "datetime", "today"])
        })
    {
        return;
    }

    if helpers::followed_by_astimezone(checker) {
        return;
    }

    checker.report_diagnostic(CallDatetimeToday, location);
}
