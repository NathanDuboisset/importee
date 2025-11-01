# Importee

Python import checker with Rust backend. Checks import ordering rules in Python projects.

## Features

- Rust-powered import analysis
- Configurable via `pyproject.toml`
- Linear ordering rules for enforcing module dependencies
- CLI interface

## Installation

### From PyPI

```bash
pip install importee
```

### From Snap Store

```bash
snap install importee
```

### From Source

```bash
git clone https://github.com/yourusername/importee.git
cd importee
pip install maturin
maturin develop
```

## Quick Start

Check your project's imports:

```bash
importee check
```

Configuration is done via `pyproject.toml`:

```toml
[tool.importee]
source_module = ["myproject"]

[tool.importee.rules.linear]
order = ["models", "utils", "api", "cli"]
```

## Configuration

Importee reads configuration from your `pyproject.toml` file. Here's what you can configure:

### Basic Configuration

```toml
[tool.importee]
# Modules to check
source_module = ["myapp"]
```

### Linear Ordering Rules

Enforce a specific order for imports within your project:

```toml
[tool.importee.rules.linear]
# Modules must be imported in this order
order = ["config", "database", "models", "services", "api"]
```

This ensures that modules listed earlier in the order cannot import from modules listed later.

## Development

### Prerequisites

- Python 3.9+
- Rust 1.70+
- Maturin

### Building

```bash
# Development build
make dev

# Clean rebuild
make rebuild
```

### Running Tests

```bash
pytest tests/
```

## License

[Add your license here]

## Deployment

See [DEPLOYMENT.md](DEPLOYMENT.md) for information on releasing and deploying this project.

