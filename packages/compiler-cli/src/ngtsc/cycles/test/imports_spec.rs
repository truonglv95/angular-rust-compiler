use crate::ngtsc::cycles::{CycleAnalyzer, ImportGraph};
use crate::ngtsc::file_system::{PathManipulation};
use super::util::{create_fs_from_graph, import_path_to_string};

#[test]
fn test_type_only_imports() {
    // "a:b,c!;b;c" -> c is type-only import in a
    let fs = create_fs_from_graph("a:b,c!;b;c");
    let graph = ImportGraph::new(&fs);
    let analyzer = CycleAnalyzer::new(&graph);
    
    let a = fs.resolve(&["/a.ts"]);
    let b = fs.resolve(&["/b.ts"]);
    let c = fs.resolve(&["/c.ts"]);
    
    // c -> a check
    // If a imports c (value), then c->a would be a cycle.
    // But a imports c (type), so no cycle.
    assert!(analyzer.would_create_cycle(&c, &a).is_none());
    
    // b -> a check
    // a imports b (value). b->a would be a cycle.
    let cycle = analyzer.would_create_cycle(&b, &a);
    assert!(cycle.is_some());
    assert_eq!(import_path_to_string(&fs, cycle.unwrap().get_path()), "b,a,b");
}

#[test]
fn test_imports_of() {
    let fs = create_fs_from_graph("a:b;b:c;c");
    let graph = ImportGraph::new(&fs);
    
    let a = fs.resolve(&["/a.ts"]);
    let b = fs.resolve(&["/b.ts"]);
    
    let imports_a = graph.imports_of(&a);
    assert_eq!(imports_a.len(), 1);
    assert!(imports_a.contains(&b));

    let imports_b = graph.imports_of(&b);
    assert_eq!(imports_b.len(), 1);
    let c = fs.resolve(&["/c.ts"]);
    assert!(imports_b.contains(&c));
}

#[test]
fn test_find_path() {
     // a:*b,*c;b:*e,*f;c:*g,*h;e:f;f;g:e;h:g
    let fs = create_fs_from_graph("a:*b,*c;b:*e,*f;c:*g,*h;e:f;f;g:e;h:g");
    let graph = ImportGraph::new(&fs);

    let a = fs.resolve(&["/a.ts"]);
    let b = fs.resolve(&["/b.ts"]);
    let c = fs.resolve(&["/c.ts"]);
    let e = fs.resolve(&["/e.ts"]);

    // a -> a
    let path = graph.find_path(&a, &a);
    assert!(path.is_some());
    assert_eq!(import_path_to_string(&fs, &path.unwrap()), "a");

    // a -> b
    let path = graph.find_path(&a, &b);
    assert!(path.is_some());
    assert_eq!(import_path_to_string(&fs, &path.unwrap()), "a,b");
    
    // c -> e (c -> g -> e)
    let path = graph.find_path(&c, &e);
    assert!(path.is_some());
    assert_eq!(import_path_to_string(&fs, &path.unwrap()), "c,g,e");

    // e -> c (no path)
    assert!(graph.find_path(&e, &c).is_none());
    
    // b -> c (no path)
    assert!(graph.find_path(&b, &c).is_none());
}

#[test]
fn test_find_path_circular() {
    // a -> b -> c -> d
    // ^----/    |
    // ^---------/
    let fs = create_fs_from_graph("a:b;b:a,c;c:a,d;d");
    let graph = ImportGraph::new(&fs);
    
    let a = fs.resolve(&["/a.ts"]);
    let d = fs.resolve(&["/d.ts"]);
    
    let path = graph.find_path(&a, &d);
    assert!(path.is_some());
    assert_eq!(import_path_to_string(&fs, &path.unwrap()), "a,b,c,d");
}
