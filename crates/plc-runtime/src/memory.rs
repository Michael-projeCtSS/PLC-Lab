use crate::{
    errors::{InputError, InternalError},
    plan::{MemoryRole, Slot},
};
use plc_types::{PlcValue, identity::SymbolId};
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
            let value = slot.plc_type.default_value();
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
    pub(crate) fn begin_scan(&mut self) -> WorkingState {
        self.input_image = self.staged_inputs.clone();
        WorkingState {
            inputs: self.input_image.clone(),
            writable: self.committed.clone(),
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
