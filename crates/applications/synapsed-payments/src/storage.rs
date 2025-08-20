use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::error::{PaymentError, PaymentResult};
use crate::processor::PaymentStorage;
use crate::types::{Customer, PaymentIntent, PaymentMethod, PaymentStatus, Refund, Transaction};

/// In-memory payment storage implementation for development/testing
#[derive(Debug)]
pub struct MemoryPaymentStorage {
    payments: Arc<RwLock<HashMap<Uuid, PaymentIntent>>>,
    transactions: Arc<RwLock<HashMap<Uuid, Vec<Transaction>>>>,
    refunds: Arc<RwLock<HashMap<Uuid, Refund>>>,
    customers: Arc<RwLock<HashMap<String, Customer>>>,
}

impl MemoryPaymentStorage {
    /// Create a new memory storage instance
    pub fn new() -> Self {
        Self {
            payments: Arc::new(RwLock::new(HashMap::new())),
            transactions: Arc::new(RwLock::new(HashMap::new())),
            refunds: Arc::new(RwLock::new(HashMap::new())),
            customers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Add a customer for testing
    pub async fn add_customer(&self, customer: Customer) {
        let mut customers = self.customers.write().await;
        customers.insert(customer.id.clone(), customer);
    }

    /// Get payment count
    pub async fn get_payment_count(&self) -> usize {
        let payments = self.payments.read().await;
        payments.len()
    }

    /// Get transaction count
    pub async fn get_transaction_count(&self) -> usize {
        let transactions = self.transactions.read().await;
        transactions.values().map(|v| v.len()).sum()
    }

    /// Clear all data
    pub async fn clear(&self) {
        let mut payments = self.payments.write().await;
        let mut transactions = self.transactions.write().await;
        let mut refunds = self.refunds.write().await;
        let mut customers = self.customers.write().await;

        payments.clear();
        transactions.clear();
        refunds.clear();
        customers.clear();
    }
}

#[async_trait]
impl PaymentStorage for MemoryPaymentStorage {
    async fn store_payment(&self, payment: &PaymentIntent) -> PaymentResult<()> {
        let mut payments = self.payments.write().await;
        payments.insert(payment.id, payment.clone());
        Ok(())
    }

    async fn get_payment(&self, payment_id: Uuid) -> PaymentResult<PaymentIntent> {
        let payments = self.payments.read().await;
        payments
            .get(&payment_id)
            .cloned()
            .ok_or_else(|| PaymentError::PaymentNotFound {
                payment_id: payment_id.to_string(),
            })
    }

    async fn update_payment_status(
        &self,
        payment_id: Uuid,
        status: PaymentStatus,
    ) -> PaymentResult<()> {
        let mut payments = self.payments.write().await;
        if let Some(payment) = payments.get_mut(&payment_id) {
            payment.status = status;
            Ok(())
        } else {
            Err(PaymentError::PaymentNotFound {
                payment_id: payment_id.to_string(),
            })
        }
    }

    async fn store_transaction(&self, transaction: &Transaction) -> PaymentResult<()> {
        let mut transactions = self.transactions.write().await;
        transactions
            .entry(transaction.payment_id)
            .or_insert_with(Vec::new)
            .push(transaction.clone());
        Ok(())
    }

    async fn get_payment_transactions(&self, payment_id: Uuid) -> PaymentResult<Vec<Transaction>> {
        let transactions = self.transactions.read().await;
        Ok(transactions
            .get(&payment_id)
            .cloned()
            .unwrap_or_default())
    }

    async fn store_refund(&self, refund: &Refund) -> PaymentResult<()> {
        let mut refunds = self.refunds.write().await;
        refunds.insert(refund.id, refund.clone());
        Ok(())
    }

    async fn get_customer(&self, customer_id: &str) -> PaymentResult<Option<Customer>> {
        let customers = self.customers.read().await;
        Ok(customers.get(customer_id).cloned())
    }
}

impl Default for MemoryPaymentStorage {
    fn default() -> Self {
        Self::new()
    }
}

/// SQLite-based payment storage implementation
#[cfg(feature = "sqlite")]
pub struct SqlitePaymentStorage {
    pool: sqlx::SqlitePool,
}

#[cfg(feature = "sqlite")]
impl SqlitePaymentStorage {
    /// Create a new SQLite storage instance
    pub async fn new(database_url: &str) -> PaymentResult<Self> {
        let pool = sqlx::SqlitePool::connect(database_url).await?;
        
        // Run migrations
        sqlx::migrate!("./migrations").run(&pool).await?;
        
        Ok(Self { pool })
    }

    /// Create an in-memory SQLite instance for testing
    pub async fn new_in_memory() -> PaymentResult<Self> {
        Self::new(":memory:").await
    }
}

#[cfg(feature = "sqlite")]
#[async_trait]
impl PaymentStorage for SqlitePaymentStorage {
    async fn store_payment(&self, payment: &PaymentIntent) -> PaymentResult<()> {
        let payment_method_json = payment.payment_method
            .as_ref()
            .map(|pm| serde_json::to_string(pm))
            .transpose()?;
        
        let metadata_json = serde_json::to_string(&payment.metadata)?;

        sqlx::query!(
            r#"
            INSERT INTO payments (
                id, amount_value, amount_currency, description, customer_id,
                metadata, created_at, expires_at, status, payment_method,
                confirmation_method
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            payment.id,
            payment.amount.value,
            payment.amount.currency.to_string(),
            payment.description,
            payment.customer_id,
            metadata_json,
            payment.created_at,
            payment.expires_at,
            payment.status as i32,
            payment_method_json,
            payment.confirmation_method as i32
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn get_payment(&self, payment_id: Uuid) -> PaymentResult<PaymentIntent> {
        let row = sqlx::query!(
            "SELECT * FROM payments WHERE id = ?",
            payment_id
        )
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| PaymentError::PaymentNotFound {
            payment_id: payment_id.to_string(),
        })?;

        // Reconstruct payment intent from database row
        // This would involve proper deserialization of JSON fields
        // For brevity, this is a simplified version
        todo!("Implement full payment reconstruction from SQLite row")
    }

    async fn update_payment_status(
        &self,
        payment_id: Uuid,
        status: PaymentStatus,
    ) -> PaymentResult<()> {
        let result = sqlx::query!(
            "UPDATE payments SET status = ? WHERE id = ?",
            status as i32,
            payment_id
        )
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(PaymentError::PaymentNotFound {
                payment_id: payment_id.to_string(),
            });
        }

        Ok(())
    }

    async fn store_transaction(&self, transaction: &Transaction) -> PaymentResult<()> {
        let gateway_response_json = transaction.gateway_response
            .as_ref()
            .map(|gr| serde_json::to_string(gr))
            .transpose()?;
        
        let metadata_json = serde_json::to_string(&transaction.metadata)?;

        sqlx::query!(
            r#"
            INSERT INTO transactions (
                id, payment_id, transaction_type, amount_value, amount_currency,
                status, gateway_transaction_id, gateway_response, created_at,
                processed_at, metadata
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            transaction.id,
            transaction.payment_id,
            transaction.transaction_type as i32,
            transaction.amount.value,
            transaction.amount.currency.to_string(),
            transaction.status as i32,
            transaction.gateway_transaction_id,
            gateway_response_json,
            transaction.created_at,
            transaction.processed_at,
            metadata_json
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn get_payment_transactions(&self, payment_id: Uuid) -> PaymentResult<Vec<Transaction>> {
        let rows = sqlx::query!(
            "SELECT * FROM transactions WHERE payment_id = ? ORDER BY created_at",
            payment_id
        )
        .fetch_all(&self.pool)
        .await?;

        // Convert rows to Transaction objects
        // This would involve proper deserialization
        // For brevity, this is simplified
        todo!("Implement transaction reconstruction from SQLite rows")
    }

    async fn store_refund(&self, refund: &Refund) -> PaymentResult<()> {
        let metadata_json = serde_json::to_string(&refund.metadata)?;

        sqlx::query!(
            r#"
            INSERT INTO refunds (
                id, payment_id, amount_value, amount_currency, reason,
                status, created_at, processed_at, gateway_refund_id, metadata
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            refund.id,
            refund.payment_id,
            refund.amount.value,
            refund.amount.currency.to_string(),
            refund.reason,
            refund.status as i32,
            refund.created_at,
            refund.processed_at,
            refund.gateway_refund_id,
            metadata_json
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn get_customer(&self, customer_id: &str) -> PaymentResult<Option<Customer>> {
        let row = sqlx::query!(
            "SELECT * FROM customers WHERE id = ?",
            customer_id
        )
        .fetch_optional(&self.pool)
        .await?;

        if let Some(_row) = row {
            // Convert row to Customer object
            // This would involve proper deserialization
            todo!("Implement customer reconstruction from SQLite row")
        } else {
            Ok(None)
        }
    }
}

/// PostgreSQL-based payment storage implementation
#[cfg(feature = "postgres")]
pub struct PostgresPaymentStorage {
    pool: sqlx::PgPool,
}

#[cfg(feature = "postgres")]
impl PostgresPaymentStorage {
    /// Create a new PostgreSQL storage instance
    pub async fn new(database_url: &str) -> PaymentResult<Self> {
        let pool = sqlx::PgPool::connect(database_url).await?;
        
        // Run migrations
        sqlx::migrate!("./migrations").run(&pool).await?;
        
        Ok(Self { pool })
    }
}

#[cfg(feature = "postgres")]
#[async_trait]
impl PaymentStorage for PostgresPaymentStorage {
    async fn store_payment(&self, payment: &PaymentIntent) -> PaymentResult<()> {
        let payment_method_json = payment.payment_method
            .as_ref()
            .map(|pm| serde_json::to_value(pm))
            .transpose()?;
        
        let metadata_json = serde_json::to_value(&payment.metadata)?;

        sqlx::query!(
            r#"
            INSERT INTO payments (
                id, amount_value, amount_currency, description, customer_id,
                metadata, created_at, expires_at, status, payment_method,
                confirmation_method
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            "#,
            payment.id,
            payment.amount.value,
            payment.amount.currency.to_string(),
            payment.description,
            payment.customer_id,
            metadata_json,
            payment.created_at,
            payment.expires_at,
            payment.status as i32,
            payment_method_json,
            payment.confirmation_method as i32
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn get_payment(&self, payment_id: Uuid) -> PaymentResult<PaymentIntent> {
        // Similar to SQLite implementation but with PostgreSQL-specific queries
        todo!("Implement PostgreSQL payment retrieval")
    }

    async fn update_payment_status(
        &self,
        payment_id: Uuid,
        status: PaymentStatus,
    ) -> PaymentResult<()> {
        let result = sqlx::query!(
            "UPDATE payments SET status = $1 WHERE id = $2",
            status as i32,
            payment_id
        )
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(PaymentError::PaymentNotFound {
                payment_id: payment_id.to_string(),
            });
        }

        Ok(())
    }

    async fn store_transaction(&self, _transaction: &Transaction) -> PaymentResult<()> {
        todo!("Implement PostgreSQL transaction storage")
    }

    async fn get_payment_transactions(&self, _payment_id: Uuid) -> PaymentResult<Vec<Transaction>> {
        todo!("Implement PostgreSQL transaction retrieval")
    }

    async fn store_refund(&self, _refund: &Refund) -> PaymentResult<()> {
        todo!("Implement PostgreSQL refund storage")
    }

    async fn get_customer(&self, _customer_id: &str) -> PaymentResult<Option<Customer>> {
        todo!("Implement PostgreSQL customer retrieval")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Amount, Currency, FiatCurrency, PaymentMethod};
    use chrono::Utc;
    use rust_decimal::Decimal;
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_memory_storage_payment_lifecycle() {
        let storage = MemoryPaymentStorage::new();

        // Create and store a payment
        let amount = Amount::new(Decimal::new(10000, 2), Currency::Fiat(FiatCurrency::USD)).expect("Failed to create amount");
        let payment = PaymentIntent::new(amount, "Test payment".to_string());
        let payment_id = payment.id;

        // Store payment
        storage.store_payment(&payment).await.unwrap();

        // Retrieve payment
        let retrieved = storage.get_payment(payment_id).await.unwrap();
        assert_eq!(retrieved.id, payment_id);
        assert_eq!(retrieved.description, "Test payment");

        // Update status
        storage
            .update_payment_status(payment_id, PaymentStatus::Completed)
            .await
            .unwrap();

        let updated = storage.get_payment(payment_id).await.unwrap();
        assert_eq!(updated.status, PaymentStatus::Completed);
    }

    #[tokio::test]
    async fn test_memory_storage_transactions() {
        let storage = MemoryPaymentStorage::new();

        let payment_id = Uuid::new_v4();
        let amount = Amount::new(Decimal::new(10000, 2), Currency::Fiat(FiatCurrency::USD)).expect("Failed to create amount");

        // Create and store transactions
        let payment_method = PaymentMethod::CreditCard {
            last_four: "4242".to_string(),
            brand: "Visa".to_string(),
            exp_month: 12,
            exp_year: 2025,
            holder_name: "Test User".to_string(),
        };
        
        let mut tx1 = Transaction::new(
            amount.clone(),
            payment_method.clone(),
            "user_123".to_string(),
            "Test payment".to_string(),
        );
        tx1.payment_id = payment_id; // Set the payment_id to match what we're searching for
        
        let mut tx2 = Transaction::new(
            amount,
            payment_method,
            "user_123".to_string(),
            "Test refund".to_string(),
        );
        tx2.payment_id = payment_id; // Set the payment_id to match what we're searching for

        storage.store_transaction(&tx1).await.unwrap();
        storage.store_transaction(&tx2).await.unwrap();

        // Retrieve transactions
        let transactions = storage.get_payment_transactions(payment_id).await.unwrap();
        assert_eq!(transactions.len(), 2);
    }

    #[tokio::test]
    async fn test_memory_storage_customers() {
        let storage = MemoryPaymentStorage::new();

        // Add customer
        let customer = Customer {
            id: "cust_123".to_string(),
            email: "test@example.com".to_string(),
            name: Some("Test Customer".to_string()),
            phone: None,
            billing_address: None,
            shipping_address: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            metadata: HashMap::new(),
        };

        storage.add_customer(customer.clone()).await;

        // Retrieve customer
        let retrieved = storage.get_customer("cust_123").await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().email, "test@example.com".to_string());

        // Try non-existent customer
        let not_found = storage.get_customer("cust_999").await.unwrap();
        assert!(not_found.is_none());
    }

    #[tokio::test]
    async fn test_memory_storage_clear() {
        let storage = MemoryPaymentStorage::new();

        // Add some data
        let amount = Amount::new(Decimal::new(10000, 2), Currency::Fiat(FiatCurrency::USD)).expect("Failed to create amount");
        let payment = PaymentIntent::new(amount, "Test payment".to_string());
        storage.store_payment(&payment).await.unwrap();

        assert_eq!(storage.get_payment_count().await, 1);

        // Clear all data
        storage.clear().await;
        assert_eq!(storage.get_payment_count().await, 0);
    }

    #[tokio::test]
    async fn test_payment_not_found() {
        let storage = MemoryPaymentStorage::new();
        let result = storage.get_payment(Uuid::new_v4()).await;
        
        assert!(result.is_err());
        match result.unwrap_err() {
            PaymentError::PaymentNotFound { .. } => {}, // Expected
            _ => panic!("Expected PaymentNotFound error"),
        }
    }
}