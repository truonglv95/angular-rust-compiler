use crate::ngtsc::cycles::{CycleAnalyzer, ImportGraph};
use crate::ngtsc::file_system::{PathManipulation};
use super::util::{create_fs_from_graph, import_path_to_string};

#[test]
fn test_no_cycle() {
    let fs = create_fs_from_graph("a:b,c;b;c");
    let graph = ImportGraph::new(&fs);
    let analyzer = CycleAnalyzer::new(&graph);
    
    let b = fs.resolve(&["/b.ts"]);
    let c = fs.resolve(&["/c.ts"]);
    
    assert!(analyzer.would_create_cycle(&b, &c).is_none());
    assert!(analyzer.would_create_cycle(&c, &b).is_none());
}

#[test]
fn test_simple_cycle() {
    let fs = create_fs_from_graph("a:b;b");
    let graph = ImportGraph::new(&fs);
    let analyzer = CycleAnalyzer::new(&graph);
    
    let a = fs.resolve(&["/a.ts"]);
    let b = fs.resolve(&["/b.ts"]);
    
    assert!(analyzer.would_create_cycle(&a, &b).is_none());
    
    let cycle = analyzer.would_create_cycle(&b, &a);
    assert!(cycle.is_some());
    assert_eq!(import_path_to_string(&fs, cycle.unwrap().get_path()), "b,a,b");
}

#[test]
fn test_complex_cycle() {
    // a -> b -> c -> d
    //      ^---------/
    let fs = create_fs_from_graph("a:b;b:c;c:d;d:b");
    let graph = ImportGraph::new(&fs);
    let analyzer = CycleAnalyzer::new(&graph);
    
    let a = fs.resolve(&["/a.ts"]);
    let b = fs.resolve(&["/b.ts"]);
    let c = fs.resolve(&["/c.ts"]);
    let d = fs.resolve(&["/d.ts"]);
    
    assert!(analyzer.would_create_cycle(&a, &b).is_none());
    assert!(analyzer.would_create_cycle(&a, &c).is_none());
    assert!(analyzer.would_create_cycle(&a, &d).is_none());
    
    assert!(analyzer.would_create_cycle(&b, &a).is_some());
    assert!(analyzer.would_create_cycle(&b, &c).is_some()); 
    assert!(analyzer.would_create_cycle(&b, &d).is_some());
}

#[test]
fn test_reexport_cycle() {
    let fs = create_fs_from_graph("a:*b;b:c;c");
    let graph = ImportGraph::new(&fs);
    let analyzer = CycleAnalyzer::new(&graph);

    let a = fs.resolve(&["/a.ts"]);
    let c = fs.resolve(&["/c.ts"]);
    
    assert!(analyzer.would_create_cycle(&a, &c).is_none());
    
    let cycle = analyzer.would_create_cycle(&c, &a);
    assert!(cycle.is_some());
    assert_eq!(import_path_to_string(&fs, cycle.unwrap().get_path()), "c,a,b,c");
}

#[test]
fn test_synthetic_edge_cycle() {
    let fs = create_fs_from_graph("a:b,c;b;c");
    let graph = ImportGraph::new(&fs);
    let analyzer = CycleAnalyzer::new(&graph);
    
    let b = fs.resolve(&["/b.ts"]);
    let c = fs.resolve(&["/c.ts"]);
    
    assert!(analyzer.would_create_cycle(&b, &c).is_none());
    
    analyzer.record_synthetic_import(&c, &b);
    let cycle = analyzer.would_create_cycle(&b, &c);
    assert!(cycle.is_some());
    assert_eq!(import_path_to_string(&fs, cycle.unwrap().get_path()), "b,c,b");
}

#[test]
fn test_more_complex_cycle() {
    // a:*b,*c;b:*e,*f;c:*g,*h;e:f;f:c;g;h:g
    let fs = create_fs_from_graph("a:*b,*c;b:*e,*f;c:*g,*h;e:f;f:c;g;h:g");
    let graph = ImportGraph::new(&fs);
    let analyzer = CycleAnalyzer::new(&graph);

    let b = fs.resolve(&["/b.ts"]);
    let g = fs.resolve(&["/g.ts"]);
    
    // Check b -> g (no cycle)
    assert!(analyzer.would_create_cycle(&b, &g).is_none());

    // Check g -> b (cycle: g -> b -> f -> c -> g)
    let cycle = analyzer.would_create_cycle(&g, &b);
    assert!(cycle.is_some());
    
    assert_eq!(import_path_to_string(&fs, cycle.unwrap().get_path()), "g,b,f,c,g");
}
