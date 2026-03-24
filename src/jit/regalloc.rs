use crate::jit::emitter::{Address, Emitter, Reg};
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValueLoc {
    Register(Reg),
    SpillSlot(u32),
}

impl ValueLoc {
    pub fn is_register(&self) -> bool {
        matches!(self, ValueLoc::Register(_))
    }

    pub fn is_spill(&self) -> bool {
        matches!(self, ValueLoc::SpillSlot(_))
    }

    pub fn as_register(&self) -> Option<Reg> {
        match self {
            ValueLoc::Register(r) => Some(*r),
            ValueLoc::SpillSlot(_) => None,
        }
    }

    pub fn as_spill_slot(&self) -> Option<u32> {
        match self {
            ValueLoc::Register(_) => None,
            ValueLoc::SpillSlot(s) => Some(*s),
        }
    }
}

#[derive(Clone, Debug)]
pub struct LiveInterval {
    pub value_id: u32,
    pub start: u32,
    pub end: u32,
    pub loc: ValueLoc,
    pub is_fixed: bool,
    pub is_split: bool,
}

impl LiveInterval {
    pub fn new(value_id: u32, start: u32) -> Self {
        Self {
            value_id,
            start,
            end: start,
            loc: ValueLoc::Register(Reg::Rax),
            is_fixed: false,
            is_split: false,
        }
    }

    pub fn covers(&self, pos: u32) -> bool {
        pos >= self.start && pos < self.end
    }

    pub fn extends_to(&mut self, pos: u32) {
        if pos > self.end {
            self.end = pos;
        }
    }
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Interval(pub u32, pub u32);

pub struct LinearScanAllocator {
    available_regs: Vec<Reg>,
    allocated_regs: HashMap<Reg, u32>,
    spill_slots: u32,
    intervals: Vec<LiveInterval>,
    #[allow(dead_code)]
    spilled_values: HashMap<u32, ValueLoc>,
}

impl LinearScanAllocator {
    pub fn new() -> Self {
        let available_regs = vec![
            Reg::Rax,
            Reg::Rcx,
            Reg::Rdx,
            Reg::Rbx,
            Reg::Rsi,
            Reg::Rdi,
            Reg::R8,
            Reg::R9,
            Reg::R10,
            Reg::R11,
        ];
        Self {
            available_regs,
            allocated_regs: HashMap::new(),
            spill_slots: 0,
            intervals: Vec::new(),
            spilled_values: HashMap::new(),
        }
    }

    pub fn create_interval(&mut self, value_id: u32, start: u32) {
        let interval = LiveInterval::new(value_id, start);
        self.intervals.push(interval);
    }

    pub fn get_interval(&self, value_id: u32) -> Option<&LiveInterval> {
        self.intervals.iter().find(|i| i.value_id == value_id)
    }

    pub fn get_interval_mut(&mut self, value_id: u32) -> Option<&mut LiveInterval> {
        self.intervals.iter_mut().find(|i| i.value_id == value_id)
    }

    pub fn update_interval_end(&mut self, value_id: u32, end: u32) {
        if let Some(interval) = self.get_interval_mut(value_id) {
            interval.extends_to(end);
        }
    }

    pub fn set_location(&mut self, value_id: u32, loc: ValueLoc) {
        if let Some(interval) = self.get_interval_mut(value_id) {
            interval.loc = loc;
        }
    }

    pub fn allocate_register(&mut self) -> Option<Reg> {
        for reg in &self.available_regs {
            if !self.allocated_regs.contains_key(reg) {
                return Some(*reg);
            }
        }
        None
    }

    pub fn allocate_spill_slot(&mut self) -> u32 {
        let slot = self.spill_slots;
        self.spill_slots += 1;
        slot
    }

    pub fn free_register(&mut self, reg: Reg) {
        self.allocated_regs.remove(&reg);
    }

    pub fn alloc(&mut self) {
        self.intervals.sort_by_key(|i| i.start);

        let intervals_data: Vec<(u32, bool)> = self
            .intervals
            .iter()
            .map(|i| (i.value_id, i.is_fixed))
            .collect();

        let mut new_locations: HashMap<u32, ValueLoc> = HashMap::new();

        for (value_id, is_fixed) in intervals_data {
            if is_fixed {
                continue;
            }

            if let Some(reg) = self.allocate_register() {
                new_locations.insert(value_id, ValueLoc::Register(reg));
            } else {
                let spill_slot = self.allocate_spill_slot();
                new_locations.insert(value_id, ValueLoc::SpillSlot(spill_slot));
            }
        }

        for (value_id, loc) in new_locations {
            self.set_location(value_id, loc);
        }
    }

    pub fn get_location(&self, value_id: u32) -> Option<ValueLoc> {
        self.get_interval(value_id).map(|i| i.loc)
    }

    pub fn spill_slot_offset(slot: u32) -> i32 {
        -(8 * (slot as i32 + 1))
    }

    pub fn emit_spill(&self, emitter: &mut Emitter, _value_id: u32, slot: u32) {
        let offset = Self::spill_slot_offset(slot);
        let addr = Address::new(Reg::Rsp).with_displacement(offset);
        emitter.emit_mov_mr(&addr, Reg::Rax);
    }

    pub fn emit_reload(&self, emitter: &mut Emitter, _value_id: u32, slot: u32) {
        let offset = Self::spill_slot_offset(slot);
        let addr = Address::new(Reg::Rsp).with_displacement(offset);
        emitter.emit_mov_rm(Reg::Rax, &addr);
    }
}

impl Default for LinearScanAllocator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_allocator_creation() {
        let alloc = LinearScanAllocator::new();
        assert_eq!(alloc.spill_slots, 0);
    }

    #[test]
    fn test_create_interval() {
        let mut alloc = LinearScanAllocator::new();
        alloc.create_interval(1, 0);
        let interval = alloc.get_interval(1);
        assert!(interval.is_some());
        assert_eq!(interval.unwrap().value_id, 1);
    }

    #[test]
    fn test_allocate_register() {
        let mut alloc = LinearScanAllocator::new();
        let reg = alloc.allocate_register();
        assert!(reg.is_some());
    }

    #[test]
    fn test_spill_slot_allocation() {
        let mut alloc = LinearScanAllocator::new();
        let slot1 = alloc.allocate_spill_slot();
        let slot2 = alloc.allocate_spill_slot();
        assert_eq!(slot1, 0);
        assert_eq!(slot2, 1);
    }

    #[test]
    fn test_spill_slot_offset() {
        assert_eq!(LinearScanAllocator::spill_slot_offset(0), -8);
        assert_eq!(LinearScanAllocator::spill_slot_offset(1), -16);
    }

    #[test]
    fn test_linear_scan_alloc() {
        let mut alloc = LinearScanAllocator::new();
        alloc.create_interval(1, 0);
        alloc.create_interval(2, 5);
        alloc.create_interval(3, 10);

        alloc.update_interval_end(1, 20);
        alloc.update_interval_end(2, 15);
        alloc.update_interval_end(3, 25);

        alloc.alloc();

        for i in 1..=3 {
            let loc = alloc.get_location(i);
            assert!(loc.is_some());
        }
    }
}
