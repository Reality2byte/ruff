use ruff_macros::{ViolationMetadata, derive_message_formats};
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_text_size::Ranged;

use crate::Violation;
use crate::checkers::ast::Checker;
use crate::rules::flake8_async::helpers::AsyncModule;

/// ## What it does
/// Checks for the use of an async sleep function in a `while` loop.
///
/// ## Why is this bad?
/// Instead of sleeping in a `while` loop, and waiting for a condition
/// to become true, it's preferable to `await` on an `Event` object such
/// as: `asyncio.Event`, `trio.Event`, or `anyio.Event`.
///
/// ## Example
/// ```python
/// import asyncio
///
/// DONE = False
///
///
/// async def func():
///     while not DONE:
///         await asyncio.sleep(1)
/// ```
///
/// Use instead:
/// ```python
/// import asyncio
///
/// DONE = asyncio.Event()
///
///
/// async def func():
///     await DONE.wait()
/// ```
///
/// ## References
/// - [`asyncio` events](https://docs.python.org/3/library/asyncio-sync.html#asyncio.Event)
/// - [`anyio` events](https://anyio.readthedocs.io/en/latest/api.html#anyio.Event)
/// - [`trio` events](https://trio.readthedocs.io/en/latest/reference-core.html#trio.Event)
#[derive(ViolationMetadata)]
pub(crate) struct AsyncBusyWait {
    module: AsyncModule,
}

impl Violation for AsyncBusyWait {
    #[derive_message_formats]
    fn message(&self) -> String {
        let Self { module } = self;
        format!("Use `{module}.Event` instead of awaiting `{module}.sleep` in a `while` loop")
    }
}

/// ASYNC110
pub(crate) fn async_busy_wait(checker: &Checker, while_stmt: &ast::StmtWhile) {
    // The body should be a single `await` call.
    let [stmt] = while_stmt.body.as_slice() else {
        return;
    };
    let Stmt::Expr(ast::StmtExpr { value, .. }) = stmt else {
        return;
    };
    let Expr::Await(ast::ExprAwait { value, .. }) = value.as_ref() else {
        return;
    };
    let Expr::Call(ast::ExprCall { func, .. }) = value.as_ref() else {
        return;
    };

    let Some(qualified_name) = checker.semantic().resolve_qualified_name(func.as_ref()) else {
        return;
    };

    if matches!(
        qualified_name.segments(),
        ["trio" | "anyio", "sleep" | "sleep_until"] | ["asyncio", "sleep"]
    ) {
        checker.report_diagnostic(
            AsyncBusyWait {
                module: AsyncModule::try_from(&qualified_name).unwrap(),
            },
            while_stmt.range(),
        );
    }
}
