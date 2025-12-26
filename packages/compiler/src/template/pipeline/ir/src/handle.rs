//! IR Handles
//!
//! Corresponds to packages/compiler/src/template/pipeline/ir/src/handle.ts
//! Defines handles and IDs used in the IR

/// Branded type for a cross-reference ID. During ingest, `XrefId`s are generated to link together
/// different IR operations which need to reference each other.
///
/// Note: In TypeScript, this is defined as `export type XrefId = number & {__brand: 'XrefId'};`
/// In Rust, we use a newtype struct to achieve type safety.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct XrefId(pub usize);

impl XrefId {
    pub fn new(id: usize) -> Self {
        XrefId(id)
    }

    pub fn as_usize(&self) -> usize {
        self.0
    }
}

/// Branded type for a constant index.
/// This is used to index into the consts array.
/// Note: This type is not explicitly defined in the TypeScript handle.ts, but is used throughout
/// the codebase as a branded type for constant indices.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ConstIndex(pub usize);

impl ConstIndex {
    pub fn new(index: usize) -> Self {
        ConstIndex(index)
    }

    pub fn as_usize(&self) -> usize {
        self.0
    }
}

use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex};

/// Slot handle for operations that consume slots.
///
/// In TypeScript, this is defined as a class with a nullable `slot` field:
/// ```typescript
/// export class SlotHandle {
///   slot: number | null = null;
/// }
/// ```
///
/// In Rust, we use a struct with an `Arc<Mutex<Option<usize>>>` to represent the nullable slot value
/// with shared ownership and locked mutability, allowing thread-safe access.
#[derive(Debug, Clone)]
pub struct SlotHandle(Arc<Mutex<Option<usize>>>);

impl SlotHandle {
    /// Create a new SlotHandle with no slot assigned (slot = None/null).
    /// This matches the TypeScript default: `slot: number | null = null`
    /// Create a new specific slot handle (initially unallocated).
    pub fn new() -> Self {
        SlotHandle(Arc::new(Mutex::new(None)))
    }

    /// Create a SlotHandle with a specific slot number.
    pub fn with_slot(slot: usize) -> Self {
        SlotHandle(Arc::new(Mutex::new(Some(slot))))
    }

    /// Check if this handle has an assigned slot.
    pub fn has_slot(&self) -> bool {
        self.0.lock().unwrap().is_some()
    }

    /// Get the slot number if assigned.
    pub fn get_slot(&self) -> Option<usize> {
        *self.0.lock().unwrap()
    }

    /// Set the slot number.
    pub fn set_slot(&self, slot: usize) {
        *self.0.lock().unwrap() = Some(slot);
    }
}

impl Default for SlotHandle {
    fn default() -> Self {
        SlotHandle::new()
    }
}

impl PartialEq for SlotHandle {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for SlotHandle {}

impl Hash for SlotHandle {
    fn hash<H: Hasher>(&self, state: &mut H) {
        Arc::as_ptr(&self.0).hash(state);
    }
}

// Implement PartialEq<usize> for SlotHandle compatibility (value check)
impl PartialEq<usize> for SlotHandle {
    fn eq(&self, other: &usize) -> bool {
        self.get_slot() == Some(*other)
    }
}

impl PartialEq<SlotHandle> for usize {
    fn eq(&self, other: &SlotHandle) -> bool {
        Some(*self) == other.get_slot()
    }
}
