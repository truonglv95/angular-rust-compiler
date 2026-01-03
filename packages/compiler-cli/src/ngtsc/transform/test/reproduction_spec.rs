#[cfg(test)]
mod tests {
    use crate::ngtsc::core::{
        CompilationTicket, CompilationTicketKind, NgCompiler, NgCompilerOptions,
    };
    use crate::ngtsc::file_system::testing::MockFileSystem;
    use crate::ngtsc::file_system::{FileSystem, ReadonlyFileSystem};
    use std::sync::Arc;

    #[test]
    fn test_reproduce_mat_form_field_output() {
        let fs = MockFileSystem::new_posix();
        fs.init_with_files(vec![(
            "/app.component.ts",
            r#"
                import { Component, ElementRef, ViewChild, contentChild, viewChild, computed } from '@angular/core';

                @Component({
                    selector: 'mat-form-field',
                    template: `
                        <div class="mat-form-field-wrapper">
                            <ng-content></ng-content>
                            <span [id]="param" (click)="handleClick($event)">Label</span>
                        </div>
                    `,
                    standalone: true
                })
                export class MatFormField {
                    @ViewChild('input') _input: ElementRef;
                    param = 'test';

                    // Signal queries
                    readonly label = contentChild<string>('label');
                    readonly internalInput = viewChild.required<ElementRef>('input');

                    // Computed signal
                    readonly hasLabel = computed(() => !!this.label());

                    handleClick(event: Event) {
                        console.log(event);
                    }
                }
            "#,
        )]);
        let fs_arc = Arc::new(fs);

        let options = NgCompilerOptions {
            project: ".".to_string(),
            strict_injection_parameters: true,
            strict_templates: true,
            skip_template_codegen: false,
            flat_module_out_file: None,
            out_dir: Some("/dist".to_string()),
            root_dir: Some("/".to_string()),
        };

        let ticket = CompilationTicket {
            kind: CompilationTicketKind::Fresh,
            options,
            fs: &*fs_arc,
        };

        let mut compiler = NgCompiler::new(ticket);
        let result = compiler
            .analyze_async(&["/app.component.ts".to_string()])
            .expect("Analysis failed");

        // Emit
        let _ = compiler.emit(&result).expect("Emit failed");

        // Read output
        let output = fs_arc
            .read_file(&crate::ngtsc::file_system::AbsoluteFsPath::from(
                "/dist/app.component.js",
            ))
            .expect("Output file not found");

        println!("=== GENERATED OUTPUT ===");
        println!("{}", output);
        println!("========================");

        // Assertions for parity checking (adjust assertions as we fix things)
        // 1. Check for IIFE wrapper around ɵcmp (we suspect it exists and want to remove it)
        // The user says Rust wraps it: static ɵcmp = (function() { ... })();
        // We want: static ɵcmp = ɵɵdefineComponent(...);

        // fail if we see IIFE wrapping pattern
        // Regex or simple string check?
        // Let's just print it for now to debug.
    }
}
