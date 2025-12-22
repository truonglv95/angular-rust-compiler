
use crate::ngtsc::core::NgCompilerOptions;
use crate::ngtsc::file_system::{FileSystem, AbsoluteFsPath};
use crate::ngtsc::metadata::{MetadataReader, OxcMetadataReader, DirectiveMetadata};
use std::path::PathBuf;
use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::SourceType;
use angular_compiler::ml_parser::tags::TagDefinition;
use angular_compiler::ml_parser::{parser::Parser as HtmlParser, html_tags::get_html_tag_definition};

fn get_html_tag_definition_wrapper(name: &str) -> &'static dyn TagDefinition {
    get_html_tag_definition(name)
}

pub enum CompilationTicketKind {
    Fresh,
    Incremental,
}

pub struct CompilationTicket<'a, T: FileSystem> {
    pub kind: CompilationTicketKind,
    pub options: NgCompilerOptions,
    pub fs: &'a T,
}

pub struct NgCompiler<'a, T: FileSystem> {
    pub options: NgCompilerOptions,
    pub fs: &'a T,
}

#[derive(Default)]
pub struct CompilationResult {
     pub files: Vec<PathBuf>,
     pub directives: Vec<DirectiveMetadata>,
}

impl<'a, T: FileSystem> NgCompiler<'a, T> {
    pub fn new(ticket: CompilationTicket<'a, T>) -> Self {
        NgCompiler {
            options: ticket.options,
            fs: ticket.fs,
        }
    }

    pub fn analyze_async(&self, root_names: &[String]) -> Result<CompilationResult, String> {
        let mut result = CompilationResult::default();
        let metadata_reader = OxcMetadataReader;
        
        for file in root_names {
            let path = PathBuf::from(file);
            let abs_path = AbsoluteFsPath::from(&path);
            println!("DEBUG: analyze_async processing file: {:?}", abs_path);
            
            let content = match self.fs.read_file(&abs_path) {
                Ok(c) => c,
                Err(e) => {
                     println!("DEBUG: Failed to read file {:?}: {}", abs_path, e);
                     return Err(format!("File not found or unreadable: {:?} ({})", abs_path, e));
                }
            };

            let allocator = Allocator::default();
            let source_type = SourceType::from_path(&path).unwrap_or_default();
            
            let ret = Parser::new(&allocator, &content, source_type).parse();

            if !ret.errors.is_empty() {
                for error in ret.errors {
                    println!("Error parsing {:?}: {:?}", path, error);
                }
                return Err(format!("Failed to parse {:?}", path));
            } else {
                let mut directives = metadata_reader.get_directive_metadata(&ret.program, &path);
                println!("Analyzed {:?} with OXC. Found {} directives.", path, directives.len());
                
                // Parse templates for directives that have inline templates
                for directive in &mut directives {
                    let template_str = if let Some(template) = &directive.template {
                        Some(template.clone())
                    } else if let Some(template_url) = &directive.template_url {
                        let component_dir = self.fs.dirname(abs_path.as_str());
                        let template_path = self.fs.resolve(&[&component_dir, template_url]);
                        match self.fs.read_file(&template_path) {
                            Ok(content) => Some(content),
                            Err(e) => {
                                println!("Failed to read template file at {:?}: {}", template_path, e);
                                None
                            }
                        }
                    } else {
                        None
                    };

                    if let Some(template) = template_str {
                        let parser = HtmlParser::new(get_html_tag_definition_wrapper);
                        let parse_result = parser.parse(&template, "template.html", None);
                        
                        if !parse_result.errors.is_empty() {
                            println!("Errors parsing template for {}: {:?}", directive.name, parse_result.errors);
                        } else {
                            directive.template_ast = Some(parse_result.root_nodes);
                        }
                    }

                    if let Some(style_urls) = &directive.style_urls {
                        let component_dir = self.fs.dirname(abs_path.as_str());
                        let mut resolved_styles = directive.styles.take().unwrap_or_default();
                        
                        for url in style_urls {
                             let style_path = self.fs.resolve(&[&component_dir, url]);
                             match self.fs.read_file(&style_path) {
                                Ok(content) => resolved_styles.push(content),
                                Err(e) => {
                                    println!("Failed to read style file at {:?}: {}", style_path, e);
                                }
                             }
                        }
                        directive.styles = Some(resolved_styles);
                    }
                }

                result.directives.extend(directives);
                result.files.push(path);
            }
        }
        Ok(result)
    }

