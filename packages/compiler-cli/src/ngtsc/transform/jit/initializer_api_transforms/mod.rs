// Initializer API Transforms
//
// This module provides transforms for Angular's initializer-based APIs
// (signal inputs, outputs, queries, models) to work with JIT compilation.

pub mod transform_api;
pub mod transform;
pub mod input_function;
pub mod output_function;
pub mod query_functions;
pub mod model_function;

pub use transform::get_initializer_api_jit_transform;
pub use transform_api::{PropertyTransform, PropertyTransformResult, PropertyInfo, SyntheticDecorator};
pub use input_function::signal_inputs_transform;
pub use output_function::initializer_api_output_transform;
pub use query_functions::query_functions_transforms;
pub use model_function::signal_model_transform;
