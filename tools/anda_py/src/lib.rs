use anda_cognitive_nexus::CognitiveNexus;
use anda_core::BoxError;
use anda_db::database::{AndaDB, DBConfig};
use anda_kip::{CommandType, Request, Response};
use object_store::memory::InMemory;
use pyo3::prelude::*;
use serde_json::{Map, Value};
use std::sync::Arc;

/// Formats the sum of two numbers as a string.
#[pyfunction]
fn sum_as_string(a: usize, b: usize) -> PyResult<String> {
    Ok((a + b).to_string())
}

/// A Python module implemented in Rust.
#[pymodule]
fn anda_py(_py: Python, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(sum_as_string, m)?)?;
    Ok(())
}

pub async fn execute_kip(
    command: String,
    parameters: Value,
    dry_run: bool,
) -> Result<(CommandType, Response), BoxError> {
    // 1. Setup the in-memory DB and Nexus
    let object_store = Arc::new(InMemory::new());
    let db_config = DBConfig {
        name: "anda_py_ephemeral".to_string(),
        ..Default::default()
    };
    let db = Arc::new(AndaDB::connect(object_store, db_config).await?);
    let nexus = Arc::new(CognitiveNexus::connect(db, async |_| Ok(())).await?);

    // 2. Create a KIP Request
    let params_map: Map<String, Value> = if let Value::Object(map) = parameters {
        map
    } else {
        Map::new()
    };

    let request = Request {
        command,
        parameters: params_map,
        dry_run,
    };

    // 3. Execute the request using its own method
    Ok(request.execute(nexus.as_ref()).await)
}