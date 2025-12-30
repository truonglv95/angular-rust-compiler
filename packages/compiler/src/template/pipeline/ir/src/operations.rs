//! IR Operations
//!
//! Corresponds to packages/compiler/src/template/pipeline/ir/src/operations.ts
//! Defines the base operation structures and OpList

use crate::parse_util::ParseSourceSpan;
use crate::template::pipeline::ir::enums::OpKind;
use crate::template::pipeline::ir::handle::XrefId;
use std::fmt::Debug;
// use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Base trait for semantic operations being performed within a template.
pub trait Op: Debug {
    /// Get the operation kind
    fn kind(&self) -> OpKind;

    /// Get the source span (if available)
    fn source_span(&self) -> Option<&ParseSourceSpan>;

    fn as_any(&self) -> &dyn std::any::Any;
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}

/// Base trait for creation operations
pub trait CreateOp: Op {
    /// Get the xref ID for this operation
    fn xref(&self) -> XrefId;
}

/// Base trait for update operations
pub trait UpdateOp: Op {
    /// Get the xref ID for this operation
    fn xref(&self) -> XrefId;
}

/// A linked list of `Op` nodes of a given subtype.
///
/// In TypeScript, this uses a proper linked list with head/tail nodes and prev/next pointers.
/// In Rust, we use a Vec-based implementation for simplicity, which provides similar functionality
/// but with different performance characteristics. The Vec approach is simpler and avoids
/// the complexity of managing pointer links, while still providing all necessary operations.
///
/// @param T specific subtype of `Op` nodes which this list contains.
#[derive(Clone)]
pub struct OpList<T> {
    /// The operations in this list
    ops: Vec<T>,
    /// Debug ID of this `OpList` instance.
    debug_list_id: usize,
}

// Manual Debug impl for OpList - handle both regular types and trait objects
impl<T> std::fmt::Debug for OpList<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OpList")
            .field("ops", &format!("[{} operations]", self.ops.len()))
            .field("debug_list_id", &self.debug_list_id)
            .finish()
    }
}

static NEXT_LIST_ID: AtomicUsize = AtomicUsize::new(0);

impl<T> OpList<T> {
    /// Create a new empty OpList with a unique debug ID.
    pub fn new() -> Self {
        let id = NEXT_LIST_ID.fetch_add(1, Ordering::Relaxed);
        OpList {
            ops: Vec::new(),
            debug_list_id: id,
        }
    }

    /// Push a new operation to the tail of the list.
    /// Corresponds to TypeScript `push(op: OpT | Array<OpT>): void`
    pub fn push(&mut self, op: T) {
        self.ops.push(op);
    }

    /// Push multiple operations to the tail of the list.
    /// Corresponds to TypeScript `push(op: Array<OpT>): void`
    pub fn push_all(&mut self, ops: impl IntoIterator<Item = T>) {
        self.ops.extend(ops);
    }

    /// Prepend one or more nodes to the start of the list.
    /// Corresponds to TypeScript `prepend(ops: OpT[]): void`
    pub fn prepend(&mut self, ops: impl IntoIterator<Item = T>) {
        let mut new_ops: Vec<T> = ops.into_iter().collect();
        if new_ops.is_empty() {
            return;
        }

        // Reverse to maintain order when prepending
        new_ops.reverse();
        for op in new_ops {
            self.ops.insert(0, op);
        }
    }

    /// Get an iterator over the operations (forward iteration).
    /// Corresponds to TypeScript iterator protocol.
    pub fn iter(&self) -> std::slice::Iter<'_, T> {
        self.ops.iter()
    }

    /// Get a mutable iterator over the operations (forward iteration).
    pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, T> {
        self.ops.iter_mut()
    }

    /// Get a reversed iterator over the operations.
    /// Corresponds to TypeScript `reversed(): Generator<OpT>`
    pub fn reversed(&self) -> impl Iterator<Item = &T> {
        self.ops.iter().rev()
    }

    /// Get the number of operations
    pub fn len(&self) -> usize {
        self.ops.len()
    }

    /// Check if the list is empty
    pub fn is_empty(&self) -> bool {
        self.ops.is_empty()
    }

    /// Get the debug list ID
    pub fn debug_list_id(&self) -> usize {
        self.debug_list_id
    }

    /// Get a reference to an operation by index
    pub fn get(&self, index: usize) -> Option<&T> {
        self.ops.get(index)
    }

    /// Get a mutable reference to an operation by index
    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        self.ops.get_mut(index)
    }

    /// Remove an operation at the given index and return it.
    /// Corresponds to TypeScript `OpList.remove(op: OpT): void` but uses index instead of reference.
    pub fn remove_at(&mut self, index: usize) -> Option<T> {
        if index < self.ops.len() {
            Some(self.ops.remove(index))
        } else {
            None
        }
    }

    /// Insert an operation at the given index.
    /// Corresponds to TypeScript `OpList.insertBefore(op: OpT, target: OpT): void` but uses index.
    pub fn insert_at(&mut self, index: usize, op: T) {
        if index <= self.ops.len() {
            self.ops.insert(index, op);
        } else {
            self.ops.push(op);
        }
    }

    /// Replace an operation at the given index with a new operation.
    /// Returns the old operation.
    /// Corresponds to TypeScript `OpList.replace(oldOp: OpT, newOp: OpT): void` but uses index.
    pub fn replace_at(&mut self, index: usize, new_op: T) -> Option<T> {
        if index < self.ops.len() {
            Some(std::mem::replace(&mut self.ops[index], new_op))
        } else {
            None
        }
    }

    /// Replace an operation at the given index with multiple operations.
    /// Returns the old operation.
    /// Corresponds to TypeScript `OpList.replaceWithMany(oldOp: OpT, newOps: OpT[]): void` but uses index.
    pub fn replace_at_with_many(&mut self, index: usize, new_ops: Vec<T>) -> Option<T> {
        if new_ops.is_empty() {
            return self.remove_at(index);
        }

        if index < self.ops.len() {
            let old_op = self.ops.remove(index);
            for (i, new_op) in new_ops.into_iter().enumerate() {
                self.ops.insert(index + i, new_op);
            }
            Some(old_op)
        } else {
            None
        }
    }

    /// Clear all operations from the list
    pub fn clear(&mut self) {
        self.ops.clear();
    }

    /// Extend this list with operations from another list
    pub fn extend(&mut self, other: OpList<T>) {
        self.ops.extend(other.ops);
    }

    /// Get all operations as a vector (consumes the list)
    pub fn into_vec(self) -> Vec<T> {
        self.ops
    }

    /// Get a reference to the underlying vector
    pub fn as_slice(&self) -> &[T] {
        &self.ops
    }
}

impl<T> Default for OpList<T> {
    fn default() -> Self {
        Self::new()
    }
}

// Implement Iterator for OpList to allow for-in loops
impl<T> IntoIterator for OpList<T> {
    type Item = T;
    type IntoIter = std::vec::IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.ops.into_iter()
    }
}

impl<'a, T> IntoIterator for &'a OpList<T> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.ops.iter()
    }
}

impl<'a, T> IntoIterator for &'a mut OpList<T> {
    type Item = &'a mut T;
    type IntoIter = std::slice::IterMut<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.ops.iter_mut()
    }
}

/// Alias for CreateOp list
pub type CreateOpList = OpList<Box<dyn CreateOp>>;

/// Alias for UpdateOp list  
pub type UpdateOpList = OpList<Box<dyn UpdateOp>>;
