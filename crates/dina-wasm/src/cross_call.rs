use dina_core::error::DinaError;
use dina_core::types::Address;

/// A single frame in the cross-contract call stack.
///
/// Each time contract A calls contract B, a new `CallFrame` is pushed onto the
/// `CrossCallContext` stack. When B returns, its frame is popped and the return
/// data becomes available to A.
#[derive(Debug, Clone)]
pub struct CallFrame {
    /// The address that initiated this call (the calling contract or EOA).
    pub caller: Address,
    /// The contract being called in this frame.
    pub contract: Address,
    /// The method name dispatched in this frame.
    pub method: String,
    /// Nesting depth of this frame (0 = top-level call).
    pub depth: u32,
    /// Maximum gas this frame is allowed to consume.
    pub gas_limit: u64,
    /// Gas actually consumed so far in this frame.
    pub gas_used: u64,
    /// USDC micro-units forwarded with this call.
    pub usdc_forwarded: u64,
    /// Return data produced by this frame (populated on pop).
    pub return_data: Option<Vec<u8>>,
}

/// Manages the cross-contract call stack and enforces depth limits.
///
/// Every contract execution begins with an empty `CrossCallContext`. When a
/// contract issues a cross-contract call, a new frame is pushed. The context
/// prevents unbounded recursion by enforcing `max_depth`.
#[derive(Debug)]
pub struct CrossCallContext {
    call_stack: Vec<CallFrame>,
    max_depth: u32,
    total_gas_used: u64,
}

impl CrossCallContext {
    /// Create a new context with the given maximum call depth.
    pub fn new(max_depth: u32) -> Self {
        Self {
            call_stack: Vec::new(),
            max_depth,
            total_gas_used: 0,
        }
    }

    /// Push a new call frame onto the stack.
    ///
    /// Returns an error if the call would exceed `max_depth`.
    pub fn push_frame(
        &mut self,
        caller: Address,
        contract: Address,
        method: String,
        gas_limit: u64,
        usdc_forwarded: u64,
    ) -> Result<(), DinaError> {
        let depth = self.call_stack.len() as u32;
        if depth >= self.max_depth {
            return Err(DinaError::WasmExecutionError(format!(
                "cross-contract call depth {} exceeds maximum {}",
                depth, self.max_depth
            )));
        }

        self.call_stack.push(CallFrame {
            caller,
            contract,
            method,
            depth,
            gas_limit,
            gas_used: 0,
            usdc_forwarded,
            return_data: None,
        });

        Ok(())
    }

    /// Pop the top frame from the call stack.
    ///
    /// Accumulates the frame's gas usage into the total. Returns an error if
    /// the stack is empty.
    pub fn pop_frame(&mut self) -> Result<CallFrame, DinaError> {
        let frame = self
            .call_stack
            .pop()
            .ok_or_else(|| DinaError::WasmExecutionError("call stack underflow".into()))?;
        self.total_gas_used += frame.gas_used;
        Ok(frame)
    }

    /// Reference to the current (top-of-stack) call frame.
    pub fn current_frame(&self) -> Option<&CallFrame> {
        self.call_stack.last()
    }

    /// Mutable reference to the current (top-of-stack) call frame.
    pub fn current_frame_mut(&mut self) -> Option<&mut CallFrame> {
        self.call_stack.last_mut()
    }

    /// Current nesting depth (number of frames on the stack).
    pub fn depth(&self) -> u32 {
        self.call_stack.len() as u32
    }

    /// Whether another nested call is allowed without exceeding the limit.
    pub fn can_call(&self) -> bool {
        (self.call_stack.len() as u32) < self.max_depth
    }

    /// Total gas consumed across all completed frames.
    pub fn total_gas(&self) -> u64 {
        self.total_gas_used
    }