    pub fn emit(&self, compilation_result: &CompilationResult) -> Result<(), String> {
        let fs = self.fs;
        
        let component_handler = crate::ngtsc::annotations::component::src::handler::ComponentDecoratorHandler::new();
        let directive_handler = crate::ngtsc::annotations::directive::src::handler::DirectiveDecoratorHandler::new(false);
        
        // Track which files have components (they get special handling)
        let mut component_files: std::collections::HashSet<PathBuf> = std::collections::HashSet::new();

        // First pass: emit Angular component files with decorators
        println!("DEBUG: Emit loop starting with {} directives", compilation_result.directives.len());
        for directive in &compilation_result.directives {
             let compiled_results = if directive.is_component {
                 component_handler.compile_ivy(directive)
             } else {
                 directive_handler.compile_ivy(directive)
             };
             println!("DEBUG: Compiled results for {}: {}", directive.name, compiled_results.len());
            
             for result in compiled_results {
                      let initializer = result.initializer.clone().unwrap_or_default();
                      let hoisted_statements = result.statements.join("\n");
                      
                      // Find source file for this directive (using metadata source_file if available)
                      let source_file = directive.source_file.as_ref();
                      if let Some(s) = source_file {
                          println!("DEBUG: Emitter found source file for {}: {:?}", directive.name, s);
                      } else {
                          println!("DEBUG: Emitter NO source file for {}", directive.name);
                      }
                      
                      // Skip if source file is in node_modules or is a spec file
                       if let Some(src_file) = source_file {
                           let src_path = src_file.to_string_lossy();
                           if src_path.contains("node_modules") || src_path.ends_with(".spec.ts") || src_path.ends_with(".d.ts") {
                               continue;
                           }
                       }
                      
                      let out_path = if let Some(src_file) = source_file {
                          if let Some(out_dir) = &self.options.out_dir {
                              // Calculate relative path from project root
                              let project_path = std::path::Path::new(&self.options.project);
                              let project_root = project_path.parent().unwrap_or(std::path::Path::new("."));
                              let absolute_project_root = std::fs::canonicalize(project_root).unwrap_or(project_root.to_path_buf());
                              
                              let absolute_src_file = std::fs::canonicalize(src_file.as_path()).unwrap_or(src_file.as_path().to_path_buf());
                              
                              // Try to strip prefix
                              let relative_path = absolute_src_file.strip_prefix(&absolute_project_root)
                                  .unwrap_or_else(|_| {
                                      // Fallback: just use filename if stripping fails
                                      std::path::Path::new(src_file.file_name().unwrap())
                                  });
                                  
                              let mut p = PathBuf::from(out_dir);
                              p.push(relative_path);
                              p.set_extension("js");
                              
                              // Ensure parent dir exists
                              if let Some(parent) = p.parent() {
                                  let _ = fs.ensure_dir(&AbsoluteFsPath::from(parent));
                              }
                              
                              AbsoluteFsPath::from(p)
                          } else {
                              // Only for safety, shouldn't reach here if out_dir checked above
                              let mut p = PathBuf::from(self.options.out_dir.as_deref().unwrap_or("."));
                              p.push(format!("{}.js", directive.name));
                              AbsoluteFsPath::from(p)
                          }
                      } else {
                          // Fallback if no source file found
                          let mut p = PathBuf::from(self.options.out_dir.as_deref().unwrap_or("."));
                          p.push(format!("{}.js", directive.name));
                          AbsoluteFsPath::from(p)
                      };
                      // ============= AST-BASED TRANSFORMATION =============
                      // 1. Parse source file → OXC AST
                      // 2. Strip TypeScript types (transformer)
                      // 3. Transform AST: remove decorators, add Ivy statements
                      // 4. Codegen → JavaScript output
                      
                      let final_content = if let Some(src_file) = source_file {
                          let source_path = AbsoluteFsPath::from(src_file.as_path());
                          match fs.read_file(&source_path) {
                              Ok(source_content) => {
                                  let allocator = Allocator::default();
                                  let source_type = SourceType::ts();
                                  let file_path = src_file.to_string_lossy().to_string();
                                  let parser = Parser::new(&allocator, &source_content, source_type);
                                  let mut parse_result = parser.parse();
                                  
                                  if !parse_result.errors.is_empty() {
                                      println!("DEBUG: Parser errors for {}: {:?}", directive.name, parse_result.errors);
                                  }
                                  
                                  if parse_result.errors.is_empty() {
                                      // Step 1: Run semantic analysis for scoping
                                      let semantic = oxc_semantic::SemanticBuilder::new()
                                          .with_excess_capacity(0.0)
                                          .build(&parse_result.program);
                                      
                                      // Step 2: Apply TypeScript transformer to strip types
                                      let transform_options = oxc_transformer::TransformOptions::default();
                                      let transformer = oxc_transformer::Transformer::new(
                                          &allocator,
                                          std::path::Path::new(&file_path),
                                          &transform_options,
                                      );
                                      let _ = transformer.build_with_scoping(
                                          semantic.semantic.into_scoping(),
                                          &mut parse_result.program,
                                      );
                                      
                                      // Step 3: AST-based Ivy transformation
                                      // - Remove Angular decorators
                                      // - Add import * as i0 from '@angular/core'
                                      // - Append Ivy statements (hoisted functions + factory + component def)
                                      
                                      // Generate expressions for ɵfac and ɵcmp separately
                                      let fac_expr_str = format!("function {}_Factory(t) {{ return new (t || {})(); }}", directive.name, directive.name);
                                      let cmp_expr_str = format!("/*@__PURE__*/ {}", initializer);
                                      
                                      // Allocate expressions in the arena for lifetime compatibility
                                      let hoisted_statements_arena: &str = allocator.alloc_str(&hoisted_statements);
                                      let fac_expr_arena: &str = allocator.alloc_str(&fac_expr_str);
                                      let cmp_expr_arena: &str = allocator.alloc_str(&cmp_expr_str);
                                      
                                      super::ast_transformer::transform_component_ast(
                                          &allocator,
                                          &mut parse_result.program,
                                          &directive.name,
                                          hoisted_statements_arena,
                                          fac_expr_arena,
                                          cmp_expr_arena,
                                          &result.name,
                                      );
                                      
                                      // Step 4: Codegen final JavaScript
                                      let codegen = oxc_codegen::Codegen::new();
                                      codegen.build(&parse_result.program).code
                                  } else {
                                      // Fallback to empty class if parse fails
                                      format!("import * as i0 from '@angular/core';\nexport class {} {{}}\n{}.ɵfac = function {}_Factory(t) {{ return new (t || {})(); }};\n{}.ɵcmp = /*@__PURE__*/ {};",
                                          directive.name, directive.name, directive.name, directive.name, directive.name, initializer)
                                  }
                              }
                              Err(_) => format!("import * as i0 from '@angular/core';\nexport class {} {{}}\n{}.ɵfac = function {}_Factory(t) {{ return new (t || {})(); }};\n{}.ɵcmp = /*@__PURE__*/ {};",
                                  directive.name, directive.name, directive.name, directive.name, directive.name, initializer)
                          }
                       } else {
                           // This else matches "if let Some(src_file) = source_file"
                           format!("import * as i0 from '@angular/core';\nexport class {} {{}}\n{}.ɵfac = function {}_Factory(t) {{ return new (t || {})(); }};\n{}.ɵcmp = /*@__PURE__*/ {};",
                               directive.name, directive.name, directive.name, directive.name, directive.name, initializer)
                       };

                       match fs.write_file(&out_path, final_content.as_bytes(), None) {
                           Ok(_) => {},
                           Err(e) => println!("Failed to emit {:?}: {}", out_path, e),
                       }

                       // Mark this source file as having a component
                       for file in &compilation_result.files {
                           let mut is_match = false;
                           
                           // Check by exact path match
                           if let Some(src_file) = source_file {
                               if file == src_file.as_path() {
                                   is_match = true;
                               }
                           }
                           
                           // Check by name match (handling kebab-case vs PascalCase)
                           if !is_match {
                               if let Some(stem) = file.file_stem() {
                                   let stem_str = stem.to_string_lossy();
                                   // Simple check: removing hyphens should match directive name (case-insensitive)
                                   let clean_stem = stem_str.replace("-", "");
                                   if clean_stem.eq_ignore_ascii_case(&directive.name) {
                                       is_match = true;
                                   } else if stem_str.eq_ignore_ascii_case("app") {
                                       // Special case for app
                                       is_match = true;
                                   }
                               }
                           }

                           if is_match {
                               component_files.insert(file.clone());
                           }
                       }

             }
        }
        
        // Second pass: transpile non-component TypeScript files
        for file in &compilation_result.files {
            // Skip files in node_modules, spec files, and declaration files
            let src_path = file.to_string_lossy();
            if src_path.contains("node_modules") || src_path.ends_with(".spec.ts") || src_path.ends_with(".d.ts") {
                continue;
            }

            // Skip files that have components (already emitted)
            if component_files.contains(file) {
                continue;
            }
            
            if let Some(out_dir) = &self.options.out_dir {
                // Calculate output path preserving directory structure
                let project_path = std::path::Path::new(&self.options.project);
                let project_root = project_path.parent().unwrap_or(std::path::Path::new("."));
                let absolute_project_root = std::fs::canonicalize(project_root).unwrap_or(project_root.to_path_buf());
                
                let absolute_src_file = std::fs::canonicalize(file.as_path()).unwrap_or(file.as_path().to_path_buf());
                
                let relative_path = absolute_src_file.strip_prefix(&absolute_project_root)
                    .unwrap_or_else(|_| {
                         std::path::Path::new(file.file_name().unwrap())
                    });
                
                let mut out_path = PathBuf::from(out_dir);
                out_path.push(relative_path);
                out_path.set_extension("js");
                
                // Ensure parent dir exists
                if let Some(parent) = out_path.parent() {
                    let _ = fs.ensure_dir(&AbsoluteFsPath::from(parent));
                }
                
                // Read the source file
                let source_path = AbsoluteFsPath::from(file.as_path());
                match fs.read_file(&source_path) {
                    Ok(source_content) => {
                        // Transpilation: parse TypeScript, transform to strip types, then codegen
                        let allocator = Allocator::default();
                        let source_type = SourceType::ts();
                        let file_path = file.to_string_lossy().to_string();
                        let parser = Parser::new(&allocator, &source_content, source_type);
                        let mut parse_result = parser.parse();
                        
                        if !parse_result.errors.is_empty() {
                            println!("Parse errors in {:?}, skipping", file);
                            continue;
                        }
                        
                        // Run semantic analysis to get scoping information
                        let semantic = oxc_semantic::SemanticBuilder::new()
                            .with_excess_capacity(0.0)
                            .build(&parse_result.program);
                        
                        // Apply TypeScript transformer to strip types
                        let transform_options = oxc_transformer::TransformOptions::default();
                        let transformer = oxc_transformer::Transformer::new(
                            &allocator,
                            std::path::Path::new(&file_path),
                            &transform_options,
                        );
                        let _ = transformer.build_with_scoping(
                            semantic.semantic.into_scoping(),
                            &mut parse_result.program,
                        );
                        
                        // Use OXC codegen to emit JavaScript without types
                        let codegen = oxc_codegen::Codegen::new();
                        let js_output = codegen.build(&parse_result.program).code;
                        
                        let out_path_abs = AbsoluteFsPath::from(out_path.as_path());
                        match fs.write_file(&out_path_abs, js_output.as_bytes(), None) {
                            Ok(_) => println!("Transpiled: {:?}", out_path_abs),
                            Err(e) => println!("Failed to transpile {:?}: {}", out_path_abs, e),
                        }
                    }
                    Err(e) => {
                        println!("Failed to read {:?}: {}", file, e);
                    }
                }
            }
        }
        
        Ok(())
    }
}

