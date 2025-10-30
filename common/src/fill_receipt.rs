//! Fill receipt - written by slab/AMM for router to read

/// Fill receipt - per-transaction fill summary
/// Router provides an account for the slab/AMM to write this
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FillReceipt {
    /// Used flag (1 if written)
    pub used: u32,
    /// Header.seqno at time of commit
    pub seqno_committed: u32,
    /// Filled quantity (signed: +buy, -sell, 1e6 scale)
    pub filled_qty: i64,
    /// Volume-weighted average price (1e6 scale)
    pub vwap_px: i64,
    /// Notional value: abs(filled_qty) * contract_size * vwap_px / 1e6
    pub notional: i64,
    /// Fee charged (1e6 scale)
    pub fee: i64,
    /// Realized PnL delta (optional in v0)
    pub pnl_delta: i64,
}

impl FillReceipt {
    pub const LEN: usize = core::mem::size_of::<Self>();

    /// Create empty receipt
    pub fn new() -> Self {
        Self {
            used: 0,
            seqno_committed: 0,
            filled_qty: 0,
            vwap_px: 0,
            notional: 0,
            fee: 0,
            pnl_delta: 0,
        }
    }

    /// Mark as used with fill data
    pub fn write(
        &mut self,
        seqno: u32,
        filled_qty: i64,
        vwap_px: i64,
        notional: i64,
        fee: i64,
    ) {
        self.used = 1;
        self.seqno_committed = seqno;
        self.filled_qty = filled_qty;
        self.vwap_px = vwap_px;
        self.notional = notional;
        self.fee = fee;
        self.pnl_delta = 0; // Not calculated in v0
    }

    /// Check if receipt was written
    pub fn is_used(&self) -> bool {
        self.used == 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_receipt_creation() {
        let receipt = FillReceipt::new();
        assert!(!receipt.is_used());
        assert_eq!(receipt.seqno_committed, 0);
    }

    #[test]
    fn test_receipt_write() {
        let mut receipt = FillReceipt::new();

        receipt.write(
            123,                  // seqno
            1_000_000,           // filled 1.0 BTC
            50_000_000_000,      // vwap $50,000
            50_000_000_000,      // notional $50,000
            10_000_000,          // fee $10
        );

        assert!(receipt.is_used());
        assert_eq!(receipt.seqno_committed, 123);
        assert_eq!(receipt.filled_qty, 1_000_000);
        assert_eq!(receipt.vwap_px, 50_000_000_000);
        assert_eq!(receipt.fee, 10_000_000);
    }
}
