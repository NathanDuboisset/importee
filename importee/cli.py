import pathlib
import sys

import click

from .config import discover_config
from .checker import run_check


def check_return_status(
    target_path: pathlib.Path,
    config: pathlib.Path | None,
    quiet: bool,
    verbose: bool,
    no_cache: bool,
) -> bool:
    """Programmatic API: run check and return True if no issues, False otherwise.

    Also prints summary and timing like the CLI, honoring quiet/verbose.
    """
    import time

    start = time.perf_counter()
    cfg = discover_config(target_path, config)  # type: ignore[arg-type]
    issues = run_check(cfg, verbose=verbose, quiet=quiet, no_cache=no_cache)
    elapsed = time.perf_counter() - start
    had_issues = bool(issues)
    if had_issues:
        if not quiet:
            click.echo(f"\n=== Found {len(issues)} issues ===\n")
        for issue in issues:
            click.echo(f"[{issue.rule_name}] {str(issue)}", err=True)
        if not quiet:
            click.echo(f"\nCompleted in {elapsed:.3f}s")
        return False
    if not quiet:
        click.echo("No issues found.")
        click.echo(f"Completed in {elapsed:.3f}s")
    return True


@click.group(context_settings={"help_option_names": ["-h", "--help"]})
def cli() -> None:
    """Importee CLI."""


@cli.command("check")
@click.option(
    "--path",
    "target_path",
    type=click.Path(
        path_type=pathlib.Path,
        exists=True,
        file_okay=False,
        dir_okay=True,
        resolve_path=True,
    ),
    default=".",
    show_default=True,
    help="Target directory to analyze",
)
@click.option(
    "--config",
    type=click.Path(
        path_type=pathlib.Path,
        exists=True,
        file_okay=True,
        dir_okay=False,
        resolve_path=True,
    ),
    help="Config file to use",
)
@click.option(
    "--quiet",
    "-q",
    is_flag=True,
    help="Quiet output (only issues)",
)
@click.option(
    "--verbose",
    "-v",
    is_flag=True,
    help="Verbose output",
)
@click.option(
    "--no-cache",
    is_flag=True,
    help="Disable import cache",
)
@click.option(
    "--no-exit",
    is_flag=True,
    help="Do not exit with status; return True/False instead",
)
def check_cmd(
    target_path: pathlib.Path,
    config: pathlib.Path,
    quiet: bool,
    verbose: bool,
    no_cache: bool,
    no_exit: bool,
) -> None:
    """Scan a directory for invalid imports."""
    if verbose and quiet:
        raise click.UsageError("--quiet and --verbose are mutually exclusive")
    ok = check_return_status(target_path, config, quiet, verbose, no_cache)
    if not ok and not no_exit:
        sys.exit(1)


@cli.command("clear-cache")
@click.option(
    "--path",
    "target_path",
    type=click.Path(
        path_type=pathlib.Path,
        exists=True,
        file_okay=False,
        dir_okay=True,
        resolve_path=True,
    ),
    default=".",
    show_default=True,
    help="Directory inside the project to locate the project root from",
)
@click.option(
    "--config",
    type=click.Path(
        path_type=pathlib.Path,
        exists=True,
        file_okay=True,
        dir_okay=False,
        resolve_path=True,
    ),
    help="Optional pyproject.toml or .ini to locate project root",
)
def clear_cache_cmd(target_path: pathlib.Path, config: pathlib.Path | None) -> None:
    """Remove the .importee_cache directory at the project root."""

    def find_project_root(start: pathlib.Path) -> pathlib.Path:
        cur = start.resolve()
        while True:
            if (cur / "pyproject.toml").exists():
                return cur
            if cur.parent == cur:
                return start
            cur = cur.parent

    root = config.parent if config is not None else find_project_root(target_path)
    cache_dir = root / ".importee_cache"
    if cache_dir.exists():
        import shutil

        try:
            shutil.rmtree(cache_dir)
            click.echo(f"Removed cache directory: {cache_dir}")
        except Exception as exc:  # pragma: no cover
            click.echo(f"Failed to remove cache: {exc}", err=True)
            sys.exit(1)
    else:
        click.echo("No cache directory to remove.")


if __name__ == "__main__":  # pragma: no cover
    cli()
