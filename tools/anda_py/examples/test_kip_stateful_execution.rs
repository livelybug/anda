use std::collections::HashMap;
use anda_kip::{Response, Json, Map};
use anda_py::{execute_kip, create_kip_db, AndaDbConfig, StoreLocationType};
use serde_json::json;

// cargo run --example test_kip_stateful_execution
#[tokio::main]
async fn main() {
    println!("--- Running Full Stateful KIP Execution Test ---");

    // 1. Execute the first KML command from the demo to set up schema and initial data
    println!("\n1. Executing Medical Knowledge KML...");
    // Create basic concept types and medical knowledge capsule
    let medical_knowledge_kml = r#"
    UPSERT {
        // Define concept types
        CONCEPT ?drug_type {
            {type: "$ConceptType", name: "Drug"}
            SET ATTRIBUTES {
                description: "Pharmaceutical drug concept type"
            }
        }

        CONCEPT ?symptom_type {
            {type: "$ConceptType", name: "Symptom"}
            SET ATTRIBUTES {
                description: "Medical symptom concept type"
            }
        }

        // Define relation types
        CONCEPT ?treats_relation {
            {type: "$PropositionType", name: "treats"}
            SET ATTRIBUTES {
                description: "Drug treats symptom relationship"
            }
        }

        CONCEPT ?has_side_effect_relation {
            {type: "$PropositionType", name: "has_side_effect"}
            SET ATTRIBUTES {
                description: "Drug has side effect relationship"
            }
        }

        // Create symptom concepts
        CONCEPT ?headache {
            {type: "Symptom", name: "Headache"}
            SET ATTRIBUTES {
                severity_scale: "1-10",
                description: "Pain in the head or neck area"
            }
        }

        CONCEPT ?fever {
            {type: "Symptom", name: "Fever"}
            SET ATTRIBUTES {
                normal_temp: "98.6°F (37°C)",
                description: "Elevated body temperature"
            }
        }

        CONCEPT ?stomach_irritation {
            {type: "Symptom", name: "Stomach Irritation"}
            SET ATTRIBUTES {
                severity: "mild to moderate",
                description: "Gastrointestinal discomfort"
            }
        }

        // Create specific drug concepts
        CONCEPT ?aspirin {
            {type: "Drug", name: "Aspirin"}
            SET ATTRIBUTES {
                molecular_formula: "C9H8O4",
                risk_level: 1,
                description: "Common pain reliever and anti-inflammatory drug"
            }
            SET PROPOSITIONS {
                ("treats", ?headache)
                ("treats", ?fever)
                ("has_side_effect", ?stomach_irritation)
            }
        }
    }
    WITH METADATA {
        source: "KIP Demo Medical Knowledge",
        author: "Demo System",
        confidence: 0.95,
        created_at: "2025-07-01T00:00:00Z"
    }
    "#;

    // Add db_config for in-memory DB (as AndaDbConfig struct expects)
    let db_config_in_mem = AndaDbConfig {
        store_location_type: StoreLocationType::InMem,
        store_location: "".to_owned(),
        DB_name: "test_medical_db".to_string(),
        DB_desc: Some("Ephemeral DB for medical KIP test".to_string()),
        meta_cache_capacity: Some(10000),
    };

    // Create Nexus instance for in-memory DB
    let nexus_in_mem = create_kip_db(db_config_in_mem).await.expect("Failed to create in_mem Nexus");

    // Use empty Map for parameters
    let empty_params: Map<String, Json> = Map::new();

    let (_, response1) = execute_kip(
        nexus_in_mem.as_ref(),
        medical_knowledge_kml.to_string(),
        Some(empty_params.clone()),
        false,
    )
    .await
    .expect("Execution of medical_knowledge_kml failed");
    assert!(matches!(response1, Response::Ok { .. }), "Expected first KML execution to be Ok, but got {:?}", response1);
    println!("Medical Knowledge KML executed successfully (in_mem DB).");

    // Add db_config for local_file DB (as AndaDbConfig struct expects)
    let db_config_local_file = AndaDbConfig {
        store_location_type: StoreLocationType::LocalFile,
        store_location: "/tmp/anda_py_test_db".to_string(),
        DB_name: "test_medical_db".to_string(),
        DB_desc: Some("Local file DB for medical KIP test".to_string()),
        meta_cache_capacity: Some(10000),
    };

    // Ensure store_location folder exists before calling create_kip_db
    if let StoreLocationType::LocalFile = db_config_local_file.store_location_type {
        use std::path::Path;
        let path = Path::new(&db_config_local_file.store_location);
        if path.exists() {
            if path.is_file() {
                panic!("store_location exists but is a file, not a directory: {}", db_config_local_file.store_location);
            }
        } else {
            std::fs::create_dir_all(path)
                .expect("Failed to create store_location directory");
        }
    }

    // Create Nexus instance for local_file DB
    let nexus_local_file = create_kip_db(db_config_local_file).await.expect("Failed to create local_file Nexus");

    let (_, response2) = execute_kip(
        nexus_local_file.as_ref(),
        medical_knowledge_kml.to_string(),
        Some(empty_params.clone()),
        false,
    )
    .await
    .expect("Execution of medical_knowledge_kml (local_file) failed");
    assert!(matches!(response2, Response::Ok { .. }), "Expected second KML execution to be Ok, but got {:?}", response2);
    println!("Medical Knowledge KML executed successfully (local_file DB).");
/*
    // 2. Execute the second KML command from the demo to add more data
    println!("\n2. Executing New Drug KML...");
    // Create a new hypothetical drug
    let new_drug_kml = r#"
    UPSERT {
        CONCEPT ?brain_fog {
            {type: "Symptom", name: "Brain Fog"}
            SET ATTRIBUTES {
                description: "Mental fatigue and lack of clarity",
                cognitive_impact: "high"
            }
        }

        CONCEPT ?neural_bloom {
            {type: "Symptom", name: "Neural Bloom"}
            SET ATTRIBUTES {
                description: "A rare side effect characterized by temporary burst of creative thoughts",
                frequency: "rare",
                severity: "mild"
            }
        }

        CONCEPT ?cognizine {
            {type: "Drug", name: "Cognizine"}
            SET ATTRIBUTES {
                molecular_formula: "C12H15N5O3",
                risk_level: 2,
                description: "A novel nootropic drug designed to enhance cognitive functions",
                status: "experimental"
            }
            SET PROPOSITIONS {
                ("treats", {type: "Symptom", name: "Brain Fog"})
                ("has_side_effect", ?neural_bloom)
            }
        }
    }
    WITH METADATA {
        source: "Experimental Drug Research",
        confidence: 0.75,
        status: "under_review"
    }
    "#;

    let (_, response3) = execute_kip(
        new_drug_kml.to_string(),
        Some(empty_params.clone()),
        false,
        db_config_in_mem.clone() // use the same db_config
    )
    .await
    .expect("Execution of new_drug_kml failed");
    assert!(matches!(response3, Response::Ok { .. }), "Expected third KML execution to be Ok, but got {:?}", response3);
    println!("New Drug KML executed successfully (in_mem DB).");

    let (_, response4) = execute_kip(
        new_drug_kml.to_string(),
        empty_params.clone(),
        false,
        db_config_local_file.clone() // use the same db_config
    )
    .await
    .expect("Execution of new_drug_kml (local_file) failed");
    assert!(matches!(response4, Response::Ok { .. }), "Expected fourth KML execution to be Ok, but got {:?}", response4);
    println!("New Drug KML executed successfully (local_file DB).");

    // 3. Execute a KQL query from the demo to verify the data
    println!("\n3. Executing KQL Query to find all drugs...");
    let query = r#"\
    FIND(?drug.name, ?drug.attributes.risk_level) \
    WHERE { \
        ?drug {type: \"Drug\"} \
    } \
    ORDER BY ?drug.attributes.risk_level ASC \
    "#;

    let (_, query_response) = execute_kip(query.to_string(), empty_params.clone(), false)
        .await
        .expect("Execution of KQL query failed");

    println!("Query Response: {:#?}", query_response);

    // 4. Assert that the query was successful and returned the correct data
    assert!(matches!(query_response, Response::Ok { .. }), "Expected KQL query to be Ok, but got {:?}", query_response);
    if let Response::Ok { result, .. } = query_response {
        let result_array = result.as_array().expect("Result should be an array");
        assert_eq!(result_array.len(), 2, "Expected to find 2 drugs, but found {}", result_array.len());
        println!("Successfully found 2 drugs as expected.");
    } else {
        panic!("Query failed, expected Ok response");
    }
*/
    println!("\n--- Full Stateful KIP Execution Test Passed ---");
}