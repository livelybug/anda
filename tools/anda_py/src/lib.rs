use anda_cognitive_nexus::CognitiveNexus;
use anda_core::BoxError;
use anda_db::database::{AndaDB, DBConfig};
use anda_engine::store::LocalFileSystem;
use anda_kip::{CommandType, KipError, Request, Response};
use object_store::memory::InMemory;
use pyo3::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::sync::Arc;
use anda_object_store::{MetaStore, MetaStoreBuilder};
use anda_kip::Json;
use anda_kip::executor::Executor;

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

/// Create a CognitiveNexus instance from AndaDbConfig.
/// Returns an Arc-wrapped Nexus for use in KIP execution.
///
/// # Errors
/// Returns an error if the config is invalid or DB/Nexus creation fails.
pub async fn create_kip_db(
    db_config: AndaDbConfig,
) -> Result<Arc<CognitiveNexus>, BoxError> {
    db_config.verify_config()
        .map_err(|e| KipError::Execution(e))?;

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

    let db_config = DBConfig {
        name: db_name.to_string(),
        description: db_desc.to_string(),
        ..Default::default()
    };

    let db = Arc::new(AndaDB::connect(object_store, db_config).await?);
    let nexus = Arc::new(CognitiveNexus::connect(db, async |_| Ok(())).await?);
    Ok(nexus)
}

/// Executes a KIP command using an existing Executor instance.
///
/// # Arguments
///
/// * `nexus` - Reference to an Executor instance (`&(impl Executor + Sync)`).
/// * `command` - The KIP command string to execute (KML/KQL/META).
/// * `parameters` - An optional map of command parameters (`Option<Map<String, Json>>`). If `None`, treated as empty.
/// * `dry_run` - If true, performs a dry run without committing changes.
///
/// # Returns
///
/// Returns a tuple of the command type and the response on success, or a boxed error on failure.
///
/// # Errors
///
/// Returns an error if the KIP command execution fails.
///
/// # Example
/// 
/// Refer to tools/anda_py/examples directory
pub async fn execute_kip(
    nexus: &(impl Executor + Sync),
    command: String,
    parameters: Option<Map<String, Json>>,
    dry_run: bool,
) -> Result<(CommandType, Response), BoxError> {
    let params_map = parameters.unwrap_or_default();

    let request = Request {
        command,
        parameters: params_map,
        dry_run,
    };

    Ok(request.execute(nexus).await)
}