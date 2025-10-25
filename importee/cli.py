import pathlib
import sys

import click

from .config import discover_config
from .checker import run_check


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
def check_cmd(
    target_path: pathlib.Path, config: pathlib.Path, quiet: bool, verbose: bool
) -> None:
    """Scan a directory for invalid imports."""
    if verbose and quiet:
        raise click.UsageError("--quiet and --verbose are mutually exclusive")
    cfg = discover_config(target_path, config)
    issues = run_check(cfg, verbose, quiet)
    if issues:
        for issue in issues:
            click.echo(str(issue), err=True)
        sys.exit(1)
    click.echo("No issues found.")


if __name__ == "__main__":  # pragma: no cover
    cli()
