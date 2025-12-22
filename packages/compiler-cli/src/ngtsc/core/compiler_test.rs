
#[cfg(test)]
mod tests {
    use crate::ngtsc::core::{CompilationTicket, CompilationTicketKind, NgCompiler, NgCompilerOptions};
    use crate::ngtsc::file_system::testing::MockFileSystem;
    use crate::ngtsc::file_system::FileSystem;
    use std::sync::Arc;
    use angular_compiler::ml_parser::ast::Node;

    #[test]
    fn test_analyze_async_parses_template() {
        let fs = MockFileSystem::new_posix();
        fs.init_with_files(vec![
            ("/app.component.ts", r#"
                import { Component } from '@angular/core';

                @Component({
                    selector: 'app-root',
                    template: '<h1>Hello World</h1>',
                    standalone: true
                })
                export class AppComponent {}
            "#)
        ]);
        let fs_arc = Arc::new(fs);
        
        let options = NgCompilerOptions {
            project: ".".to_string(),
            strict_injection_parameters: true,
            strict_templates: true,
            skip_template_codegen: false,
            flat_module_out_file: None,
            out_dir: None,
        };

        let ticket = CompilationTicket {
            kind: CompilationTicketKind::Fresh,
            options,
            fs: &*fs_arc, // Deref Arc to get reference to MockFileSystem
        };

        let compiler = NgCompiler::new(ticket);
        let result = compiler.analyze_async(&["/app.component.ts".to_string()]).expect("Analysis failed");

        assert_eq!(result.directives.len(), 1);
        let meta = &result.directives[0];
        
        assert_eq!(meta.name, "AppComponent");
        assert_eq!(meta.template, Some("<h1>Hello World</h1>".to_string()));
        
        assert!(meta.template_ast.is_some(), "Template AST should be present");
        let ast = meta.template_ast.as_ref().unwrap();
        assert_eq!(ast.len(), 1);
        
        if let Node::Element(el) = &ast[0] {
            assert_eq!(el.name, "h1");
            assert_eq!(el.children.len(), 1);
            if let Node::Text(text) = &el.children[0] {
                assert_eq!(text.value, "Hello World");
            } else {
                panic!("Expected Text node child of h1");
            }
        } else {
            panic!("Expected Element node as root");
        }
    }

    #[test]
    fn test_analyze_async_parses_template_url() {
        let fs = MockFileSystem::new_posix();
        fs.init_with_files(vec![
            ("/app.component.ts", r#"
                import { Component } from '@angular/core';

                @Component({
                    selector: 'app-root',
                    templateUrl: './app.component.html',
                    standalone: true
                })
                export class AppComponent {}
            "#),
            ("/app.component.html", "<h1>Hello World External</h1>")
        ]);
        let fs_arc = Arc::new(fs);
        
        let options = NgCompilerOptions {
            project: ".".to_string(),
            strict_injection_parameters: true,
            strict_templates: true,
            skip_template_codegen: false,
            flat_module_out_file: None,
            out_dir: None,
        };

        let ticket = CompilationTicket {
            kind: CompilationTicketKind::Fresh,
            options,
            fs: &*fs_arc,
        };

        let compiler = NgCompiler::new(ticket);
        let result = compiler.analyze_async(&["/app.component.ts".to_string()]);
        
        assert!(result.is_ok());
        let res = result.unwrap();
        assert_eq!(res.directives.len(), 1);
        let directive = &res.directives[0];
        assert_eq!(directive.name, "AppComponent");
        assert!(directive.template_ast.is_some(), "Template AST should be populated from templateUrl");
        
        let ast = directive.template_ast.as_ref().unwrap();
        if let Node::Element(el) = &ast[0] {
             assert_eq!(el.name, "h1");
             if let Node::Text(text) = &el.children[0] {
                 assert_eq!(text.value, "Hello World External");
             }
        }
    }

    #[test]
    fn test_analyze_async_parses_style_urls() {
        let fs = MockFileSystem::new_posix();
        fs.init_with_files(vec![
            ("/app.component.ts", r#"
                import { Component } from '@angular/core';

                @Component({
                    selector: 'app-root',
                    template: '',
                    styleUrls: ['./app.component.css'],
                    standalone: true
                })
                export class AppComponent {}
            "#),
            ("/app.component.css", "h1 { color: red; }")
        ]);
        let fs_arc = Arc::new(fs);
        
        let options = NgCompilerOptions {
            project: ".".to_string(),
            strict_injection_parameters: true,
            strict_templates: true,
            skip_template_codegen: false,
            flat_module_out_file: None,
            out_dir: None,
        };

        let ticket = CompilationTicket {
            kind: CompilationTicketKind::Fresh,
            options,
            fs: &*fs_arc,
        };

        let compiler = NgCompiler::new(ticket);
        let result = compiler.analyze_async(&["/app.component.ts".to_string()]);
        
        assert!(result.is_ok());
        let res = result.unwrap();
        assert_eq!(res.directives.len(), 1);
        let directive = &res.directives[0];
        assert_eq!(directive.name, "AppComponent");
        
        assert!(directive.styles.is_some());
        let styles = directive.styles.as_ref().unwrap();
        assert_eq!(styles.len(), 1);
        assert_eq!(styles[0], "h1 { color: red; }");
    }
}
