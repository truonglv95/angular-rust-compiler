const path = require('path');
// Import the native binding
// In a real setup, this would be node_modules/@angular/rust-binding
// Here we point directly to the built package
const bindingPath = path.resolve(__dirname, '../packages/binding');
const { Compiler } = require(bindingPath);

// Instantiate compiler once (or per request if needed, but reusing is better if stateful)
const compiler = new Compiler();

module.exports = function (source) {
  const callback = this.async();
  const resourcePath = this.resourcePath;

  try {
    // Call the native compile method
    // Note: The binding reads from disk for imports, but we pass the filename
    // and potentially the content (though currently binding ignores content to ensure disk consistency)
    const result = compiler.compile(resourcePath, source);

    // Check if result contains error comment (simple heuristic for this demo)
    if (result.startsWith('/* Error')) {
      return callback(new Error(`Rust Compilation Failed:\n${result}`));
    }

    callback(null, result, null, null);
  } catch (err) {
    callback(err);
  }
};
