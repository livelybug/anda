use anda_cognitive_nexus::CognitiveNexus;
use anda_core::BoxError;
use anda_db::database::{AndaDB, DBConfig};
use anda_engine::store::LocalFileSystem;
use anda_kip::{CommandType, KipError, Request, Response};
use object_store::memory::InMemory;
use pyo3::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::sync::Arc;
use anda_object_store::{MetaStore, MetaStoreBuilder};

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

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum StoreLocationType {
    InMem,
    LocalFile,
}

#[derive(Debug, Deserialize)]
pub struct AndaDbConfig {
    pub store_location_type: StoreLocationType,
    pub store_location: String, // changed from Option<String> to String
    pub DB_name: String,
    pub DB_desc: Option<String>,
    pub meta_cache_capacity: Option<u64>,
}

impl AndaDbConfig {
    /// Verifies the configuration for AndaDbConfig.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - `store_location_type` is `LocalFile` and `store_location` is empty.
    /// - `store_location` does not exist on the filesystem.
    pub fn verify_config(&self) -> Result<(), String> {
        if let StoreLocationType::LocalFile = self.store_location_type {
            if self.store_location.trim().is_empty() {
                return Err("store_location is required when store_location_type is LocalFile".to_string());
            }
            use std::path::Path;
            if !Path::new(&self.store_location).exists() {
                return Err(format!("store_location path does not exist: {}", self.store_location));
            }
        }
        Ok(())
    }
}

/// Executes a KIP command using the Anda cognitive engine.
///
/// # Arguments
///
/// * `command` - The KIP command string to execute (KML/KQL/META).
/// * `parameters` - A JSON value containing command parameters.
/// * `dry_run` - If true, performs a dry run without committing changes.
/// * `db_config` - Database configuration as an `AndaDbConfig` struct.
///     - `store_location_type`: `"InMem"` for in-memory DB, `"LocalFile"` for file-backed DB.
///     - `store_location`: Required if `store_location_type` is `"LocalFile"`.
///     - `DB_name`: Name of the database.
///     - `DB_desc`: Optional description of the database.
///     - `meta_cache_capacity`: Optional cache capacity for metadata (default: 10000).
///
/// # Returns
///
/// Returns a tuple of the command type and the response on success, or a boxed error on failure.
///
/// # Errors
///
/// Returns an error if the database configuration is invalid, required fields are missing,
/// or if the KIP command execution fails.
///
/// # Example
/// 
/// Refer to tools/anda_py/examples directory
pub async fn execute_kip(
    command: String,
    parameters: Value, // Map<String, Json>, pub type Json = serde_json::Value;
    dry_run: bool,
    db_config: AndaDbConfig,
) -> Result<(CommandType, Response), BoxError> {
    // Verify db_config before proceeding
    db_config.verify_config()
        .map_err(|e| KipError::Execution(e))?;

    // Parse and validate db_config
    let db_name = db_config.DB_name.as_str();
    let db_desc = db_config.DB_desc.as_deref().unwrap_or_default();
    let meta_cache_capacity = db_config.meta_cache_capacity.unwrap_or(10000);

    let object_store: Arc<dyn object_store::ObjectStore> = match db_config.store_location_type {
        StoreLocationType::InMem => Arc::new(InMemory::new()),
        StoreLocationType::LocalFile => {
            let local_file = MetaStoreBuilder::new(
                LocalFileSystem::new_with_prefix(&db_config.store_location)
                    .map_err(|err| KipError::Execution(err.to_string()))?,
                meta_cache_capacity,
            ).build();
            Arc::new(local_file)
        }
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