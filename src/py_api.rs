use pyo3::prelude::*;

use crate::configs::{ProjectConfig, RunConfig};
use crate::walker::run_check_imports;

/// Run the importee checker, parse the project and run config and return the results as a string
#[pyfunction]
fn check_imports(project_config: String, run_config: String) -> PyResult<String> {
    let project_config: ProjectConfig = serde_json::from_str(&project_config).map_err(|e| {
        PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("project_config json error: {}", e))
    })?;
    let run_config: RunConfig = serde_json::from_str(&run_config).map_err(|e| {
        PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("run_config json error: {}", e))
    })?;

    let result = run_check_imports(project_config, run_config);
    let json = serde_json::to_string(&result).map_err(|e| {
        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("serialize error: {}", e))
    })?;
    Ok(json)
}

/// Python module definition
#[pymodule]
fn _rust(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(check_imports, m)?)?;
    Ok(())
}
