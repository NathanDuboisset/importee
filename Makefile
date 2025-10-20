
dev:
	maturin develop
	pyo3-stubgen importee._rust .

rebuild:
	cargo clean
	maturin develop
	pyo3-stubgen importee._rust .