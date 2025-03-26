//! Module for interacting with ICP Ledgers using ICRC-1 standard
//!
//! This module provides functionality for:
//! - Loading and managing multiple ICP ledger canisters
//! - Transferring tokens between accounts
//! - Querying account balances
//!
//! The implementation supports:
//! - Multiple token symbols (though primarily designed for ICP)
//! - Configurable subaccount usage for transfers
//! - ICRC-1 standard compliant operations
//!
//! # Examples
//! ```rust,ignore
//! use anda_bsc::ledger::BSCLedgers;
//! use anda_core::CanisterCaller;
//! use std::collections::BTreeSet;
//!
//! async fn example(ctx: &impl CanisterCaller) {
//!     let canisters = BTreeSet::from([Principal::from_text("ryjl3-tyaaa-aaaaa-aaaba-cai").unwrap()]);
//!     let ledgers = BSCLedgers::load(ctx, canisters, false).await.unwrap();
//!     // Use ledgers for transfers or balance queries
//! }
//! ```

use anda_core::BoxError;
use std::{collections::{BTreeMap, BTreeSet}, str::FromStr};

pub mod balance;
pub mod transfer;

pub use balance::*;
pub use transfer::*;

use alloy::{
    primitives::{Address, U256}, 
    providers::{
        fillers::TxFiller, Network, Provider, ProviderLayer, RootProvider},
        sol
};
use eyre::Result;

// Codegen from artifact.
sol!(
    #[allow(missing_docs)]
    #[sol(rpc)]
    ERC20STD,
    "artifacts/ERC20Example.json"
);


/// BSC Ledger Transfer tool implementation
#[derive(Debug, Clone)]
pub struct BSCLedgers {
    /// Map of token symbols to their corresponding canister ID and decimals places
    pub ledgers: BTreeMap<String, (Address, u8)>,
}

impl BSCLedgers {
    /// Creates a new BSCLedgerTransfer instance
    ///
    /// # Arguments
    /// * `ctx` - Canister caller context
    /// * `ledger_canisters` - Set of 1 to N BSC token ledger canister IDs
    /// * `from_user_subaccount` - When false, the from account is the Agent's main account.
    ///   When true, the from account is a user-specific subaccount derived from the Agent's main account.
    pub async fn load<L, N, F>(
        ctx: &F::Provider,  //  TODO: is DynProvider appicable?
        addresses: BTreeSet<Address>,
    ) -> Result<BSCLedgers, BoxError>
        where
            L: ProviderLayer<RootProvider<N>, N>,
            F: TxFiller<N> + ProviderLayer<L::Provider, N>,
            N: Network, 
    {
        if addresses.is_empty() {
            return Err("No BSC ledger canister specified".into()); // TODO: is it token address?
        }
        let mut ledgers: BTreeMap<String, (Address, u8)> = BTreeMap::new();
        for address in addresses {
            let contract = ERC20STD::new(address, ctx);
            let symbol = contract.symbol().call().await?._0;
            let decimals = contract.decimals().call().await?._0;
            ledgers.insert(symbol, (address, decimals));
        }

        Ok(BSCLedgers {
            ledgers
        })
    }

    /// Performs the token transfer operation
    ///
    /// # Arguments
    /// * `ctx` - Canister caller context
    /// * `args` - Transfer arguments containing destination account, amount, and memo
    ///
    /// # Returns
    /// Result containing the ledger ID and transaction ID (Nat) or an error
    async fn transfer(
        &self,
        ctx: &impl Provider,  
        me: Address,
        args: transfer::TransferToArgs,
    ) -> Result<(Address, String), BoxError> {
        let to_addr = Address::from_str(&args.account)?;  
        let to_amount = U256::from_str(&args.amount)?;

        let (token_addr, _decimals) = self
            .ledgers
            .get(&args.symbol)
            .ok_or_else(|| format!("Token {} is not supported", args.symbol))?;

        let contract = ERC20STD::new(*token_addr, ctx);

        let balance = contract
            .balanceOf(me)
            .call()
            .await?._0;

        if balance < to_amount {
            return Err("insufficient balance".into());
        }

        let res = contract.transfer(to_addr, to_amount).send().await?.watch().await;

        log::info!(
            account = args.account,
            symbol = args.symbol,
            amount = args.amount,
            result = res.is_ok();
            "{}", TransferTool::NAME,
        );

        let tx_hash = match res {
            Ok(tx_hash) => tx_hash,
            Err(err) => {
                return Err(format!("failed to transfer tokens, error: {:?}", err).into())
            }
        };

        println!("Token transfer transaction: {:#?}", tx_hash);

        // return the token address and amount transferred
        Ok((*token_addr, args.amount))   
    }

    /// Retrieves the balance of a specific account for a given token
    ///
    /// # Arguments
    /// * `ctx` - Canister caller context
    /// * `args` - Balance query arguments containing account and token symbol
    ///
    /// # Returns
    /// Result containing the ledger ID and token balance (f64) or an error
    async fn balance_of(
        &self,
        ctx: &impl Provider,
        args: balance::BalanceOfArgs,
    ) -> Result<(Address, String), BoxError>
    {
        let user_addr = Address::from_str(&args.account)?;  

        let (token_addr, _decimals) = self
            .ledgers
            .get(&args.symbol)
            .ok_or_else(|| format!("Token {} is not supported", args.symbol))?;

        let contract = ERC20STD::new(*token_addr, ctx);

        let balance = contract
            .balanceOf(user_addr)
            .call()
            .await?._0;

        let balance = balance.to_string();
        log::info!(
            account = args.account,
            symbol = args.symbol,
            balance = balance;
            "{}", BalanceOfTool::NAME,
        );
        Ok((user_addr, balance))
    }
}
