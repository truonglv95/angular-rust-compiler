
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorCode {
    DecoratorArgNotLiteral = 1001,
    DecoratorArityWrong = 1002,
    DecoratorNotCalled = 1003,
    DecoratorUnexpected = 1005,

    /// This error code indicates that there are incompatible decorators on a type or a class field.
    DecoratorCollision = 1006,

    ValueHasWrongType = 1010,
    ValueNotLiteral = 1011,

    DuplicateDecoratedProperties = 1012,

    /// Raised when an initializer API is annotated with an unexpected decorator.
    ///
    /// e.g. `@Input` is also applied on the class member using `input`.
    InitializerApiWithDisallowedDecorator = 1050,

    /// Raised when an initializer API feature (like signal inputs) are also
    /// declared in the class decorator metadata.
    ///
    /// e.g. a signal input is also declared in the `@Directive` `inputs` array.
    InitializerApiDecoratorMetadataCollision = 1051,

    /// Raised whenever an initializer API does not support the `.required`
    /// function, but is still detected unexpectedly.
    InitializerApiNoRequiredFunction = 1052,

    /// Raised whenever an initializer API is used on a class member
    /// and the given access modifiers (e.g. `private`) are not allowed.
    InitializerApiDisallowedMemberVisibility = 1053,

    /// An Angular feature, like inputs, outputs or queries is incorrectly
    /// declared on a static member.
    IncorrectlyDeclaredOnStaticMember = 1100,

    ComponentMissingTemplate = 2001,
    PipeMissingName = 2002,
    ParamMissingToken = 2003,
    DirectiveMissingSelector = 2004,

    /// Raised when an undecorated class is passed in as a provider to a module or a directive.
    UndecoratedProvider = 2005,

    /// Raised when a Directive inherits its constructor from a base class without an Angular
    /// decorator.
    DirectiveInheritsUndecoratedCtor = 2006,

    /// Raised when an undecorated class that is using Angular features
    /// has been discovered.
    UndecoratedClassUsingAngularFeatures = 2007,

    /// Raised when an component cannot resolve an external resource, such as a template or a style
    /// sheet.
    ComponentResourceNotFound = 2008,

    /// Raised when a component uses `ShadowDom` view encapsulation, but its selector
    /// does not match the shadow DOM tag name requirements.
    ComponentInvalidShadowDomSelector = 2009,

    /// Raised when a component has `imports` but is not marked as `standalone: true`.
    ComponentNotStandalone = 2010,

    /// Raised when a type in the `imports` of a component is a directive or pipe, but is not
    /// standalone.
    ComponentImportNotStandalone = 2011,

    /// Raised when a type in the `imports` of a component is not a directive, pipe, or NgModule.
    ComponentUnknownImport = 2012,

    /// Raised when the compiler wasn't able to resolve the metadata of a host directive.
    HostDirectiveInvalid = 2013,

    /// Raised when a host directive isn't standalone.
    HostDirectiveNotStandalone = 2014,

    /// Raised when a host directive is a component.
    HostDirectiveComponent = 2015,

    /// Raised when a type with Angular decorator inherits its constructor from a base class
    /// which has a constructor that is incompatible with Angular DI.
    InjectableInheritsInvalidConstructor = 2016,

    /// Raised when a host tries to alias a host directive binding that does not exist.
    HostDirectiveUndefinedBinding = 2017,

    /// Raised when a host tries to alias a host directive
    /// binding to a pre-existing binding's public name.
    HostDirectiveConflictingAlias = 2018,

    /// Raised when a host directive definition doesn't expose a
    /// required binding from the host directive.
    HostDirectiveMissingRequiredBinding = 2019,

    /// Raised when a component specifies both a `transform` function on an input
    /// and has a corresponding `ngAcceptInputType_` member for the same input.
    ConflictingInputTransform = 2020,

    /// Raised when a component has both `styleUrls` and `styleUrl`.
    ComponentInvalidStyleUrls = 2021,

    /// Raised when a type in the `deferredImports` of a component is not a component, directive or
    /// pipe.
    ComponentUnknownDeferredImport = 2022,

    /// Raised when a `standalone: false` component is declared but `strictStandalone` is set.
    NonStandaloneNotAllowed = 2023,

    /// Raised when a named template dependency isn't defined in the component's source file.
    MissingNamedTemplateDependency = 2024,

    /// Raised if an incorrect type is used for a named template dependency (e.g. directive
    /// class used as a component).
    IncorrectNamedTemplateDependencyType = 2025,

    /// Raised for `@Component` fields that aren't supported in a selectorless context.
    UnsupportedSelectorlessComponentField = 2026,

    /// A component is using both the `animations` property and `animate.enter` or `animate.leave`
    /// in the template.
    ComponentAnimationsConflict = 2027,

    SymbolNotExported = 3001,
    /// Raised when a relationship between directives and/or pipes would cause a cyclic import to be
    /// created that cannot be handled, such as in partial compilation mode.
    ImportCycleDetected = 3003,

    /// Raised when the compiler is unable to generate an import statement for a reference.
    ImportGenerationFailure = 3004,

    ConfigFlatModuleNoIndex = 4001,
    ConfigStrictTemplatesImpliesFullTemplateTypecheck = 4002,
    ConfigExtendedDiagnosticsImpliesStrictTemplates = 4003,
    ConfigExtendedDiagnosticsUnknownCategoryLabel = 4004,
    ConfigExtendedDiagnosticsUnknownCheck = 4005,
    ConfigEmitDeclarationOnlyUnsupported = 4006,

    /// Raised when a host expression has a parse error, such as a host listener or host binding
    /// expression containing a pipe.
    HostBindingParseError = 5001,

    /// Raised when the compiler cannot parse a component's template.
    TemplateParseError = 5002,

    /// Raised when an NgModule contains an invalid reference in `declarations`.
    NgmoduleInvalidDeclaration = 6001,

    /// Raised when an NgModule contains an invalid type in `imports`.
    NgmoduleInvalidImport = 6002,

    /// Raised when an NgModule contains an invalid type in `exports`.
    NgmoduleInvalidExport = 6003,

    /// Raised when an NgModule contains a type in `exports` which is neither in `declarations` nor
    /// otherwise imported.
    NgmoduleInvalidReexport = 6004,

    /// Raised when a `ModuleWithProviders` with a missing
    /// generic type argument is passed into an `NgModule`.
    NgmoduleModuleWithProvidersMissingGeneric = 6005,

    /// Raised when an NgModule exports multiple directives/pipes of the same name and the compiler
    /// attempts to generate private re-exports within the NgModule file.
    NgmoduleReexportNameCollision = 6006,

    /// Raised when a directive/pipe is part of the declarations of two or more NgModules.
    NgmoduleDeclarationNotUnique = 6007,

    /// Raised when a standalone directive/pipe is part of the declarations of an NgModule.
    NgmoduleDeclarationIsStandalone = 6008,

    /// Raised when a standalone component is part of the bootstrap list of an NgModule.
    NgmoduleBootstrapIsStandalone = 6009,

    /// Indicates that an NgModule is declared with `id: module.id`. This is an anti-pattern that is
    /// disabled explicitly in the compiler, that was originally based on a misunderstanding of
    /// `NgModule.id`.
    WarnNgmoduleIdUnnecessary = 6100,

    /// 6999 was previously assigned to NGMODULE_VE_DEPENDENCY_ON_IVY_LIB
    /// To prevent any confusion, let's not reassign it.

    /// An element name failed validation against the DOM schema.
    SchemaInvalidElement = 8001,

    /// An element's attribute name failed validation against the DOM schema.
    SchemaInvalidAttribute = 8002,

    /// No matching directive was found for a `#ref="target"` expression.
    MissingReferenceTarget = 8003,

    /// No matching pipe was found for a
    MissingPipe = 8004,

    /// The left-hand side of an assignment expression was a template variable. Effectively, the
    /// template looked like:
    ///
    /// ```html
    /// <ng-template let-something>
    ///   <button (click)="something = ...">...</button>
    /// </ng-template>
    /// ```
    ///
    /// Template variables are read-only.
    WriteToReadOnlyVariable = 8005,

    /// A template variable was declared twice. For example:
    ///
    /// ```html
    /// <div *ngFor="let i of items; let i = index">
    /// </div>
    /// ```
    DuplicateVariableDeclaration = 8006,

    /// A template has a two way binding (two bindings created by a single syntactical element)
    /// in which the input and output are going to different places.
    SplitTwoWayBinding = 8007,

    /// A directive usage isn't binding to one or more required inputs.
    MissingRequiredInputs = 8008,

    /// The tracking expression of a `for` loop block is accessing a variable that is unavailable,
    /// for example:
    ///
    /// ```angular-html
    /// <ng-template let-ref>
    ///   @for (item of items; track ref) {}
    /// </ng-template>
    /// ```
    IllegalForLoopTrackAccess = 8009,

    /// The trigger of a `defer` block cannot access its trigger element,
    /// either because it doesn't exist or it's in a different view.
    ///
    /// ```angular-html
    /// @defer (on interaction(trigger)) {...}
    ///
    /// <ng-template>
    ///   <button #trigger></button>
    /// </ng-template>
    /// ```
    InaccessibleDeferredTriggerElement = 8010,

    /// A control flow node is projected at the root of a component and is preventing its direct
    /// descendants from being projected, because it has more than one root node.
    ///
    /// ```angular-html
    /// <comp>
    ///  @if (expr) {
    ///    <div projectsIntoSlot></div>
    ///    Text preventing the div from being projected
    ///  }
    /// </comp>
    /// ```
    ControlFlowPreventingContentProjection = 8011,

    /// A pipe imported via `@Component.deferredImports` is
    /// used outside of a `@defer` block in a template.
    DeferredPipeUsedEagerly = 8012,

    /// A directive/component imported via `@Component.deferredImports` is
    /// used outside of a `@defer` block in a template.
    DeferredDirectiveUsedEagerly = 8013,

    /// A directive/component/pipe imported via `@Component.deferredImports` is
    /// also included into the `@Component.imports` list.
    DeferredDependencyImportedEagerly = 8014,

    /// An expression is trying to write to an `@let` declaration.
    IllegalLetWrite = 8015,

    /// An expression is trying to read an `@let` before it has been defined.
    LetUsedBeforeDefinition = 8016,

    /// A `@let` declaration conflicts with another symbol in the same scope.
    ConflictingLetDeclaration = 8017,

    /// A binding inside selectorless directive syntax did
    /// not match any inputs/outputs of the directive.
    UnclaimedDirectiveBinding = 8018,

    /// An `@defer` block with an implicit trigger does not have a placeholder, for example:
    ///
    /// ```
    /// @defer(on viewport) {
    ///   Hello
    /// }
    /// ```
    DeferImplicitTriggerMissingPlaceholder = 8019,

    /// The `@placeholder` for an implicit `@defer` trigger is not set up correctly, for example:
    ///
    /// ```
    /// @defer(on viewport) {
    ///   Hello
    /// } @placeholder {
    ///   <!-- Multiple root nodes. -->
    ///   <button></button>
    ///   <div></div>
    /// }
    /// ```
    DeferImplicitTriggerInvalidPlaceholder = 8020,

    /// Raised when an `@defer` block defines unreachable or redundant triggers.
    /// Examples: multiple main triggers, 'on immediate' together with other mains or any prefetch,
    /// prefetch timer delay that is not earlier than the main timer, or an identical prefetch
    DeferTriggerMisconfiguration = 8021,

    /// Raised when the user has an unsupported binding on a `Field` directive.
    FormFieldUnsupportedBinding = 8022,

    /// A two way binding in a template has an incorrect syntax,
    /// parentheses outside brackets. For example:
    ///
    /// ```html
    /// <div ([foo])="bar" />
    /// ```
    InvalidBananaInBox = 8101,

    /// The left side of a nullish coalescing operation is not nullable.
    ///
    /// ```html
    /// {{ foo ?? bar }}
    /// ```
    /// When the type of foo doesn't include `null` or `undefined`.
    NullishCoalescingNotNullable = 8102,

    /// A known control flow directive (e.g. `*ngIf`) is used in a template,
    /// but the `CommonModule` is not imported.
    MissingControlFlowDirective = 8103,

    /// A text attribute is not interpreted as a binding but likely intended to be.
    ///
    /// For example:
    /// ```html
    /// <div
    ///   attr.x="value"
    ///   class.blue="true"
    ///   style.margin-right.px="5">
    /// </div>
    /// ```
    ///
    /// All of the above attributes will just be static text attributes and will not be interpreted as
    /// bindings by the compiler.
    TextAttributeNotBinding = 8104,

    /// NgForOf is used in a template, but the user forgot to include let
    /// in their statement.
    ///
    /// For example:
    /// ```html
    /// <ul><li *ngFor="item of items">{{item["name"]}};</li></ul>
    /// ```
    MissingNgforofLet = 8105,
    /// Indicates that the binding suffix is not supported
    ///
    /// Style bindings support suffixes like `style.width.px`, `.em`, and `.%`.
    /// These suffixes are _not_ supported for attribute bindings.
    ///
    /// For example `[attr.width.px]="5"` becomes `width.px="5"` when bound.
    /// This is almost certainly unintentional and this error is meant to
    /// surface this mistake to the developer.
    SuffixNotSupported = 8106,

    /// The left side of an optional chain operation is not nullable.
    ///
    /// ```html
    /// {{ foo?.bar }}
    /// {{ foo?.['bar'] }}
    /// {{ foo?.() }}
    /// ```
    /// When the type of foo doesn't include `null` or `undefined`.
    OptionalChainNotNullable = 8107,

    /// `ngSkipHydration` should not be a binding (it should be a static attribute).
    ///
    /// For example:
    /// ```html
    /// <my-cmp [ngSkipHydration]="someTruthyVar" />
    /// ```
    ///
    /// `ngSkipHydration` cannot be a binding and can not have values other than "true" or an empty
    /// value
    SkipHydrationNotStatic = 8108,

    /// Signal functions should be invoked when interpolated in templates.
    ///
    /// For example:
    /// ```html
    /// {{ mySignal() }}
    /// ```
    InterpolatedSignalNotInvoked = 8109,

    /// Initializer-based APIs can only be invoked from inside of an initializer.
    ///
    /// ```ts
    /// // Allowed
    /// myInput = input();
    ///
    /// // Not allowed
    /// function myInput() {
    ///   return input();
    /// }
    /// ```
    UnsupportedInitializerApiUsage = 8110,

    /// A function in an event binding is not called.
    ///
    /// For example:
    /// ```html
    /// <button (click)="myFunc"></button>
    /// ```
    ///
    /// This will not call `myFunc` when the button is clicked. Instead, it should be
    /// `<button (click)="myFunc()"></button>`.
    UninvokedFunctionInEventBinding = 8111,

    /// A `@let` declaration in a template isn't used.
    ///
    /// For example:
    /// ```angular-html
    /// @let used = 1; <!-- Not an error -->
    /// @let notUsed = 2; <!-- Error -->
    ///
    /// {{used}}
    /// ```
    UnusedLetDeclaration = 8112,

    /// A symbol referenced in `@Component.imports` isn't being used within the template.
    UnusedStandaloneImports = 8113,

    /// An expression mixes nullish coalescing and logical and/or without parentheses.
    UnparenthesizedNullishCoalescing = 8114,

    /// The function passed to `@for` track is not invoked.
    ///
    /// For example:
    /// ```angular-html
    /// @for (item of items; track trackByName) {}
    /// ```
    ///
    /// For the track function to work properly, it must be invoked.
    ///
    /// For example:
    /// ```angular-html
    /// @for (item of items; track trackByName(item)) {}
    /// ```
    UninvokedTrackFunction = 8115,

    /// A structural directive is used in a template, but the directive is not imported.
    MissingStructuralDirective = 8116,

    /// A function in a text interpolation is not invoked.
    ///
    /// For example:
    /// ```html
    /// <p> {{ firstName }} </p>
    /// ```
    ///
    /// The `firstName` function is not invoked. Instead, it should be:
    /// ```html
    /// <p> {{ firstName() }} </p>
    /// ```
    UninvokedFunctionInTextInterpolation = 8117,

    /// A required initializer is being invoked in a forbidden context such as a property initializer
    /// or a constructor.
    ///
    /// For example:
    /// ```ts
    /// class MyComponent {
    ///  myInput = input.required();
    ///  somValue = this.myInput(); // Error
    ///
    ///  constructor() {
    ///    this.myInput(); // Error
    ///  }
    ForbiddenRequiredInitializerInvocation = 8118,

    /// The template type-checking engine would need to generate an inline type check block for a
    /// component, but the current type-checking environment doesn't support it.
    InlineTcbRequired = 8900,

    /// The template type-checking engine would need to generate an inline type constructor for a
    /// directive or component, but the current type-checking environment doesn't support it.
    InlineTypeCtorRequired = 8901,

    /// An injectable already has a `ɵprov` property.
    InjectableDuplicateProv = 9001,

    // 10XXX error codes are reserved for diagnostics with categories other than
    // `ts.DiagnosticCategory.Error`. These diagnostics are generated by the compiler when configured
    // to do so by a tool such as the Language Service, or by the Language Service itself.

    /// Suggest users to enable `strictTemplates` to make use of full capabilities
    /// provided by Angular language service.
    SuggestStrictTemplates = 10001,

    /// Indicates that a particular structural directive provides advanced type narrowing
    /// functionality, but the current template type-checking configuration does not allow its usage in
    /// type inference.
    SuggestSuboptimalTypeInference = 10002,

    /// In local compilation mode a const is required to be resolved statically but cannot be so since
    /// it is imported from a file outside of the compilation unit. This usually happens with const
    /// being used as Angular decorators parameters such as `@Component.template`,
    /// `@HostListener.eventName`, etc.
    LocalCompilationUnresolvedConst = 11001,

    /// In local compilation mode a certain expression or syntax is not supported. This is usually
    /// because the expression/syntax is not very common and so we did not add support for it yet. This
    /// can be changed in the future and support for more expressions could be added if need be.
    /// Meanwhile, this error is thrown to indicate a current unavailability.
    LocalCompilationUnsupportedExpression = 11003,
}
