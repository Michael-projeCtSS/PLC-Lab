use crate::{
    errors::{InputError, InternalError},
    plan::{MemoryRole, Slot},
};
use plc_types::{
    PlcValue,
    identity::SymbolId,
    symbols::{Retention, SystemVariable},
};
use std::collections::BTreeMap;
pub(crate) type Values = BTreeMap<SymbolId, PlcValue>;

pub(crate) struct Memory {
    pub(crate) staged_inputs: Values,
    pub(crate) input_image: Values,
    pub(crate) committed: Values,
    pub(crate) outputs: Values,
}
pub(crate) struct WorkingState {
    inputs: Values,
    writable: Values,
}
impl Memory {
    pub(crate) fn new(layout: &BTreeMap<SymbolId, Slot>) -> Self {
        let mut inputs = Values::new();
        let mut committed = Values::new();
        let mut outputs = Values::new();
        for (id, slot) in layout {
            let value = match slot.role {
                MemoryRole::ExternalInput => slot.plc_type.default_value(),
                _ => slot.initial,
            };
            match slot.role {
                MemoryRole::ExternalInput => {
                    inputs.insert(*id, value);
                }
                MemoryRole::ExternalOutput => {
                    outputs.insert(*id, value);
                    committed.insert(*id, value);
                }
                MemoryRole::Internal => {
                    committed.insert(*id, value);
                }
            }
        }
        Self {
            staged_inputs: inputs.clone(),
            input_image: inputs,
            committed,
            outputs,
        }
    }
    pub(crate) fn set_input(
        &mut self,
        layout: &BTreeMap<SymbolId, Slot>,
        id: SymbolId,
        value: PlcValue,
    ) -> Result<(), InputError> {
        let slot = layout.get(&id).ok_or(InputError::UnknownSymbol(id))?;
        if slot.role != MemoryRole::ExternalInput {
            return Err(InputError::NotExternalInput(id));
        }
        if value.plc_type() != slot.plc_type {
            return Err(InputError::TypeMismatch {
                symbol: id,
                expected: slot.plc_type,
                actual: value.plc_type(),
            });
        }
        self.staged_inputs.insert(id, value);
        Ok(())
    }
    /// An online write between scans (ADR-0049): the committed value of a variable or
    /// output; the next scan starts from it. Inputs belong to the process image.
    pub(crate) fn modify(
        &mut self,
        layout: &BTreeMap<SymbolId, Slot>,
        id: SymbolId,
        value: PlcValue,
    ) -> Result<(), InputError> {
        let slot = layout.get(&id).ok_or(InputError::UnknownSymbol(id))?;
        if slot.role == MemoryRole::ExternalInput {
            return Err(InputError::ExternalInput(id));
        }
        if value.plc_type() != slot.plc_type {
            return Err(InputError::TypeMismatch {
                symbol: id,
                expected: slot.plc_type,
                actual: value.plc_type(),
            });
        }
        self.committed.insert(id, value);
        if slot.role == MemoryRole::ExternalOutput {
            self.outputs.insert(id, value);
        }
        Ok(())
    }
    /// Warm initialization: non-retained cells return to their initial values, retained
    /// cells keep theirs; the input images are rebuilt and sampled again (ADR-0038).
    pub(crate) fn warm(&mut self, layout: &BTreeMap<SymbolId, Slot>) {
        let mut fresh = Self::new(layout);
        for (id, slot) in layout {
            if slot.retention == Retention::Retained
                && let Some(value) = self.committed.get(id)
            {
                fresh.committed.insert(*id, *value);
            }
        }
        *self = fresh;
    }
    /// Retained cells of the layout.
    pub(crate) fn retained(layout: &BTreeMap<SymbolId, Slot>) -> usize {
        let retained = layout
            .values()
            .filter(|s| s.retention == Retention::Retained);
        retained.count()
    }
    pub(crate) fn begin_scan(
        &mut self,
        layout: &BTreeMap<SymbolId, Slot>,
        first_scan: bool,
    ) -> WorkingState {
        self.input_image = self.staged_inputs.clone();
        let mut writable = self.committed.clone();
        for (id, slot) in layout {
            if slot.system == Some(SystemVariable::FirstScan) {
                writable.insert(*id, PlcValue::Bool(first_scan));
            }
        }
        WorkingState {
            inputs: self.input_image.clone(),
            writable,
        }
    }
    pub(crate) fn commit(
        &mut self,
        layout: &BTreeMap<SymbolId, Slot>,
        working: WorkingState,
    ) -> Result<(), InternalError> {
        let mut outputs = Values::new();
        for (id, slot) in layout {
            if slot.role == MemoryRole::ExternalOutput {
                let value = working.writable.get(id).copied().ok_or_else(|| {
                    InternalError::new("A compiled output is missing from working memory.")
                })?;
                outputs.insert(*id, value);
            }
        }
        // No fallible semantic work remains; publish both images together under &mut Runtime.
        self.committed = working.writable;
        self.outputs = outputs;
        Ok(())
    }
}
impl WorkingState {
    pub(crate) fn read(&self, id: SymbolId) -> Result<PlcValue, InternalError> {
        self.inputs
            .get(&id)
            .or_else(|| self.writable.get(&id))
            .copied()
            .ok_or_else(|| {
                InternalError::new(
                    "An execution symbol is missing from its compiled memory layout.",
                )
            })
    }
    pub(crate) fn write(&mut self, id: SymbolId, value: PlcValue) -> Result<(), InternalError> {
        let target = self.writable.get_mut(&id).ok_or_else(|| {
            InternalError::new("An execution write does not target writable compiled memory.")
        })?;
        if target.plc_type() != value.plc_type() {
            return Err(InternalError::new(
                "An execution write has an incompatible compiled type.",
            ));
        }
        *target = value;
        Ok(())
    }
}
