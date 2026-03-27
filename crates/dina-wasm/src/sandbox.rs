use thiserror::Error;

/// Hard limits enforced on every contract execution to prevent abuse.
#[derive(Debug, Clone)]
pub struct SandboxLimits {
    /// Maximum WASM linear memory in bytes (default 16 MiB).
    pub max_memory: usize,
    /// Maximum gas (fuel) a single call may consume.
    pub max_gas: u64,
    /// Wall-clock timeout for a single execution in milliseconds.
    pub max_execution_time_ms: u64,
    /// Maximum number of storage writes per call.
    pub max_storage_writes: u32,
    /// Maximum nested cross-contract call depth.
    pub max_call_depth: u32,
    /// Maximum number of events a single call may emit.
    pub max_events: u32,
}

impl Default for SandboxLimits {
    fn default() -> Self {
        Self {
            max_memory: 16 * 1024 * 1024, // 16 MiB
            max_gas: 10_000_000,
            max_execution_time_ms: 100,
            max_storage_writes: 100,
            max_call_depth: 10,
            max_events: 50,
        }
    }
}

/// Violations raised when a contract exceeds sandbox limits.
#[derive(Error, Debug, Clone)]
pub enum SandboxViolation {
    #[error("out of gas: limit was {limit}")]
    OutOfGas { limit: u64 },

    #[error("memory limit exceeded: requested {requested} bytes, limit {limit} bytes")]
    MemoryLimitExceeded { requested: usize, limit: usize },

    #[error("execution timeout: exceeded {limit_ms}ms")]
    ExecutionTimeout { limit_ms: u64 },

    #[error("storage write limit exceeded: {count} writes, limit {limit}")]
    StorageWriteLimitExceeded { count: u32, limit: u32 },

    #[error("call depth exceeded: depth {depth}, limit {limit}")]
    CallDepthExceeded { depth: u32, limit: u32 },

    #[error("event limit exceeded: {count} events, limit {limit}")]
    EventLimitExceeded { count: u32, limit: u32 },
}

impl SandboxLimits {
    /// Check that the current counters are within limits.
    ///
    /// Returns `Ok(())` if everything is within bounds, or the first
    /// violation encountered.
    pub fn validate_within_limits(
        &self,
        storage_writes: u32,
        call_depth: u32,
        events: u32,
    ) -> Result<(), SandboxViolation> {
        if storage_writes > self.max_storage_writes {
            return Err(SandboxViolation::StorageWriteLimitExceeded {
                count: storage_writes,
                limit: self.max_storage_writes,
            });
        }
        if call_depth > self.max_call_depth {
            return Err(SandboxViolation::CallDepthExceeded {
                depth: call_depth,
                limit: self.max_call_depth,
            });
        }
        if events > self.max_events {
            return Err(SandboxViolation::EventLimitExceeded {
                count: events,
                limit: self.max_events,
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_limits() {
        let limits = SandboxLimits::default();
        assert_eq!(limits.max_memory, 16 * 1024 * 1024);
        assert_eq!(limits.max_gas, 10_000_000);
        assert_eq!(limits.max_execution_time_ms, 100);
        assert_eq!(limits.max_storage_writes, 100);
        assert_eq!(limits.max_call_depth, 10);
        assert_eq!(limits.max_events, 50);
    }

    #[test]
    fn validate_passes_within_limits() {
        let limits = SandboxLimits::default();
        assert!(limits.validate_within_limits(50, 5, 25).is_ok());
    }

    #[test]
    fn validate_catches_storage_writes() {
        let limits = SandboxLimits::default();
        let err = limits.validate_within_limits(101, 1, 1).unwrap_err();
        assert!(matches!(
            err,
            SandboxViolation::StorageWriteLimitExceeded { .. }
        ));
    }

    #[test]
    fn validate_catches_call_depth() {
        let limits = SandboxLimits::default();
        let err = limits.validate_within_limits(1, 11, 1).unwrap_err();
        assert!(matches!(err, SandboxViolation::CallDepthExceeded { .. }));
    }

    #[test]
    fn validate_catches_events() {
        let limits = SandboxLimits::default();
        let err = limits.validate_within_limits(1, 1, 51).unwrap_err();
        assert!(matches!(err, SandboxViolation::EventLimitExceeded { .. }));
    }
}
