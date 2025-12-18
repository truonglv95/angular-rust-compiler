use crate::ngtsc::cycles::src::imports::ImportGraph;
use crate::ngtsc::file_system::AbsoluteFsPath;
use std::collections::HashMap;
use std::cell::RefCell;

pub struct CycleAnalyzer<'a> {
    import_graph: &'a ImportGraph<'a>,
    cached_results: RefCell<Option<CycleResults<'a>>>,
}

impl<'a> CycleAnalyzer<'a> {
    pub fn new(import_graph: &'a ImportGraph<'a>) -> Self {
        Self { 
            import_graph,
            cached_results: RefCell::new(None),
        }
    }

    pub fn would_create_cycle(&self, from: &AbsoluteFsPath, to: &AbsoluteFsPath) -> Option<Cycle> {
        // Try to reuse the cached results as long as the `from` source file is the same.
        let mut cache = self.cached_results.borrow_mut();
        
        // Check if we need to invalidate or create new cache
        let reset_cache = if let Some(results) = &*cache {
            &results.from != from
        } else {
            true
        };

        if reset_cache {
            *cache = Some(CycleResults::new(from.clone(), self.import_graph));
        }

        // Import of 'from' -> 'to' is illegal if an edge 'to' -> 'from' already exists.
        if let Some(results) = &mut *cache {
            if results.would_be_cyclic(to) {
                return Some(Cycle::new(
                    self.import_graph,
                    from.clone(),
                    to.clone(),
                ));
            }
        }
        
        None
    }

    pub fn record_synthetic_import(&self, from: &AbsoluteFsPath, to: &AbsoluteFsPath) {
        self.cached_results.replace(None);
        self.import_graph.add_synthetic_import(from, to);
    }
}

pub struct Cycle {
    pub from: AbsoluteFsPath,
    pub to: AbsoluteFsPath,
    path: Vec<AbsoluteFsPath>,
}

impl Cycle {
    pub fn new<'a>(import_graph: &'a ImportGraph<'a>, from: AbsoluteFsPath, to: AbsoluteFsPath) -> Self {
        let path = vec![from.clone()]
            .into_iter()
            .chain(import_graph.find_path(&to, &from).unwrap_or_default().into_iter())
            .collect();
            
        Self { from, to, path }
    }
    
    pub fn get_path(&self) -> &Vec<AbsoluteFsPath> {
        &self.path
    }
}

#[derive(Clone, Copy, PartialEq)]
enum CycleState {
    Cyclic,
    Acyclic,
}

struct CycleResults<'a> {
    from: AbsoluteFsPath,
    import_graph: &'a ImportGraph<'a>,
    results: HashMap<AbsoluteFsPath, CycleState>,
}

impl<'a> CycleResults<'a> {
    fn new(from: AbsoluteFsPath, import_graph: &'a ImportGraph<'a>) -> Self {
        Self {
            from,
            import_graph,
            results: HashMap::new(),
        }
    }

    fn would_be_cyclic(&mut self, sf: &AbsoluteFsPath) -> bool {
        if let Some(&state) = self.results.get(sf) {
             return state == CycleState::Cyclic;
        }

        if sf == &self.from {
             return true;
        }

        // Assume for now that the file will be acyclic; this prevents infinite recursion
        self.results.insert(sf.clone(), CycleState::Acyclic);

        let imports = self.import_graph.imports_of(sf);
        for imported in imports {
            if self.would_be_cyclic(&imported) {
                self.results.insert(sf.clone(), CycleState::Cyclic);
                return true;
            }
        }

        false
    }
}

pub enum CycleHandlingStrategy {
    UseRemoteScoping,
    Error,
}