    /// Read-only view of the entire call stack.
    pub fn call_stack(&self) -> &[CallFrame] {
        &self.call_stack
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn addr(byte: u8) -> Address {
        Address([byte; 32])
    }

    #[test]
    fn new_context_is_empty() {
        let ctx = CrossCallContext::new(5);
        assert_eq!(ctx.depth(), 0);
        assert!(ctx.can_call());
        assert_eq!(ctx.total_gas(), 0);
        assert!(ctx.current_frame().is_none());
    }

    #[test]
    fn push_and_pop_frame() {
        let mut ctx = CrossCallContext::new(5);
        ctx.push_frame(addr(1), addr(2), "transfer".into(), 1000, 50)
            .unwrap();
        assert_eq!(ctx.depth(), 1);

        let frame = ctx.current_frame().unwrap();
        assert_eq!(frame.caller, addr(1));
        assert_eq!(frame.contract, addr(2));
        assert_eq!(frame.method, "transfer");
        assert_eq!(frame.depth, 0);
        assert_eq!(frame.gas_limit, 1000);
        assert_eq!(frame.usdc_forwarded, 50);

        let popped = ctx.pop_frame().unwrap();
        assert_eq!(popped.method, "transfer");
        assert_eq!(ctx.depth(), 0);
    }

    #[test]
    fn depth_tracking() {
        let mut ctx = CrossCallContext::new(10);
        for i in 0..5 {
            ctx.push_frame(addr(0), addr(i as u8 + 1), format!("m{i}"), 100, 0)
                .unwrap();
        }
        assert_eq!(ctx.depth(), 5);

        ctx.pop_frame().unwrap();
        assert_eq!(ctx.depth(), 4);
    }

    #[test]
    fn max_depth_enforced() {
        let mut ctx = CrossCallContext::new(2);
        ctx.push_frame(addr(1), addr(2), "a".into(), 100, 0)
            .unwrap();
        ctx.push_frame(addr(2), addr(3), "b".into(), 100, 0)
            .unwrap();

        let result = ctx.push_frame(addr(3), addr(4), "c".into(), 100, 0);
        assert!(result.is_err());
        assert!(!ctx.can_call());
    }

    #[test]
    fn can_call_reflects_capacity() {
        let mut ctx = CrossCallContext::new(1);
        assert!(ctx.can_call());
        ctx.push_frame(addr(1), addr(2), "x".into(), 100, 0)
            .unwrap();
        assert!(!ctx.can_call());
    }

    #[test]
    fn total_gas_accumulates_across_frames() {
        let mut ctx = CrossCallContext::new(5);

        ctx.push_frame(addr(1), addr(2), "a".into(), 500, 0)
            .unwrap();
        ctx.current_frame_mut().unwrap().gas_used = 100;
        ctx.pop_frame().unwrap();

        ctx.push_frame(addr(1), addr(3), "b".into(), 500, 0)
            .unwrap();
        ctx.current_frame_mut().unwrap().gas_used = 250;
        ctx.pop_frame().unwrap();

        assert_eq!(ctx.total_gas(), 350);
    }

    #[test]
    fn pop_empty_stack_returns_error() {
        let mut ctx = CrossCallContext::new(5);
        let result = ctx.pop_frame();
        assert!(result.is_err());
    }

    #[test]
    fn call_stack_view() {
        let mut ctx = CrossCallContext::new(5);
        ctx.push_frame(addr(1), addr(2), "first".into(), 100, 0)
            .unwrap();
        ctx.push_frame(addr(2), addr(3), "second".into(), 200, 10)
            .unwrap();

        let stack = ctx.call_stack();
        assert_eq!(stack.len(), 2);
        assert_eq!(stack[0].method, "first");
        assert_eq!(stack[1].method, "second");
        assert_eq!(stack[1].depth, 1);
    }

    #[test]
    fn frame_return_data() {
        let mut ctx = CrossCallContext::new(5);
        ctx.push_frame(addr(1), addr(2), "calc".into(), 1000, 0)
            .unwrap();
        ctx.current_frame_mut().unwrap().return_data = Some(vec![42, 43, 44]);
        let frame = ctx.pop_frame().unwrap();
        assert_eq!(frame.return_data, Some(vec![42, 43, 44]));
    }

    #[test]
    fn zero_max_depth_blocks_all_calls() {
        let mut ctx = CrossCallContext::new(0);
        assert!(!ctx.can_call());
        let result = ctx.push_frame(addr(1), addr(2), "x".into(), 100, 0);
        assert!(result.is_err());
    }
}
