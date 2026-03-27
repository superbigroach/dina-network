use dina_core::error::DinaError;

use crate::gas::GasCosts;

/// A record of a single gas-consuming operation for diagnostics.
#[derive(Debug, Clone)]
pub struct GasOperation {
    /// Name of the operation (e.g. "storage_write", "sha256").
    pub operation: String,
    /// Gas cost charged for this operation.
    pub gas_cost: u64,
    /// Timestamp (block time or monotonic counter) when the operation occurred.
    pub timestamp: u64,
}

/// Enhanced gas meter that tracks per-operation costs and supports refunds.
///
/// Wraps the basic fuel-based metering with a detailed operation log so that
/// contract authors and node operators can see exactly where gas is spent.
#[derive(Debug)]
pub struct GasMeter {
    limit: u64,
    used: u64,
    costs: GasCosts,
    operation_log: Vec<GasOperation>,
}

impl GasMeter {
    /// Create a new gas meter with the given limit and cost table.
    pub fn new(limit: u64, costs: GasCosts) -> Self {
        Self {
            limit,
            used: 0,
            costs,
            operation_log: Vec::new(),
        }
    }

    /// Consume gas for the named operation.
    ///
    /// Returns an error if the meter is exhausted (used + amount > limit).
    pub fn consume(&mut self, operation: &str, amount: u64) -> Result<(), DinaError> {
        let new_used = self.used.checked_add(amount).unwrap_or(u64::MAX);
        if new_used > self.limit {
            return Err(DinaError::WasmExecutionError(format!(
                "out of gas: operation '{}' requires {} gas, only {} remaining",
                operation,
                amount,
                self.remaining()
            )));
        }

        self.used = new_used;
        self.operation_log.push(GasOperation {
            operation: operation.to_string(),
            gas_cost: amount,
            timestamp: self.operation_log.len() as u64,
        });

        Ok(())
    }

    /// Gas remaining before the limit is reached.
    pub fn remaining(&self) -> u64 {
        self.limit.saturating_sub(self.used)
    }

    /// Total gas consumed so far.
    pub fn used(&self) -> u64 {
        self.used
    }

    /// Whether the meter has been fully exhausted.
    pub fn is_exhausted(&self) -> bool {
        self.used >= self.limit
    }

    /// Refund unused pre-charged gas.
    ///
    /// Refunds cannot exceed gas already consumed; excess refunds are clamped
    /// to zero used.
    pub fn refund(&mut self, amount: u64) {
        self.used = self.used.saturating_sub(amount);
    }

    /// The full log of gas-consuming operations.
    pub fn operation_log(&self) -> &[GasOperation] {
        &self.operation_log
    }

    /// The single most expensive operation recorded so far.
    pub fn most_expensive_operation(&self) -> Option<&GasOperation> {
        self.operation_log.iter().max_by_key(|op| op.gas_cost)
    }

    /// Reference to the cost table used by this meter.
    pub fn costs(&self) -> &GasCosts {
        &self.costs
    }

    /// Consume gas for a storage read operation using the cost table.
    pub fn charge_storage_read(&mut self) -> Result<(), DinaError> {
        self.consume("storage_read", self.costs.storage_read)
    }

    /// Consume gas for a storage write operation using the cost table.
    pub fn charge_storage_write(&mut self) -> Result<(), DinaError> {
        self.consume("storage_write", self.costs.storage_write)
    }

    /// Consume gas for a transfer operation using the cost table.
    pub fn charge_transfer(&mut self) -> Result<(), DinaError> {
        self.consume("transfer", self.costs.transfer)
    }

    /// Consume gas for a cross-contract call using the cost table.
    pub fn charge_cross_contract_call(&mut self) -> Result<(), DinaError> {
        self.consume("cross_contract_call", self.costs.cross_contract_call)
    }

