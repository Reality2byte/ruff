use std::path::Path;

use ruff_macros::{ViolationMetadata, derive_message_formats};
use ruff_text_size::TextRange;

use crate::Violation;
use crate::checkers::ast::LintContext;
#[cfg(target_family = "unix")]
use crate::rules::flake8_executable::helpers::is_executable;

/// ## What it does
/// Checks for a shebang directive in a file that is not executable.
///
/// ## Why is this bad?
/// In Python, a shebang (also known as a hashbang) is the first line of a
/// script, which specifies the interpreter that should be used to run the
/// script.
///
/// The presence of a shebang suggests that a file is intended to be
/// executable. If a file contains a shebang but is not executable, then the
/// shebang is misleading, or the file is missing the executable bit.
///
/// If the file is meant to be executable, add the executable bit to the file
/// (e.g., `chmod +x __main__.py` or `git update-index --chmod=+x __main__.py`).
///
/// Otherwise, remove the shebang.
///
/// A file is considered executable if it has the executable bit set (i.e., its
/// permissions mode intersects with `0o111`). As such, _this rule is only
/// available on Unix-like systems_, and is not enforced on Windows or WSL.
///
/// ## Example
/// ```python
/// #!/usr/bin/env python
/// ```
///
/// ## References
/// - [Python documentation: Executable Python Scripts](https://docs.python.org/3/tutorial/appendix.html#executable-python-scripts)
/// - [Git documentation: `git update-index --chmod`](https://git-scm.com/docs/git-update-index#Documentation/git-update-index.txt---chmod-x)
#[derive(ViolationMetadata)]
pub(crate) struct ShebangNotExecutable;

impl Violation for ShebangNotExecutable {
    #[derive_message_formats]
    fn message(&self) -> String {
        "Shebang is present but file is not executable".to_string()
    }
}

/// EXE001
#[cfg(target_family = "unix")]
pub(crate) fn shebang_not_executable(filepath: &Path, range: TextRange, context: &LintContext) {
    // WSL supports Windows file systems, which do not have executable bits.
    // Instead, everything is executable. Therefore, we skip this rule on WSL.

    if is_wsl::is_wsl() {
        return;
    }

    if let Ok(false) = is_executable(filepath) {
        context.report_diagnostic_if_enabled(ShebangNotExecutable, range);
    }
}

#[cfg(not(target_family = "unix"))]
pub(crate) fn shebang_not_executable(
    _filepath: &Path,
    _range: TextRange,
    _diagnostics: &LintContext,
) {
}
