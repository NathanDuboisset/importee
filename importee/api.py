"""High-level Python API wrapping the Rust extension."""

try:
    from . import _rust
except Exception as exc:  # pragma: no cover - import error surfaces at runtime
    raise RuntimeError(
        "Failed to import Rust extension module 'importee._rust'"
    ) from exc