/// Strip Angular decorators (@Component, @Directive, @Injectable, etc.) from transpiled code
fn strip_angular_decorator(code: &str) -> String {
    // Pattern to match: export @Decorator({...}) class ClassName
    // Should produce: export class ClassName
    let mut result = code.to_string();
    
    // List of Angular decorators to strip
    let decorators = ["@Component", "@Directive", "@Injectable", "@Pipe", "@NgModule"];
    
    for decorator in &decorators {
        // Find the decorator and its configuration block
        while let Some(start) = result.find(decorator) {
            // Find the opening parenthesis
            if let Some(paren_start) = result[start..].find('(') {
                let paren_start = start + paren_start;
                
                // Count balanced parentheses to find the end
                let mut depth = 0;
                let mut end_pos = paren_start;
                for (i, c) in result[paren_start..].char_indices() {
                    match c {
                        '(' => depth += 1,
                        ')' => {
                            depth -= 1;
                            if depth == 0 {
                                end_pos = paren_start + i + 1; // include the closing paren
                                break;
                            }
                        }
                        _ => {}
                    }
                }
                
                // Remove the decorator and its config, keeping any whitespace after
                let before = &result[..start];
                let after = &result[end_pos..].trim_start();
                result = format!("{}{}", before, after);
            } else {
                break;
            }
        }
    }
    
    result
}

/// Extract import statements from code and return them separately
fn extract_and_remove_imports(code: &str) -> (Vec<String>, String) {
    let mut imports = Vec::new();
    let mut remaining_lines = Vec::new();
    
    for line in code.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("import ") {
            imports.push(line.to_string());
        } else {
            remaining_lines.push(line);
        }
    }
    
    (imports, remaining_lines.join("\n"))
}
