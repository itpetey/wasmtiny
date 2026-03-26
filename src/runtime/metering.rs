use super::{Result, TrapCode, WasmError};
use crate::memory::PAGE_SIZE_BYTES;
use parking_lot::RwLock;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
/// Resource usage statistics for a running instance.
pub struct InstanceStats {
    /// Number of instructions charged to the instance.
    pub executed_instructions: u64,
    /// Current memory size in WebAssembly pages.
    pub memory_pages: u32,
    /// Current memory size in bytes.
    pub memory_bytes: u64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
/// Configurable resource limits for a running instance.
pub struct InstanceLimits {
    /// Maximum charged instructions allowed before trapping.
    pub max_execution_instructions: Option<u64>,
    /// Maximum linear-memory size allowed in pages.
    pub max_memory_pages: Option<u32>,
}

impl InstanceLimits {
    /// Constant `fn`.
    pub const fn new(
        max_execution_instructions: Option<u64>,
        max_memory_pages: Option<u32>,
    ) -> Self {
        Self {
            max_execution_instructions,
            max_memory_pages,
        }
    }
}

#[derive(Debug, Default)]
pub(crate) struct InstanceMeter {
    state: RwLock<InstanceMeterState>,
}

#[derive(Debug, Clone, Copy, Default)]
struct InstanceMeterState {
    executed_instructions: u64,
    limits: InstanceLimits,
}

impl InstanceMeter {
    pub(crate) fn snapshot(&self, memory_pages: u32) -> InstanceStats {
        let state = *self.state.read();
        InstanceStats {
            executed_instructions: state.executed_instructions,
            memory_pages,
            memory_bytes: memory_pages as u64 * PAGE_SIZE_BYTES as u64,
        }
    }

    pub(crate) fn limits(&self) -> InstanceLimits {
        self.state.read().limits
    }

    pub(crate) fn set_limits(
        &self,
        limits: InstanceLimits,
        current_memory_pages: u32,
    ) -> Result<()> {
        let mut state = self.state.write();
        if let Some(max_memory_pages) = limits.max_memory_pages
            && current_memory_pages > max_memory_pages
        {
            return Err(WasmError::Runtime(
                "current memory usage exceeds configured memory limit".to_string(),
            ));
        }

        if let Some(max_execution_instructions) = limits.max_execution_instructions
            && state.executed_instructions > max_execution_instructions
        {
            return Err(WasmError::Runtime(
                "current execution usage exceeds configured instruction limit".to_string(),
            ));
        }

        state.limits = limits;
        Ok(())
    }

    pub(crate) fn charge_execution(&self, units: u64) -> Result<()> {
        if units == 0 {
            return Ok(());
        }

        let mut state = self.state.write();
        let Some(next) = state.executed_instructions.checked_add(units) else {
            state.executed_instructions = u64::MAX;
            if state.limits.max_execution_instructions.is_some() {
                return Err(WasmError::Trap(TrapCode::ExecutionBudgetExceeded));
            }

            return Err(WasmError::Runtime(
                "execution instruction count overflowed".to_string(),
            ));
        };

        state.executed_instructions = next;
        if let Some(max_execution_instructions) = state.limits.max_execution_instructions
            && next > max_execution_instructions
        {
            return Err(WasmError::Trap(TrapCode::ExecutionBudgetExceeded));
        }

        Ok(())
    }

    pub(crate) fn ensure_memory_pages(&self, new_total_pages: u32) -> Result<()> {
        let state = self.state.read();
        if let Some(max_memory_pages) = state.limits.max_memory_pages
            && new_total_pages > max_memory_pages
        {
            return Err(WasmError::Trap(TrapCode::MemoryLimitExceeded));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_limits_rejects_already_exceeded_execution_budget() {
        let meter = InstanceMeter::default();
        meter.charge_execution(5).unwrap();

        let error = meter
            .set_limits(InstanceLimits::new(Some(4), None), 0)
            .unwrap_err();

        assert!(
            matches!(error, WasmError::Runtime(message) if message.contains("execution usage"))
        );
    }

    #[test]
    fn test_charge_execution_traps_when_counter_overflows_budget() {
        let meter = InstanceMeter::default();
        meter
            .set_limits(InstanceLimits::new(Some(u64::MAX), None), 0)
            .unwrap();
        meter.charge_execution(u64::MAX).unwrap();

        let error = meter.charge_execution(1).unwrap_err();

        assert_eq!(error, WasmError::Trap(TrapCode::ExecutionBudgetExceeded));
        assert_eq!(meter.snapshot(0).executed_instructions, u64::MAX);
    }

    #[test]
    fn test_set_limits_rejects_already_exceeded_memory_limit() {
        let meter = InstanceMeter::default();

        let error = meter
            .set_limits(InstanceLimits::new(None, Some(1)), 2)
            .unwrap_err();

        assert!(matches!(error, WasmError::Runtime(message) if message.contains("memory usage")));
    }
}
