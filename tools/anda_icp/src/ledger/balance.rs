use anda_core::{BoxError, FunctionDefinition, Tool};
use anda_engine::context::BaseCtx;
use candid::Nat;
use schemars::{schema_for, JsonSchema};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;

use super::ICPLedgers;

/// Arguments for the balance of an account for a token
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
pub struct BalanceOfArgs {
    /// ICP account address (principal) to query, e.g. "77ibd-jp5kr-moeco-kgoar-rro5v-5tng4-krif5-5h2i6-osf2f-2sjtv-kqe"
    pub account: String,
    /// Token symbol, e.g. "ICP"
    pub symbol: String,
}

/// ICP Ledger BalanceOf tool implementation
#[derive(Debug, Clone)]
pub struct BalanceOfTool {
    ledgers: Arc<ICPLedgers>,
    schema: Value,
}

impl BalanceOfTool {
    /// Creates a new BalanceOfTool instance
    pub fn new(ledgers: Arc<ICPLedgers>) -> Self {
        let mut schema = schema_for!(BalanceOfArgs);
        schema.meta_schema = None; // Remove the $schema field

        BalanceOfTool {
            ledgers,
            schema: json!(schema),
        }
    }
}

impl Tool<BaseCtx> for BalanceOfTool {
    const CONTINUE: bool = true;
    type Args = BalanceOfArgs;
    type Output = Nat;

    fn name(&self) -> String {
        "icp_ledger_balance_of".to_string()
    }

    fn description(&self) -> String {
        let tokens = self
            .ledgers
            .ledgers
            .keys()
            .map(|k| k.as_str())
            .collect::<Vec<_>>();
        format!(
                "Query the balance of the specified account on ICP network for the following tokens: {}",
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

    async fn call(&self, ctx: BaseCtx, data: Self::Args) -> Result<Self::Output, BoxError> {
        self.ledgers.balance_of(&ctx, data).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use candid::Principal;
    use std::collections::BTreeMap;

    #[tokio::test(flavor = "current_thread")]
    async fn test_icp_ledger_transfer() {
        let panda_ledger = Principal::from_text("druyg-tyaaa-aaaaq-aactq-cai").unwrap();
        let ledgers = ICPLedgers {
            ledgers: BTreeMap::from([
                (
                    String::from("ICP"),
                    (
                        Principal::from_text("ryjl3-tyaaa-aaaaa-aaaba-cai").unwrap(),
                        8,
                    ),
                ),
                (String::from("PANDA"), (panda_ledger, 8)),
            ]),
            from_user_subaccount: true,
        };
        let ledgers = Arc::new(ledgers);
        let tool = BalanceOfTool::new(ledgers.clone());
        let definition = tool.definition();
        assert_eq!(definition.name, "icp_ledger_balance_of");
        let s = serde_json::to_string_pretty(&definition).unwrap();
        println!("{}", s);
        // {
        //     "name": "icp_ledger_balance_of",
        //     "description": "Query the balance of the specified account on ICP network for the following tokens: ICP, PANDA",
        //     "parameters": {
        //       "description": "Arguments for the balance of an account for a token",
        //       "properties": {
        //         "account": {
        //           "description": "ICP account address (principal) to query, e.g. \"77ibd-jp5kr-moeco-kgoar-rro5v-5tng4-krif5-5h2i6-osf2f-2sjtv-kqe\"",
        //           "type": "string"
        //         },
        //         "symbol": {
        //           "description": "Token symbol, e.g. \"ICP\"",
        //           "type": "string"
        //         }
        //       },
        //       "required": [
        //         "account",
        //         "symbol"
        //       ],
        //       "title": "BalanceOfArgs",
        //       "type": "object"
        //     },
        //     "strict": true
        // }
    }
}
