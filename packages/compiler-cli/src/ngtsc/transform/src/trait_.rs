// Trait System - State machine for tracking decorator compilation progress
//
// Traits are created when a DecoratorHandler matches a class. Each trait begins in a
// pending state and undergoes transitions as compilation proceeds through various steps.

use crate::ngtsc::transform::src::api::{DecoratorHandler, DetectResult};
use ts::Diagnostic;
use std::sync::Arc;

// ============================================================================
// Trait State
// ============================================================================

/// The state of a trait during compilation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TraitState {
    /// Pending traits are freshly created and have never been analyzed.
    Pending,
    
    /// Analyzed traits have successfully been analyzed, but are pending resolution.
    Analyzed,
    
    /// Resolved traits have successfully been analyzed and resolved and are ready for compilation.
    Resolved,
    
    /// Skipped traits are no longer considered for compilation.
    Skipped,
}

// ============================================================================
// Trait Implementation
// ============================================================================

/// An Ivy aspect added to a class (for example, the compilation of a component definition).
///
/// Traits are created when a `DecoratorHandler` matches a class. Each trait begins in a pending
/// state and undergoes transitions as compilation proceeds through the various steps.
///
/// This struct holds all possible state for any trait state, with the `state` field indicating
/// which fields are valid.
pub struct TraitImpl<D, A, S, R> {
    /// Current state of the trait.
    pub state: TraitState,
    
    /// The `DecoratorHandler` which matched on the class to create this trait.
    /// Using Arc to allow shared ownership without complex lifetime issues.
    pub handler: Arc<dyn DecoratorHandler<D, A, S, R>>,
    
    /// The detection result which indicated that this trait applied to the class.
    pub detected: DetectResult<D>,
    
    /// Analysis results of the given trait (valid in Analyzed and Resolved states).
    pub analysis: Option<A>,
    
    /// Semantic symbol for incremental compilation.
    pub symbol: Option<S>,
    
    /// Resolution results (valid in Resolved state).
    pub resolution: Option<R>,
    
    /// Any diagnostics that resulted from analysis.
    pub analysis_diagnostics: Option<Vec<Diagnostic>>,
    
    /// Any diagnostics that resulted from resolution.
    pub resolve_diagnostics: Option<Vec<Diagnostic>>,
}

/// Type alias for the full Trait type.
pub type Trait<D, A, S, R> = TraitImpl<D, A, S, R>;

// ============================================================================
// State-specific type aliases for clarity
// ============================================================================

/// A trait in the pending state - has yet to be analyzed.
pub type PendingTrait<D, A, S, R> = TraitImpl<D, A, S, R>;

/// A trait in the skipped state - not considered for compilation (terminal state).
pub type SkippedTrait<D, A, S, R> = TraitImpl<D, A, S, R>;

/// A trait in the analyzed state - analysis results available, eligible for resolution.
pub type AnalyzedTrait<D, A, S, R> = TraitImpl<D, A, S, R>;

/// A trait in the resolved state - ready for compilation (terminal state).
pub type ResolvedTrait<D, A, S, R> = TraitImpl<D, A, S, R>;

// ============================================================================
// Implementation
// ============================================================================

impl<D, A, S, R> TraitImpl<D, A, S, R> {
    /// Create a new pending trait.
    pub fn pending(
        handler: Arc<dyn DecoratorHandler<D, A, S, R>>,
        detected: DetectResult<D>,
    ) -> PendingTrait<D, A, S, R> {
        TraitImpl {
            state: TraitState::Pending,
            handler,
            detected,
            analysis: None,
            symbol: None,
            resolution: None,
            analysis_diagnostics: None,
            resolve_diagnostics: None,
        }
    }

    /// Transition from Pending to Analyzed state.
    ///
    /// # Panics
    /// Panics if the trait is not in Pending state.
    pub fn to_analyzed(
        &mut self,
        analysis: Option<A>,
        diagnostics: Option<Vec<Diagnostic>>,
        symbol: Option<S>,
    ) {
        self.assert_transition_legal(TraitState::Pending, TraitState::Analyzed);
        self.analysis = analysis;
        self.analysis_diagnostics = diagnostics;
        self.symbol = symbol;
        self.state = TraitState::Analyzed;
    }

    /// Transition from Analyzed to Resolved state.
    ///
    /// # Panics
    /// Panics if the trait is not in Analyzed state, or if analysis is None.
    pub fn to_resolved(
        &mut self,
        resolution: Option<R>,
        diagnostics: Option<Vec<Diagnostic>>,
    ) {
        self.assert_transition_legal(TraitState::Analyzed, TraitState::Resolved);
        if self.analysis.is_none() {
            panic!("Cannot transition an Analyzed trait with a null analysis to Resolved");
        }
        self.resolution = resolution;
        self.resolve_diagnostics = diagnostics;
        self.state = TraitState::Resolved;
    }

    /// Transition from Pending to Skipped state.
    ///
    /// # Panics
    /// Panics if the trait is not in Pending state.
    pub fn to_skipped(&mut self) {
        self.assert_transition_legal(TraitState::Pending, TraitState::Skipped);
        self.state = TraitState::Skipped;
    }

    /// Check if the trait is in Pending state.
    pub fn is_pending(&self) -> bool {
        self.state == TraitState::Pending
    }

    /// Check if the trait is in Analyzed state.
    pub fn is_analyzed(&self) -> bool {
        self.state == TraitState::Analyzed
    }

    /// Check if the trait is in Resolved state.
    pub fn is_resolved(&self) -> bool {
        self.state == TraitState::Resolved
    }

    /// Check if the trait is in Skipped state.
    pub fn is_skipped(&self) -> bool {
        self.state == TraitState::Skipped
    }

    /// Verifies that the trait is currently in the allowed state before transitioning.
    fn assert_transition_legal(&self, allowed_state: TraitState, transition_to: TraitState) {
        if self.state != allowed_state {
            panic!(
                "Assertion failure: cannot transition from {:?} to {:?}.",
                self.state, transition_to
            );
        }
    }
}

/// Factory for creating traits.
pub struct TraitFactory;

impl TraitFactory {
    /// Create a new pending trait.
    pub fn pending<D, A, S, R>(
        handler: Arc<dyn DecoratorHandler<D, A, S, R>>,
        detected: DetectResult<D>,
    ) -> PendingTrait<D, A, S, R> {
        TraitImpl::pending(handler, detected)
    }
}
