use crate::ngtsc::file_system::{AbsoluteFsPath, FileSystem, PathManipulation};
use crate::ngtsc::file_system::testing::MockFileSystem;

pub fn create_fs_from_graph(graph: &str) -> MockFileSystem {
    let fs = MockFileSystem::new_native(); // Use native style paths for simplicity
    
    // Graph string format: "a:b,c;b"
    // "file:imports"
    for segment in graph.split(';') {
        let parts: Vec<&str> = segment.split(':').collect();
        let name = parts[0];
        let deps = if parts.len() > 1 { parts[1] } else { "" };
        
        let mut content = String::new();
        if !deps.is_empty() {
            for dep in deps.split(',') {
                let is_type_only = dep.ends_with('!');
                let dep_clean = dep.trim_end_matches('!'); 
                if dep_clean.starts_with('*') {
                     let sym = &dep_clean[1..];
                     let export_kw = if is_type_only { "export type" } else { "export" };
                     content.push_str(&format!("{} {{{}}} from './{}';\n", export_kw, sym, sym));
                } else {
                     let import_kw = if is_type_only { "import type" } else { "import" };
                     content.push_str(&format!("{} {{{}}} from './{}';\n", import_kw, dep_clean, dep_clean));
                }
            }
        }
        
        let sf_name = format!("/{}.ts", name);
        fs.write_file(&AbsoluteFsPath::new(sf_name), content.as_bytes(), None).unwrap();

    }
    
    fs
}


pub fn import_path_to_string(fs: &dyn FileSystem, path: &[AbsoluteFsPath]) -> String {
    path.iter()
        .map(|p| fs.basename(p.as_str(), Some(".ts")).to_string())
        .collect::<Vec<_>>()
        .join(",")
}
