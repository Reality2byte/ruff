import io
from typing import Any, Protocol

class _IOBytes(Protocol):
    def read(self, size: int, /) -> bytes: ...
    def seek(self, size: int, whence: int = ..., /) -> Any: ...

def load_tzdata(key: str) -> io.BufferedReader: ...
def load_data(
    fobj: _IOBytes,
) -> tuple[tuple[int, ...], tuple[int, ...], tuple[int, ...], tuple[int, ...], tuple[str, ...], bytes | None]: ...

class ZoneInfoNotFoundError(KeyError):
    """Exception raised when a ZoneInfo key is not found."""
