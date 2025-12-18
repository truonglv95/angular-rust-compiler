use crate::ngtsc::file_system::{AbsoluteFsPath, FileSystem};
use std::collections::{HashMap, HashSet, VecDeque};
use std::cell::RefCell;
use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::SourceType;

pub struct ImportGraph<'a> {
    fs: &'a dyn FileSystem,
    imports: RefCell<HashMap<AbsoluteFsPath, HashSet<AbsoluteFsPath>>>,
}

impl<'a> ImportGraph<'a> {
    pub fn new(fs: &'a dyn FileSystem) -> Self {
        Self {
            fs,
            imports: RefCell::new(HashMap::new()),
        }
    }

    pub fn imports_of(&self, sf: &AbsoluteFsPath) -> HashSet<AbsoluteFsPath> {
        let mut cache = self.imports.borrow_mut();
        if let Some(imports) = cache.get(sf) {
             return imports.clone();
        }

        let imports = self.scan_imports(sf);
        cache.insert(sf.clone(), imports.clone());
        imports
    }
    
    fn scan_imports(&self, sf: &AbsoluteFsPath) -> HashSet<AbsoluteFsPath> {
        let mut imports = HashSet::new();
        let content = match self.fs.read_file(sf) {
             Ok(c) => c,
             Err(_) => return imports, // Should probably log or error, but for now empty imports
        };

        let allocator = Allocator::default();
        let source_type = SourceType::from_path(sf.as_path()).unwrap_or_default().with_typescript(true);
        let ret = Parser::new(&allocator, &content, source_type).parse();

        // Very basic resolution logic: assumes .ts extension and relative paths
        // Use dirname from FileSystem/PathManipulation
        let parent_str = self.fs.dirname(sf.as_str());
        let parent = AbsoluteFsPath::new(parent_str);


        for stmt in ret.program.body {
            let module_specifier = match stmt {
                oxc_ast::ast::Statement::ImportDeclaration(decl) => {
                    if decl.import_kind.is_value() {
                        Some(decl.source.value.to_string())
                    } else {
                        None
                    }
                },
                oxc_ast::ast::Statement::ExportNamedDeclaration(decl) => {
                    if decl.export_kind.is_value() {
                        decl.source.as_ref().map(|s| s.value.to_string())
                    } else {
                        None
                    }
                },
                oxc_ast::ast::Statement::ExportAllDeclaration(decl) => {
                     if decl.export_kind.is_value() {
                        Some(decl.source.value.to_string())
                    } else {
                        None
                    }
                }
                _ => None
            };

            if let Some(specifier) = module_specifier {
                 if specifier.starts_with(".") {
                     // Resolve relative path
                     // TODO: Better resolution (handling .d.ts, index.ts, extensions)
                     // For tests "a:b" maps to "./b" importing "b.ts"
                     let resolved_str = self.fs.join(parent.as_str(), &[&specifier]);
                     let resolved_str = self.fs.normalize(&resolved_str);

                     // Try adding .ts if missing

                     let resolved = if !resolved_str.ends_with(".ts") {
                         AbsoluteFsPath::from(format!("{}.ts", resolved_str))
                     } else {
                         AbsoluteFsPath::from(resolved_str)
                     };
                     imports.insert(resolved);
                 }
            }


        }
        
        imports
    }


    /// Find an import path from the `start` SourceFile to the `end` SourceFile.
    ///
    /// This function implements a breadth first search that results in finding the
    /// shortest path between the `start` and `end` points.
    pub fn find_path(&self, start: &AbsoluteFsPath, end: &AbsoluteFsPath) -> Option<Vec<AbsoluteFsPath>> {
        if start == end {
            return Some(vec![start.clone()]);
        }

        let mut found = HashSet::new();
        found.insert(start.clone());
        
        // Queue stores (current_node, parent_node) to reconstruct path
        // We need a way to look up parents. A map is better than storing in queue if we want to trace back.
        let mut parents: HashMap<AbsoluteFsPath, AbsoluteFsPath> = HashMap::new();
        let mut queue = VecDeque::new();
        queue.push_back(start.clone());

        while let Some(current) = queue.pop_front() {
            let imports = self.imports_of(&current);
            for imported_file in imports {
                if !found.contains(&imported_file) {
                    parents.insert(imported_file.clone(), current.clone());
                    
                    if &imported_file == end {
                        // Reconstruct path
                        let mut path = Vec::new();
                        let mut curr = Some(imported_file.clone());
                        while let Some(c) = curr {
                            path.push(c.clone());
                            curr = parents.get(&c).cloned();
                        }
                        path.reverse();
                        return Some(path);
                    }
                    
                    found.insert(imported_file.clone());
                    queue.push_back(imported_file);
                }
            }
        }
        None
    }

    /// Add a record of an import from `sf` to `imported`, that's not present in the original
    /// `ts.Program` but will be remembered by the `ImportGraph`.
    pub fn add_synthetic_import(&self, sf: &AbsoluteFsPath, imported: &AbsoluteFsPath) {
        let mut cache = self.imports.borrow_mut();
        cache.entry(sf.clone()).or_default().insert(imported.clone());
    }
}

