use crate::{
    domain::value_objects::{ArticleId, CreatorId, Money, TenantId, TransactionId, WalletAddress},
    error::Result,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Blockchain network for the transaction
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Blockchain {
    /// Solana mainnet
    #[default]
    Solana,
    /// Base (Coinbase L2)
    Base,
    /// Polygon
    Polygon,
}

impl std::fmt::Display for Blockchain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Blockchain::Solana => write!(f, "solana"),
            Blockchain::Base => write!(f, "base"),
            Blockchain::Polygon => write!(f, "polygon"),
        }
    }
}

/// Transaction status
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransactionStatus {
    /// Transaction is pending verification
    #[default]
    Pending,
    /// Transaction has been verified on-chain
    Confirmed,
    /// Transaction failed verification
    Failed,
    /// Transaction has been refunded
    Refunded,
    /// Transaction is disputed
    Disputed,
}

/// Domain Entity: Transaction
///
/// Represents a payment transaction for article access in the paywall system.
/// Transactions are immutable once confirmed - any changes create new events.
///
/// Domain Rules:
/// - Transaction must have a valid blockchain signature
/// - Amount must be positive and match article price
/// - Reader wallet must be valid
/// - Only pending transactions can be confirmed or failed
/// - Only confirmed transactions can be refunded or disputed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    id: TransactionId,
    tenant_id: TenantId,
    article_id: ArticleId,
    creator_id: CreatorId,
    reader_wallet: WalletAddress,
    amount: Money,
    platform_fee: Money,
    creator_amount: Money,
    blockchain: Blockchain,
    tx_signature: String,
    status: TransactionStatus,
    created_at: DateTime<Utc>,
    confirmed_at: Option<DateTime<Utc>>,
    refunded_at: Option<DateTime<Utc>>,
    metadata: serde_json::Value,
}

impl Transaction {
    /// Create a new pending transaction
    pub fn new(
        tenant_id: TenantId,
        article_id: ArticleId,
        creator_id: CreatorId,
        reader_wallet: WalletAddress,
        amount: Money,
        platform_fee_percentage: u64,
        blockchain: Blockchain,
        tx_signature: String,
    ) -> Result<Self> {
        Self::validate_amount(&amount)?;
        Self::validate_signature(&tx_signature)?;

        let platform_fee = amount.percentage(platform_fee_percentage);
        let creator_amount = (amount - platform_fee)?;

        Ok(Self {
            id: TransactionId::new(),
            tenant_id,
            article_id,
            creator_id,
            reader_wallet,
            amount,
            platform_fee,
            creator_amount,
            blockchain,
            tx_signature,
            status: TransactionStatus::Pending,
            created_at: Utc::now(),
            confirmed_at: None,
            refunded_at: None,
            metadata: serde_json::json!({}),
        })
    }

    /// Reconstruct transaction from storage (bypasses validation)
    #[allow(clippy::too_many_arguments)]
    pub fn reconstruct(
        id: TransactionId,
        tenant_id: TenantId,
        article_id: ArticleId,
        creator_id: CreatorId,
        reader_wallet: WalletAddress,
        amount: Money,
        platform_fee: Money,
        creator_amount: Money,
        blockchain: Blockchain,
        tx_signature: String,
        status: TransactionStatus,
        created_at: DateTime<Utc>,
        confirmed_at: Option<DateTime<Utc>>,
        refunded_at: Option<DateTime<Utc>>,
        metadata: serde_json::Value,
    ) -> Self {
        Self {
            id,
            tenant_id,
            article_id,
            creator_id,
            reader_wallet,
            amount,
            platform_fee,
            creator_amount,
            blockchain,
            tx_signature,
            status,
            created_at,
            confirmed_at,
            refunded_at,
            metadata,
        }
    }

    // Getters

    pub fn id(&self) -> &TransactionId {
        &self.id
    }

    pub fn tenant_id(&self) -> &TenantId {
        &self.tenant_id
    }

    pub fn article_id(&self) -> &ArticleId {
        &self.article_id
    }

    pub fn creator_id(&self) -> &CreatorId {
        &self.creator_id
    }