    /// Consume gas for an event emission using the cost table.
    pub fn charge_emit_event(&mut self) -> Result<(), DinaError> {
        self.consume("emit_event", self.costs.emit_event)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_meter(limit: u64) -> GasMeter {
        GasMeter::new(limit, GasCosts::default())
    }

    #[test]
    fn new_meter_full_remaining() {
        let meter = default_meter(1000);
        assert_eq!(meter.remaining(), 1000);
        assert_eq!(meter.used(), 0);
        assert!(!meter.is_exhausted());
    }

    #[test]
    fn consume_deducts_gas() {
        let mut meter = default_meter(1000);
        meter.consume("test_op", 300).unwrap();
        assert_eq!(meter.used(), 300);
        assert_eq!(meter.remaining(), 700);
    }

    #[test]
    fn consume_fails_when_exhausted() {
        let mut meter = default_meter(100);
        meter.consume("first", 80).unwrap();
        let result = meter.consume("second", 50);
        assert!(result.is_err());
        // used should not change on failure
        assert_eq!(meter.used(), 80);
    }

    #[test]
    fn is_exhausted_at_limit() {
        let mut meter = default_meter(100);
        meter.consume("fill", 100).unwrap();
        assert!(meter.is_exhausted());
        assert_eq!(meter.remaining(), 0);
    }

    #[test]
    fn refund_returns_gas() {
        let mut meter = default_meter(1000);
        meter.consume("op", 500).unwrap();
        meter.refund(200);
        assert_eq!(meter.used(), 300);
        assert_eq!(meter.remaining(), 700);
    }

    #[test]
    fn refund_clamped_to_zero() {
        let mut meter = default_meter(1000);
        meter.consume("op", 100).unwrap();
        meter.refund(500); // more than used
        assert_eq!(meter.used(), 0);
        assert_eq!(meter.remaining(), 1000);
    }

    #[test]
    fn operation_log_records_operations() {
        let mut meter = default_meter(10000);
        meter.consume("sha256", 50).unwrap();
        meter.consume("storage_write", 500).unwrap();
        meter.consume("transfer", 200).unwrap();

        let log = meter.operation_log();
        assert_eq!(log.len(), 3);
        assert_eq!(log[0].operation, "sha256");
        assert_eq!(log[0].gas_cost, 50);
        assert_eq!(log[1].operation, "storage_write");
        assert_eq!(log[2].operation, "transfer");
    }

    #[test]
    fn most_expensive_operation_finds_max() {
        let mut meter = default_meter(10000);
        meter.consume("cheap", 10).unwrap();
        meter.consume("expensive", 5000).unwrap();
        meter.consume("medium", 200).unwrap();

        let most = meter.most_expensive_operation().unwrap();
        assert_eq!(most.operation, "expensive");
        assert_eq!(most.gas_cost, 5000);
    }

    #[test]
    fn most_expensive_empty_returns_none() {
        let meter = default_meter(1000);
        assert!(meter.most_expensive_operation().is_none());
    }

    #[test]
    fn charge_helpers_use_cost_table() {
        let mut meter = default_meter(100_000);
        meter.charge_storage_read().unwrap();
        meter.charge_storage_write().unwrap();
        meter.charge_transfer().unwrap();
        meter.charge_cross_contract_call().unwrap();
        meter.charge_emit_event().unwrap();

        let costs = GasCosts::default();
        let expected = costs.storage_read
            + costs.storage_write
            + costs.transfer
            + costs.cross_contract_call
            + costs.emit_event;
        assert_eq!(meter.used(), expected);

        let log = meter.operation_log();
        assert_eq!(log.len(), 5);
        assert_eq!(log[0].operation, "storage_read");
        assert_eq!(log[0].gas_cost, costs.storage_read);
    }

    #[test]
    fn consume_exact_limit_succeeds() {
        let mut meter = default_meter(100);
        meter.consume("exact", 100).unwrap();
        assert!(meter.is_exhausted());
        assert_eq!(meter.remaining(), 0);
    }

    #[test]
    fn multiple_refunds_work() {
        let mut meter = default_meter(1000);
        meter.consume("a", 400).unwrap();
        meter.consume("b", 300).unwrap();
        meter.refund(100);
        meter.refund(100);
        assert_eq!(meter.used(), 500);
        assert_eq!(meter.remaining(), 500);
    }
}
