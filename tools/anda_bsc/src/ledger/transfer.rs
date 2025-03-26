//! Enables AI Agent to perform ICP token transfers
//!
//! Provides functionality for transferring tokens between accounts on the Internet Computer Protocol (ICP) network.
//! Supports:
//! - Multiple token types (e.g., ICP, PANDA)
//! - Memo fields for transaction identification
//! - Integration with ICP ledger standards
//! - Atomic transfers with proper error handling

use anda_core::{
    BoxError, FunctionDefinition, Resource, ToolOutput, gen_schema_for,
};
use anda_engine::context::BaseCtx;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;

use super::{BSCLedgers, ERC20STD};

use alloy::{
    primitives::Address, 
    providers::Provider, 
};

/// Arguments for transferring tokens to an account
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct TransferToArgs {
    /// ICP account address (principal) to receive token, e.g. "77ibd-jp5kr-moeco-kgoar-rro5v-5tng4-krif5-5h2i6-osf2f-2sjtv-kqe"
    pub account: String,
    /// Token symbol, e.g. "ICP"
    pub symbol: String,
    /// Token amount, e.g. 1.1 ICP
    pub amount: String,
}

/// Implementation of the ICP Ledger Transfer tool
#[derive(Debug, Clone)]
pub struct TransferTool {
    ledgers: Arc<BSCLedgers>,
    schema: Value,
}

impl TransferTool {
    pub const NAME: &'static str = "bsc_ledger_transfer";

    pub fn new(ledgers: Arc<BSCLedgers>) -> Self {
        let schema = gen_schema_for::<TransferToArgs>();

        TransferTool { ledgers, schema }
    }
}

/// Implementation of the [`Tool`] trait for TransferTool
/// Enables AI Agent to perform ICP token transfers
impl TransferTool {

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
        if tokens.len() > 1 {
            format!(
                "Transfer {} tokens to the specified account on ICP blockchain.",
                tokens.join(", ")
            )
        } else {
            format!(
                "Transfer {} token to the specified account on ICP blockchain.",
                tokens[0]
            )
        }
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
        me: Address,
        data: TransferToArgs,
        _resources: Option<Vec<Resource>>,
    ) -> Result<ToolOutput<String>, BoxError> {
        let (ledger, tx) = self.ledgers.transfer(ctx, me, data).await.unwrap();
        Ok(ToolOutput::new(format!(
            "Successful, transaction ID: {}, detail: https://www.icexplorer.io/token/details/{}",
            tx,
            ledger.to_string()
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use alloy::{
        providers::ProviderBuilder,
        primitives::U256,
    };

    #[tokio::test(flavor = "current_thread")]
    async fn test_bsc_ledger_transfer() {
        let rpc_url = "https://bsc-testnet.bnbchain.org";
        // Anvil for simulation.
        let provider =
        ProviderBuilder::new().on_anvil_with_wallet_and_config(|anvil| anvil.fork(rpc_url)).unwrap();

        let accounts = provider.get_accounts().await.unwrap();
        let alice = accounts[0];
        let bob = accounts[1];
    
        let contract = ERC20STD::deploy(provider.clone()).await.unwrap();

        // Get token symbol.
        let symbol = contract.symbol().call().await.unwrap()._0;
        let token_addr = contract.address();

        // Register the balances of Alice and Bob before the transfer.
        let alice_before_balance = contract.balanceOf(alice).call().await.unwrap()._0;
        let bob_before_balance = contract.balanceOf(bob).call().await.unwrap()._0;
        // println!("alice_before_balance: {:?}", alice_before_balance);
        // println!("bob_before_balance: {:?}", bob_before_balance);

        let ledgers = BSCLedgers {
            ledgers: BTreeMap::from([
                (
                    symbol.clone(),
                    (
                        *token_addr,
                        8,
                    ),
                ),
            ])
        };

        let tool = TransferTool::new(Arc::new(ledgers));
        let definition = tool.definition();
        assert_eq!(definition.name, "bsc_ledger_transfer");
        assert_eq!(tool.description().contains(&symbol), true);

        let transfer_amount = U256::from(100);
        let transfer_to_args = TransferToArgs {
            account: bob.to_string(),
            symbol: symbol.clone(),
            amount: transfer_amount.to_string(),
        };

        let call_result = tool.call(&provider, alice, transfer_to_args, None).await.unwrap();
        assert!(call_result.output.contains("Successful"));

        // // Transfer and wait for inclusion.
        // let tx_hash = contract.transfer(bob, amount).send().await.unwrap().watch().await.unwrap();

        // // Register the balances of Alice and Bob after the transfer.
        let alice_after_balance = contract.balanceOf(alice).call().await.unwrap()._0;
        let bob_after_balance = contract.balanceOf(bob).call().await.unwrap()._0;

        // Check the balances of Alice and Bob after the transfer.
        assert_eq!(alice_before_balance - alice_after_balance, transfer_amount);
        assert_eq!(bob_after_balance - bob_before_balance, transfer_amount);

    }
}
