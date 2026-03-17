use angular_compiler_cli::{compile_ivy, CompilerOptions, LinkerOptions};
use async_trait::async_trait;
use rolldown_common::ModuleType;
use rolldown_plugin::{
    HookLoadArgs, HookLoadOutput, HookLoadReturn, HookResolveIdArgs, HookResolveIdOutput,
    HookResolveIdReturn, HookTransformArgs, HookTransformReturn, Plugin, PluginContext,
};
use std::path::Path;
use std::sync::Arc;

/// A Rolldown Native Rust Plugin that integrates the Angular Rust Compiler.
///
/// This avoids the overhead of V8 string serialization and NAPI calls,
/// as Vite (with Rolldown) will feed AST or Source code directly into Rust memory.
#[derive(Debug)]
pub struct AngularRustCompilerPlugin {
    pub options: Arc<CompilerOptions>,
}

impl AngularRustCompilerPlugin {
    pub fn new(opts: CompilerOptions) -> Self {
        Self {
            options: Arc::new(opts),
        }
    }
}

#[async_trait]
impl Plugin for AngularRustCompilerPlugin {
    fn name(&self) -> std::borrow::Cow<'static, str> {
        "angular-rust-compiler".into()
    }

    /// Hook for custom resolving (e.g. dynamic components) if we intercept virtual CSS/HTML
    async fn resolve_id(
        &self,
        _ctx: &PluginContext,
        _args: &HookResolveIdArgs<'_>,
    ) -> HookResolveIdReturn {
        Ok(None)
    }

    /// Hook to load the content of files if we need to manually read HTML/CSS like the Vite JS plugin
    async fn load(&self, _ctx: &PluginContext, _args: &HookLoadArgs<'_>) -> HookLoadReturn {
        Ok(None)
    }

    /// Transform hook runs natively in Rust!
    /// `args.code` is a &str containing the file contents directly inside the Rolldown Threadpool.
    async fn transform(
        &self,
        _ctx: &PluginContext,
        args: &HookTransformArgs<'_>,
    ) -> HookTransformReturn {
        let is_angular_file = args.id.ends_with(".ts") || args.id.ends_with(".js");
        if !is_angular_file || args.id.contains("node_modules") {
            return Ok(None);
        }

        let filepath = args.id;
        let source_code = args.code;

        // In a real implementation we would call the actual `compile_ivy` or `linker` logic here
        // Example (pseudo-code depending on exact library signatures of angular-compiler-cli):
        /*
        let compiled_result = compile_ivy(
            source_code,
            filepath,
            &self.options
        );
        */

        // Here we just return None to pass through.
        // Once `angular_compiler` exports a synchronous/asynchronous `transform` API
        // taking `source_code: &str`, we'll return the transformed code and sourcemaps here.
        println!(
            "🚀 [Native Rolldown Plugin] Fast-path Transforming: {}",
            filepath
        );

        Ok(None)
    }
}
