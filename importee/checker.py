from __future__ import annotations

import json
import pathlib
from dataclasses import dataclass
from typing import Any, Dict, List

from .config import ImporteeConfig


@dataclass
class Issue:
    rule_name: str
    path: pathlib.Path
    line: int
    message: str

    def __str__(self) -> str:  # pragma: no cover - trivial
        return f"{self.path}:{self.line}: {self.message}"


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
    options: Dict[str, Any], verbose: bool, quiet: bool, no_cache: bool
) -> Dict[str, Any]:
    run_cfg: Dict[str, Any] = {
        "verbose": bool(verbose),
        # Note: 'quiet' is handled by Python layer, not passed to Rust
    }
    # CLI flag overrides config option; fall back to option if flag not set
    cfg_no_cache = bool(options.get("no_cache")) if isinstance(options, dict) else False
    if no_cache or cfg_no_cache:
        run_cfg["no_cache"] = True
    return run_cfg


def run_check(
    config: ImporteeConfig,
    verbose: bool = False,
    quiet: bool = False,
    no_cache: bool = False,
) -> List[Issue]:
    # Defer heavy lifting to Rust extension
    try:
        from . import _rust
    except Exception as exc:  # pragma: no cover
        raise RuntimeError("Rust extension not available") from exc

    # Build project config
    source_module = _coerce_str_list(config.options.get("source_module"))
    source_modules = [source_module] if source_module else []
    # Rules: allow single or array of tables for linear
    rules = config.options.get("rules") or {}
    linear_opt = rules.get("linear") if isinstance(rules, dict) else None
    linear_rules: List[Dict[str, Any]] = []
    if isinstance(linear_opt, list):
        for item in linear_opt:
            if isinstance(item, dict):
                order = _coerce_str_list(item.get("order"))
                src = item.get("source_module")
                linear_rules.append(
                    {"order": order, "source_module": src}
                    if src is not None
                    else {"order": order}
                )
    elif isinstance(linear_opt, dict):
        order = _coerce_str_list(linear_opt.get("order"))
        src = linear_opt.get("source_module")
        linear_rules.append(
            {"order": order, "source_module": src}
            if src is not None
            else {"order": order}
        )

    project_cfg = {
        "source_modules": source_modules,
        "rules": {"linear": linear_rules},
        # Note: project_root is dynamically determined by Rust code from file paths
    }

    run_cfg = _build_run_config(config.options, verbose, quiet, no_cache)

    result_json = _rust.check_imports(json.dumps(project_cfg), json.dumps(run_cfg))
    # The Rust currently prints diagnostics and returns an empty issues list
    try:
        payload = json.loads(result_json)
    except Exception:
        return []

    issues: List[Issue] = []
    for item in payload.get("issues", []):
        path = pathlib.Path(item.get("path", "."))
        rule_name = item.get("rule_name", "")
        line = int(item.get("line", 0))
        msg = str(item.get("message", ""))
        issues.append(Issue(rule_name, path, line, msg))
    return issues
