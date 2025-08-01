"""TestSuite"""

import unittest.case
import unittest.result
from collections.abc import Iterable, Iterator
from typing import ClassVar
from typing_extensions import TypeAlias

_TestType: TypeAlias = unittest.case.TestCase | TestSuite

class BaseTestSuite:
    """A simple test suite that doesn't provide class or module shared fixtures."""

    _tests: list[unittest.case.TestCase]
    _removed_tests: int
    def __init__(self, tests: Iterable[_TestType] = ()) -> None: ...
    def __call__(self, result: unittest.result.TestResult) -> unittest.result.TestResult: ...
    def addTest(self, test: _TestType) -> None: ...
    def addTests(self, tests: Iterable[_TestType]) -> None: ...
    def run(self, result: unittest.result.TestResult) -> unittest.result.TestResult: ...
    def debug(self) -> None:
        """Run the tests without collecting errors in a TestResult"""

    def countTestCases(self) -> int: ...
    def __iter__(self) -> Iterator[_TestType]: ...
    def __eq__(self, other: object) -> bool: ...
    __hash__: ClassVar[None]  # type: ignore[assignment]

class TestSuite(BaseTestSuite):
    """A test suite is a composite test consisting of a number of TestCases.

    For use, create an instance of TestSuite, then add test case instances.
    When all tests have been added, the suite can be passed to a test
    runner, such as TextTestRunner. It will run the individual test cases
    in the order in which they were added, aggregating the results. When
    subclassing, do not forget to call the base class constructor.
    """

    def run(self, result: unittest.result.TestResult, debug: bool = False) -> unittest.result.TestResult: ...
