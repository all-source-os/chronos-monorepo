use crate::domain::entities::{Blockchain, Transaction, TransactionStatus};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// DTO for initiating a payment transaction
#[derive(Debug, Deserialize)]
pub struct InitiatePaymentRequest {
    pub tenant_id: String,
    pub article_id: String,
    pub reader_wallet: String,
    pub tx_signature: String,
    pub blockchain: Option<BlockchainDto>,
}

/// DTO for payment initiation response
#[derive(Debug, Serialize)]
pub struct InitiatePaymentResponse {
    pub transaction: TransactionDto,
}

/// DTO for confirming a transaction
#[derive(Debug, Deserialize)]
pub struct ConfirmTransactionRequest {
    pub transaction_id: String,
}

/// DTO for transaction confirmation response
#[derive(Debug, Serialize)]
pub struct ConfirmTransactionResponse {
    pub transaction: TransactionDto,
    pub access_token: Option<String>,
}

/// DTO for refunding a transaction
#[derive(Debug, Deserialize)]
pub struct RefundTransactionRequest {
    pub transaction_id: String,
    pub refund_tx_signature: String,
    pub reason: Option<String>,
}

/// DTO for refund response
#[derive(Debug, Serialize)]
pub struct RefundTransactionResponse {
    pub transaction: TransactionDto,
}

/// DTO for listing transactions response
#[derive(Debug, Serialize)]
pub struct ListTransactionsResponse {
    pub transactions: Vec<TransactionDto>,
    pub count: usize,
}

/// DTO for blockchain type
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BlockchainDto {
    Solana,
    Base,
    Polygon,
}

impl Default for BlockchainDto {
    fn default() -> Self {
        Self::Solana
    }
}

impl From<Blockchain> for BlockchainDto {
    fn from(blockchain: Blockchain) -> Self {
        match blockchain {
            Blockchain::Solana => BlockchainDto::Solana,
            Blockchain::Base => BlockchainDto::Base,
            Blockchain::Polygon => BlockchainDto::Polygon,
        }
    }
}

impl From<BlockchainDto> for Blockchain {
    fn from(dto: BlockchainDto) -> Self {
        match dto {
            BlockchainDto::Solana => Blockchain::Solana,
            BlockchainDto::Base => Blockchain::Base,
            BlockchainDto::Polygon => Blockchain::Polygon,
        }
    }
}

/// DTO for transaction status
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TransactionStatusDto {
    Pending,
    Confirmed,
    Failed,
    Refunded,
    Disputed,
}

impl From<TransactionStatus> for TransactionStatusDto {
    fn from(status: TransactionStatus) -> Self {
        match status {
            TransactionStatus::Pending => TransactionStatusDto::Pending,
            TransactionStatus::Confirmed => TransactionStatusDto::Confirmed,
            TransactionStatus::Failed => TransactionStatusDto::Failed,
            TransactionStatus::Refunded => TransactionStatusDto::Refunded,
            TransactionStatus::Disputed => TransactionStatusDto::Disputed,
        }
    }
}

/// DTO for a transaction in responses
#[derive(Debug, Clone, Serialize)]
pub struct TransactionDto {
    pub id: Uuid,
    pub tenant_id: String,
    pub article_id: String,
    pub creator_id: String,
    pub reader_wallet: String,
    pub amount_cents: u64,
    pub platform_fee_cents: u64,
    pub creator_amount_cents: u64,
    pub blockchain: BlockchainDto,
    pub tx_signature: String,
    pub status: TransactionStatusDto,
    pub explorer_url: String,
    pub created_at: DateTime<Utc>,
    pub confirmed_at: Option<DateTime<Utc>>,
    pub refunded_at: Option<DateTime<Utc>>,
}

impl From<&Transaction> for TransactionDto {
    fn from(tx: &Transaction) -> Self {
        Self {
            id: tx.id().as_uuid(),
            tenant_id: tx.tenant_id().to_string(),
            article_id: tx.article_id().to_string(),
            creator_id: tx.creator_id().as_uuid().to_string(),
            reader_wallet: tx.reader_wallet().to_string(),
            amount_cents: tx.amount_cents(),
            platform_fee_cents: tx.platform_fee_cents(),
            creator_amount_cents: tx.creator_amount_cents(),
            blockchain: tx.blockchain().into(),
            tx_signature: tx.tx_signature().to_string(),
            status: tx.status().into(),
            explorer_url: tx.explorer_url(),
            created_at: tx.created_at(),
            confirmed_at: tx.confirmed_at(),
            refunded_at: tx.refunded_at(),
        }
    }
}

impl From<Transaction> for TransactionDto {
    fn from(tx: Transaction) -> Self {
        TransactionDto::from(&tx)
    }
}

/// DTO for revenue summary
#[derive(Debug, Serialize)]
pub struct RevenueSummaryDto {
    pub total_revenue_cents: u64,
    pub total_transactions: u64,
    pub total_platform_fees_cents: u64,
    pub total_creator_earnings_cents: u64,
}
