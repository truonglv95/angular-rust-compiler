
/// Enum holding the name of each extended template diagnostic. The name is used as a user-meaningful
/// value for configuring the diagnostic in the project's options.
///
/// See the corresponding `ErrorCode` for documentation about each specific error.
/// packages/compiler-cli/src/ngtsc/diagnostics/src/error_code.ts
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExtendedTemplateDiagnosticName {
    InvalidBananaInBox,
    NullishCoalescingNotNullable,
    OptionalChainNotNullable,
    MissingControlFlowDirective,
    MissingStructuralDirective,
    TextAttributeNotBinding,
    UninvokedFunctionInEventBinding,
    MissingNgForOfLet,
    SuffixNotSupported,
    SkipHydrationNotStatic,
    InterpolatedSignalNotInvoked,
    ControlFlowPreventingContentProjection,
    UnusedLetDeclaration,
    UninvokedTrackFunction,
    UnusedStandaloneImports,
    UnparenthesizedNullishCoalescing,
    UninvokedFunctionInTextInterpolation,
    DeferTriggerMisconfiguration,
}

impl std::fmt::Display for ExtendedTemplateDiagnosticName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::InvalidBananaInBox => "invalidBananaInBox",
            Self::NullishCoalescingNotNullable => "nullishCoalescingNotNullable",
            Self::OptionalChainNotNullable => "optionalChainNotNullable",
            Self::MissingControlFlowDirective => "missingControlFlowDirective",
            Self::MissingStructuralDirective => "missingStructuralDirective",
            Self::TextAttributeNotBinding => "textAttributeNotBinding",
            Self::UninvokedFunctionInEventBinding => "uninvokedFunctionInEventBinding",
            Self::MissingNgForOfLet => "missingNgForOfLet",
            Self::SuffixNotSupported => "suffixNotSupported",
            Self::SkipHydrationNotStatic => "skipHydrationNotStatic",
            Self::InterpolatedSignalNotInvoked => "interpolatedSignalNotInvoked",
            Self::ControlFlowPreventingContentProjection => "controlFlowPreventingContentProjection",
            Self::UnusedLetDeclaration => "unusedLetDeclaration",
            Self::UninvokedTrackFunction => "uninvokedTrackFunction",
            Self::UnusedStandaloneImports => "unusedStandaloneImports",
            Self::UnparenthesizedNullishCoalescing => "unparenthesizedNullishCoalescing",
            Self::UninvokedFunctionInTextInterpolation => "uninvokedFunctionInTextInterpolation",
            Self::DeferTriggerMisconfiguration => "deferTriggerMisconfiguration",
        };
        write!(f, "{}", s)
    }
}
