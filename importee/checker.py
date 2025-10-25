from __future__ import annotations

import json
import pathlib
from typing import Any, Dict, List

from .config import ImporteeConfig


class Issue:
    def __init__(self, path: pathlib.Path, message: str) -> None:
        self.path = path
        self.message = message

    def __str__(self) -> str:  # pragma: no cover - trivial
        return f"{self.path}: {self.message}"


def _coerce_str_list(val: Any) -> List[str]:
    if val is None:
        return []
    if isinstance(val, list):
        return [str(x) for x in val]
    if isinstance(val, str):
        # allow dot-separated fallback
        return [s for s in val.split(".") if s]
    return []


def _build_run_config(
    options: Dict[str, Any], verbose: bool, quiet: bool
) -> Dict[str, Any]:
    source_module = _coerce_str_list(options.get("source_module"))
    rules = options.get("rules") or {}
    linear = rules.get("linear") if isinstance(rules, dict) else None
    order = _coerce_str_list(linear.get("order")) if isinstance(linear, dict) else []

    run_cfg: Dict[str, Any] = {
        "source_module": source_module,
        "verbose": bool(verbose),
        "quiet": bool(quiet),
    }
    if order:
        run_cfg["rules"] = {"linear": {"order": order}}
    else:
        run_cfg["rules"] = {}
    return run_cfg


def run_check(
    config: ImporteeConfig, verbose: bool = False, quiet: bool = False
) -> List[Issue]:
    # Defer heavy lifting to Rust extension
    try:
        from . import _rust
    except Exception as exc:  # pragma: no cover
        raise RuntimeError("Rust extension not available") from exc

    # Minimal project config to satisfy Rust deserialization
    project_cfg = {
        "name": "importee",
        "version": "0",
        "description": "",
        "authors": [],
        "classifiers": [],
        "dependencies": [],
    }

    run_cfg = _build_run_config(config.options, verbose, quiet)

    result_json = _rust.check_imports(json.dumps(project_cfg), json.dumps(run_cfg))
    # The Rust currently prints diagnostics and returns an empty issues list
    try:
        payload = json.loads(result_json)
    except Exception:
        return []

    issues: List[Issue] = []
    for item in payload.get("issues", []):
        path = pathlib.Path(item.get("path", "."))
        msg = str(item.get("message", ""))
        issues.append(Issue(path, msg))
    return issues
