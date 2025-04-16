//! Enables AI Agent to query the balance of an account for a ICP token
//!
//! This module provides functionality for querying account balances on the ICP network.
//! It implements the [`Tool`] trait to enable AI agents to interact with ICP ledgers.

use anda_core::{BoxError, FunctionDefinition, Resource, Tool, ToolOutput, gen_schema_for};
use anda_engine::context::BaseCtx;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::sync::Arc;
use super::BSCLedgers;

/// Arguments for the balance of an account for a token
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct BalanceOfArgs {
    /// account address
    pub account: String,
    /// Token symbol, e.g. "cake"
    pub symbol: String,
}

/// ICP Ledger BalanceOf tool implementation
#[derive(Debug, Clone)]
pub struct BalanceOfTool {
    ledgers: Arc<BSCLedgers>,
    schema: Value,
}

impl BalanceOfTool {
    pub const NAME: &'static str = "bsc_ledger_balance_of";
    /// Creates a new BalanceOfTool instance
    pub fn new(ledgers: Arc<BSCLedgers>) -> Self {
        let schema = gen_schema_for::<BalanceOfArgs>();

        BalanceOfTool {
            ledgers,
            schema: json!(schema),
        }
    }
}

/// Implementation of the [`Tool`]` trait for BalanceOfTool
/// Enables AI Agent to query the balance of an account for a ICP token
impl Tool<BaseCtx> for BalanceOfTool {
    type Args = BalanceOfArgs;
    type Output = String;

    fn name(&self) -> String {
        Self::NAME.to_string()
    }

    fn description(&self) -> String {
        let tokens = self
            .ledgers
            .ledgers
            .keys()
            .map(|k| k.as_str())
            .collect::<Vec<_>>();
        format!(
            "Query the balance of the specified account on ICP blockchain for the following tokens: {}",
            tokens.join(", ")
        )
    }

    fn definition(&self) -> FunctionDefinition {
        FunctionDefinition {
            name: self.name(),
            description: self.description(),
            parameters: self.schema.clone(),
            strict: Some(true),
        }
    }

    async fn call(
        &self,
        ctx: BaseCtx,
        data: Self::Args,
        _resources: Option<Vec<Resource>>,
    ) -> Result<ToolOutput<Self::Output>, BoxError> {
        let token_symbol = data.symbol.clone();
        let (address, amount) = self.ledgers.balance_of(ctx, data).await?;
        Ok(ToolOutput::new(format!(
            "Successful {} balance query, user address: {}, balance {}",
            token_symbol,
            address.to_string(),
            amount
        )))

    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use alloy::{
        hex, primitives::Address, providers::ProviderBuilder
    };
    use anda_web3_client::client::{
        Client as Web3Client, load_identity
    };
    use anda_engine::{
        extension::extractor::Extractor,
        context::Web3SDK,
        engine::EngineBuilder,
    };
    use crate::ledger::{bsc_rpc, DRVT_PATH, TOKEN_ADDR};
    use crate::signer::derive_address_from_pubkey;
    use super::super::ERC20STD;
    use anda_core::KeysFeatures;

    #[tokio::test]
    async fn test_bsc_ledger_balance() {
        let _ = env_logger::builder()
        .is_test(true)
        .filter_level(log::LevelFilter::Debug)
        .try_init();

        // Read identity and root secret
        let id_secret = dotenv::var("ID_SECRET").unwrap();
        let root_secret_org = dotenv::var("ROOT_SECRET").unwrap();

        // Parse and validate cryptographic secrets
        let identity = load_identity(&id_secret).unwrap();
        let root_secret = const_hex::decode(&root_secret_org).unwrap();
        let root_secret: [u8; 48] = root_secret
            .try_into()
            .map_err(|_| format!("invalid root_secret: {:?}", &root_secret_org))
            .unwrap();

        // Initialize Web3 client for ICP network interaction
        let web3 = Web3Client::builder()
            // .with_ic_host(url)  // Todo: Why local url, i.e "localhost:8545", would hang the programe?
            .with_identity(Arc::new(identity))
            .with_root_secret(root_secret)
            .build().await.unwrap();

        // Create provider
        let rpc_url = bsc_rpc().parse().unwrap();
        let provider = ProviderBuilder::new().on_http(rpc_url);

        // ERC20 token contract address and instance
        let token_addr =  Address::parse_checksummed(TOKEN_ADDR, None).unwrap();
        let contract = ERC20STD::new(token_addr, provider.clone());

        // Get token symbol and decimals
        let symbol = contract.symbol().call().await.unwrap();
        let decimals = contract.decimals().call().await.unwrap();
        log::debug!("symbol: {:?}, decimals: {:?}", symbol.clone(), decimals);

        // Create ledgers instance
        let ledgers = BSCLedgers {
            ledgers: BTreeMap::from([
                (
                    symbol.clone(),
                    (
                        token_addr,
                        decimals,
                    ),
                ),
            ])
        };
        let ledgers = Arc::new(ledgers);
        let tool = BalanceOfTool::new(ledgers.clone());
        let definition = tool.definition();
        assert_eq!(definition.name, "bsc_ledger_balance_of");

        // Create an agent for testing
        #[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
        struct TestStruct {
            name: String,
            age: Option<u8>,
        }    
        let agent = Extractor::<TestStruct>::default();

        // Create a context for testing
        let engine_ctx = EngineBuilder::new()
                    .with_name("BSC_TEST".to_string()).unwrap()
                    .with_web3_client(Arc::new(Web3SDK::from_web3(Arc::new(web3))))
                    .register_agent(agent).unwrap()
                    .mock_ctx();
        let base_ctx = engine_ctx.base.clone();

        // Derive EVM address from derivation path
        let pubkey_bytes = base_ctx.secp256k1_public_key(DRVT_PATH)
            .await
            .map_err(|e| 
                format!("Failed to get public key from derivation path: {:?}. Error: {:?}", DRVT_PATH, e.to_string()
                ) 
            ).unwrap();
        let user_address = derive_address_from_pubkey(&pubkey_bytes).unwrap();
        log::debug!("User pubkey: {:?}, User EVM address: {:?}",
                    hex::encode(pubkey_bytes), user_address);

        // Create arguments for balance query
        let args = BalanceOfArgs {
            account: user_address.to_string(),
            symbol: symbol.clone(),
        };
        // Call the tool to query balance
        let res = tool.call(base_ctx, args, None).await;
        assert!(res.is_ok(), "Balance query failed: {:?}", res);
        println!("Balance query result: {:#?}", res.unwrap());
    }}