    pub fn reader_wallet(&self) -> &WalletAddress {
        &self.reader_wallet
    }

    pub fn amount(&self) -> &Money {
        &self.amount
    }

    pub fn amount_cents(&self) -> u64 {
        self.amount.amount()
    }

    pub fn platform_fee(&self) -> &Money {
        &self.platform_fee
    }

    pub fn platform_fee_cents(&self) -> u64 {
        self.platform_fee.amount()
    }

    pub fn creator_amount(&self) -> &Money {
        &self.creator_amount
    }

    pub fn creator_amount_cents(&self) -> u64 {
        self.creator_amount.amount()
    }

    pub fn blockchain(&self) -> Blockchain {
        self.blockchain
    }

    pub fn tx_signature(&self) -> &str {
        &self.tx_signature
    }

    pub fn status(&self) -> TransactionStatus {
        self.status
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    pub fn confirmed_at(&self) -> Option<DateTime<Utc>> {
        self.confirmed_at
    }

    pub fn refunded_at(&self) -> Option<DateTime<Utc>> {
        self.refunded_at
    }

    pub fn metadata(&self) -> &serde_json::Value {
        &self.metadata
    }

    // Domain behavior methods

    /// Check if transaction is confirmed
    pub fn is_confirmed(&self) -> bool {
        self.status == TransactionStatus::Confirmed
    }

    /// Check if transaction is pending
    pub fn is_pending(&self) -> bool {
        self.status == TransactionStatus::Pending
    }

    /// Check if transaction can grant access (confirmed and not refunded)
    pub fn grants_access(&self) -> bool {
        self.status == TransactionStatus::Confirmed
    }

    /// Confirm the transaction (payment verified on-chain)
    pub fn confirm(&mut self) -> Result<()> {
        if self.status != TransactionStatus::Pending {
            return Err(crate::error::AllSourceError::ValidationError(format!(
                "Cannot confirm transaction with status {:?}",
                self.status
            )));
        }

        self.status = TransactionStatus::Confirmed;
        self.confirmed_at = Some(Utc::now());
        Ok(())
    }

    /// Mark transaction as failed
    pub fn fail(&mut self, reason: &str) -> Result<()> {
        if self.status != TransactionStatus::Pending {
            return Err(crate::error::AllSourceError::ValidationError(format!(
                "Cannot fail transaction with status {:?}",
                self.status
            )));
        }

        self.status = TransactionStatus::Failed;
        self.metadata["failure_reason"] = serde_json::json!(reason);
        Ok(())
    }

    /// Refund the transaction
    pub fn refund(&mut self, refund_tx_signature: &str) -> Result<()> {
        if self.status != TransactionStatus::Confirmed {
            return Err(crate::error::AllSourceError::ValidationError(
                "Can only refund confirmed transactions".to_string(),
            ));
        }

        self.status = TransactionStatus::Refunded;
        self.refunded_at = Some(Utc::now());
        self.metadata["refund_tx_signature"] = serde_json::json!(refund_tx_signature);
        Ok(())
    }

    /// Mark transaction as disputed
    pub fn dispute(&mut self, reason: &str) -> Result<()> {
        if self.status != TransactionStatus::Confirmed {
            return Err(crate::error::AllSourceError::ValidationError(
                "Can only dispute confirmed transactions".to_string(),
            ));
        }

        self.status = TransactionStatus::Disputed;
        self.metadata["dispute_reason"] = serde_json::json!(reason);
        self.metadata["disputed_at"] = serde_json::json!(Utc::now().to_rfc3339());
        Ok(())
    }

    /// Resolve a dispute (back to confirmed)
    pub fn resolve_dispute(&mut self, resolution: &str) -> Result<()> {
        if self.status != TransactionStatus::Disputed {
            return Err(crate::error::AllSourceError::ValidationError(
                "Can only resolve disputed transactions".to_string(),
            ));
        }

        self.status = TransactionStatus::Confirmed;
        self.metadata["dispute_resolution"] = serde_json::json!(resolution);
        self.metadata["resolved_at"] = serde_json::json!(Utc::now().to_rfc3339());
        Ok(())
    }

    /// Get explorer URL for the transaction
    pub fn explorer_url(&self) -> String {
        match self.blockchain {
            Blockchain::Solana => {
                format!("https://solscan.io/tx/{}", self.tx_signature)
            }
            Blockchain::Base => {
                format!("https://basescan.org/tx/{}", self.tx_signature)
            }
            Blockchain::Polygon => {
                format!("https://polygonscan.com/tx/{}", self.tx_signature)
            }
        }
    }

    // Validation

    fn validate_amount(amount: &Money) -> Result<()> {
        if amount.is_zero() {
            return Err(crate::error::AllSourceError::ValidationError(
                "Transaction amount must be positive".to_string(),
            ));
        }
        Ok(())
    }

    fn validate_signature(signature: &str) -> Result<()> {
        if signature.is_empty() {
            return Err(crate::error::AllSourceError::InvalidInput(
                "Transaction signature cannot be empty".to_string(),
            ));
        }

        // Solana signatures are 88 characters base58
        // Allow some flexibility for different blockchains
        if signature.len() < 40 || signature.len() > 128 {
            return Err(crate::error::AllSourceError::InvalidInput(format!(
                "Invalid transaction signature length: {}",
                signature.len()
            )));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::value_objects::Currency;

    const VALID_WALLET: &str = "9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM";
    const VALID_SIGNATURE: &str =
        "5eykt4UsFv8P8NJdTREpY1vzqKqZKvdpKuc147dw2N9g8eykt4UsFv8P8NJdTREpY1vzqKqZKvdpKuc147dw2N9g";

    fn test_tenant_id() -> TenantId {
        TenantId::new("test-tenant".to_string()).unwrap()
    }

    fn test_article_id() -> ArticleId {
        ArticleId::new("test-article".to_string()).unwrap()
    }

    fn test_creator_id() -> CreatorId {
        CreatorId::new()
    }

    fn test_wallet() -> WalletAddress {
        WalletAddress::new(VALID_WALLET.to_string()).unwrap()
    }

    #[test]
    fn test_create_transaction() {
        let transaction = Transaction::new(
            test_tenant_id(),
            test_article_id(),
            test_creator_id(),
            test_wallet(),
            Money::usd_cents(100), // $1.00
            7,                     // 7% fee
            Blockchain::Solana,
            VALID_SIGNATURE.to_string(),
        );

        assert!(transaction.is_ok());
        let transaction = transaction.unwrap();

        assert_eq!(transaction.amount_cents(), 100);
        assert_eq!(transaction.platform_fee_cents(), 7); // 7%
        assert_eq!(transaction.creator_amount_cents(), 93); // 100 - 7
        assert_eq!(transaction.status(), TransactionStatus::Pending);
        assert!(transaction.is_pending());
    }

    #[test]
    fn test_reject_zero_amount() {
        let result = Transaction::new(
            test_tenant_id(),
            test_article_id(),
            test_creator_id(),
            test_wallet(),
            Money::zero(Currency::USD),
            7,
            Blockchain::Solana,
            VALID_SIGNATURE.to_string(),
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_reject_empty_signature() {
        let result = Transaction::new(
            test_tenant_id(),
            test_article_id(),
            test_creator_id(),
            test_wallet(),
            Money::usd_cents(100),
            7,
            Blockchain::Solana,
            String::new(),
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_confirm_transaction() {
        let mut transaction = Transaction::new(
            test_tenant_id(),
            test_article_id(),
            test_creator_id(),
            test_wallet(),
            Money::usd_cents(100),
            7,
            Blockchain::Solana,
            VALID_SIGNATURE.to_string(),
        )
        .unwrap();

        assert!(transaction.is_pending());
        assert!(!transaction.grants_access());

        let result = transaction.confirm();
        assert!(result.is_ok());
        assert!(transaction.is_confirmed());
        assert!(transaction.grants_access());
        assert!(transaction.confirmed_at().is_some());
    }

    #[test]
    fn test_cannot_confirm_confirmed() {
        let mut transaction = Transaction::new(
            test_tenant_id(),
            test_article_id(),
            test_creator_id(),
            test_wallet(),
            Money::usd_cents(100),
            7,
            Blockchain::Solana,
            VALID_SIGNATURE.to_string(),
        )
        .unwrap();

        transaction.confirm().unwrap();
        let result = transaction.confirm();
        assert!(result.is_err());
    }

    #[test]
    fn test_fail_transaction() {
        let mut transaction = Transaction::new(
            test_tenant_id(),
            test_article_id(),
            test_creator_id(),
            test_wallet(),
            Money::usd_cents(100),
            7,
            Blockchain::Solana,
            VALID_SIGNATURE.to_string(),
        )
        .unwrap();

        let result = transaction.fail("Insufficient funds");
        assert!(result.is_ok());
        assert_eq!(transaction.status(), TransactionStatus::Failed);
    }

    #[test]
    fn test_refund_transaction() {
        let mut transaction = Transaction::new(
            test_tenant_id(),
            test_article_id(),
            test_creator_id(),
            test_wallet(),
            Money::usd_cents(100),
            7,
            Blockchain::Solana,
            VALID_SIGNATURE.to_string(),
        )
        .unwrap();

        // Must confirm first
        transaction.confirm().unwrap();

        let result =
            transaction.refund("refund_tx_sig_12345678901234567890123456789012345678901234");
        assert!(result.is_ok());
        assert_eq!(transaction.status(), TransactionStatus::Refunded);
        assert!(transaction.refunded_at().is_some());
        assert!(!transaction.grants_access());
    }

    #[test]
    fn test_cannot_refund_pending() {
        let mut transaction = Transaction::new(
            test_tenant_id(),
            test_article_id(),
            test_creator_id(),
            test_wallet(),
            Money::usd_cents(100),
            7,
            Blockchain::Solana,
            VALID_SIGNATURE.to_string(),
        )
        .unwrap();

        let result = transaction.refund("refund_sig");
        assert!(result.is_err());
    }

    #[test]
    fn test_dispute_transaction() {
        let mut transaction = Transaction::new(
            test_tenant_id(),
            test_article_id(),
            test_creator_id(),
            test_wallet(),
            Money::usd_cents(100),
            7,
            Blockchain::Solana,
            VALID_SIGNATURE.to_string(),
        )
        .unwrap();

        transaction.confirm().unwrap();
        let result = transaction.dispute("Content not delivered");
        assert!(result.is_ok());
        assert_eq!(transaction.status(), TransactionStatus::Disputed);
    }

    #[test]
    fn test_resolve_dispute() {
        let mut transaction = Transaction::new(
            test_tenant_id(),
            test_article_id(),
            test_creator_id(),
            test_wallet(),
            Money::usd_cents(100),
            7,
            Blockchain::Solana,
            VALID_SIGNATURE.to_string(),
        )
        .unwrap();

        transaction.confirm().unwrap();
        transaction.dispute("Content not delivered").unwrap();

        let result = transaction.resolve_dispute("Content was delivered, access restored");
        assert!(result.is_ok());
        assert_eq!(transaction.status(), TransactionStatus::Confirmed);
    }

    #[test]
    fn test_explorer_url() {
        let transaction = Transaction::new(
            test_tenant_id(),
            test_article_id(),
            test_creator_id(),
            test_wallet(),
            Money::usd_cents(100),
            7,
            Blockchain::Solana,
            VALID_SIGNATURE.to_string(),
        )
        .unwrap();

        let url = transaction.explorer_url();
        assert!(url.contains("solscan.io"));
        assert!(url.contains(VALID_SIGNATURE));
    }

    #[test]
    fn test_serde_serialization() {
        let transaction = Transaction::new(
            test_tenant_id(),
            test_article_id(),
            test_creator_id(),
            test_wallet(),
            Money::usd_cents(100),
            7,
            Blockchain::Solana,
            VALID_SIGNATURE.to_string(),
        )
        .unwrap();

        let json = serde_json::to_string(&transaction);
        assert!(json.is_ok());

        let deserialized: Transaction = serde_json::from_str(&json.unwrap()).unwrap();
        assert_eq!(deserialized.amount_cents(), 100);
    }
}
