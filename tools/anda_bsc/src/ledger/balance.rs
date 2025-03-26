//! Enables AI Agent to query the balance of an account for a ICP token
//!
//! This module provides functionality for querying account balances on the ICP network.
//! It implements the [`Tool`] trait to enable AI agents to interact with ICP ledgers.

use anda_core::{BoxError, FunctionDefinition, Resource, ToolOutput, gen_schema_for};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::sync::Arc;
use alloy::providers::Provider;
use eyre::Result;

use super::{BSCLedgers, ERC20STD};

/// Arguments for the balance of an account for a token
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub(crate) struct BalanceOfArgs {  // TODO: if to merge with the same struct in icp
    /// ICP account address (principal) to query, e.g. "77ibd-jp5kr-moeco-kgoar-rro5v-5tng4-krif5-5h2i6-osf2f-2sjtv-kqe"
    pub account: String,
    /// Token symbol, e.g. "ICP"
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
impl BalanceOfTool {

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
            "Query the balance of the specified account on BSC blockchain for the following tokens: {}",
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
        ctx: &impl Provider,
        data: BalanceOfArgs,
        _resources: Option<Vec<Resource>>,
    ) -> Result<ToolOutput<String>, BoxError> {
        let (_, amount) = self.ledgers.balance_of(ctx, data).await?;
        Ok(ToolOutput::new(amount))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use alloy::primitives::address;
    use alloy::providers::ProviderBuilder;
    
    #[tokio::test(flavor = "current_thread")]
    async fn test_bsc_ledger_balance() {
        // Create a provider with the HTTP transport using the `reqwest` crate.
        let rpc_url = "https://bsc-testnet.bnbchain.org".parse().unwrap();
        let provider = ProviderBuilder::new().on_http(rpc_url);
        let token_addr = address!("0xDE3a190D9D26A8271Ae9C27573c03094A8A2c449");
        let contract = ERC20STD::new(token_addr, provider.clone());

        // Get token symbol.
        let symbol = contract.symbol().call().await.unwrap()._0;

        // Create two users, token dev and Bob.
        let token_dev = address!("0xA8c4AAE4ce759072D933bD4a51172257622eF128");
        let bob = address!("0xd69BddCf538da91f66EE165C6244f59122C1Ff52");

        let dev_before_balance = contract.balanceOf(token_dev).call().await.unwrap()._0;
        let bob_before_balance = contract.balanceOf(bob).call().await.unwrap()._0;
        println!("dev_before_balance: {:?}", dev_before_balance);
        println!("bob_before_balance: {:?}", bob_before_balance);

        let ledgers = BSCLedgers {
            ledgers: BTreeMap::from([
                (
                    symbol.clone(),
                    (
                        token_addr,
                        8,
                    ),
                ),
            ])
        };
        let tool = BalanceOfTool::new(Arc::new(ledgers));
        let definition = tool.definition();
        assert_eq!(definition.name, "bsc_ledger_balance_of");
        assert_eq!(tool.description().contains(&symbol), true);

        let balance_query = BalanceOfArgs {
            account: token_dev.to_string(),
            symbol: symbol.clone(),
        };

        let balance_tool = tool.call(&provider, balance_query, None).await.unwrap();
        assert_eq!(balance_tool.output, dev_before_balance.to_string());
        
    }
}
