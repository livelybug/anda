use anda_cognitive_nexus::CognitiveNexus;
use anda_core::BoxError;
use anda_db::database::{AndaDB, DBConfig};
use anda_engine::store::LocalFileSystem;
use anda_kip::{CommandType, KipError, Request, Response};
use object_store::memory::InMemory;
use pyo3::prelude::*;
use serde_json::{Map, Value};
use std::sync::Arc;
use anda_object_store::MetaStoreBuilder;

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
    db_config: Value,
) -> Result<(CommandType, Response), BoxError> {
    // Parse and validate db_config
    let obj = db_config.as_object().ok_or("db_config must be a JSON object")?;
    let store_location_type = obj.get("store_location_type")
        .and_then(|v| v.as_str())
        .ok_or("Missing or invalid 'store_location_type' in db_config")?;
    let db_name = obj.get("DB_name")
        .and_then(|v| v.as_str())
        .ok_or("Missing or invalid 'DB_name' in db_config")?;
    let db_desc = obj.get("DB_desc")
        .and_then(|v| v.as_str())
        .unwrap_or_default();

    // Setup object store
    let object_store: Arc<dyn object_store::ObjectStore> = match store_location_type {
        "in_mem" => Arc::new(InMemory::new()),
        "local_file" => {
            let store_location = obj.get("store_location")
                .and_then(|v| v.as_str())
                .ok_or("Missing or invalid 'store_location' for local_file in db_config")?;
            let local_file =MetaStoreBuilder::new(
                    LocalFileSystem::new_with_prefix(store_location)
                        .map_err(|err| KipError::Execution(err.to_string()))?,
                    10000,
                ).build();
            Arc::new(local_file)
        }
        _ => return Err(format!("Invalid store_location_type: {}", store_location_type).into()),
    };

    // Setup DB config
    let db_config = DBConfig {
        name: db_name.to_string(),
        description: db_desc.to_string(),
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