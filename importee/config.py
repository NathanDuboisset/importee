from __future__ import annotations

import configparser
import dataclasses
import pathlib
from typing import Any, Dict, Optional

try:
    import tomllib  # Python 3.11+
except ModuleNotFoundError:  # pragma: no cover
    import tomli as tomllib  # type: ignore


@dataclasses.dataclass
class ImporteeConfig:
    target_dir: pathlib.Path
    # Placeholder for future configurable options
    options: Dict[str, Any] = dataclasses.field(default_factory=dict)


def _load_from_pyproject(start_dir: pathlib.Path) -> Optional[Dict[str, Any]]:
    current = start_dir
    for parent in [current] + list(current.parents):
        pyproject = parent / "pyproject.toml"
        if pyproject.is_file():
            try:
                with pyproject.open("rb") as f:
                    data = tomllib.load(f)
                tool = data.get("tool", {})
                cfg = tool.get("importee")
                if isinstance(cfg, dict):
                    return cfg
            except Exception:
                return None
    return None


def _load_from_ini(start_dir: pathlib.Path) -> Optional[Dict[str, Any]]:
    current = start_dir
    for parent in [current] + list(current.parents):
        ini = parent / "importee.ini"
        if ini.is_file():
            parser = configparser.ConfigParser()
            try:
                parser.read(ini)
                if parser.has_section("importee"):
                    return {k: v for k, v in parser.items("importee")}
            except Exception:
                return None
    return None


def discover_config(
    target_dir: pathlib.Path, config_file: Optional[pathlib.Path] = None
) -> ImporteeConfig:
    target_dir = target_dir.resolve()
    if config_file is not None:
        config_file = config_file.resolve()
        if config_file.name == "pyproject.toml":
            with config_file.open("rb") as f:
                data = tomllib.load(f)
            tool = data.get("tool", {})
            cfg_data = tool.get("importee", {}) if isinstance(tool, dict) else {}
        else:
            parser = configparser.ConfigParser()
            parser.read(config_file)
            cfg_data = (
                {k: v for k, v in parser.items("importee")}
                if parser.has_section("importee")
                else {}
            )
    else:
        cfg_data = _load_from_pyproject(target_dir) or _load_from_ini(target_dir) or {}
    return ImporteeConfig(target_dir=target_dir, options=cfg_data)
