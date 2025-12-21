const path = require("path");

// Resolve path to the compiled NAPI module in the root workspace
const NAPI_PATH = path.resolve(__dirname, "../angular_compiler_cli.node");

let link_file;
try {
  const binding = require(NAPI_PATH);
  link_file = binding.link_file || binding.linkFile;
} catch (e) {
  console.error("[Angular Linker] Failed to load native binding:", e);
  process.exit(1);
}

module.exports = function (source) {
  const filename = this.resourcePath;
  // Link everything that passes through
  try {
    const result = link_file(source, filename);
    return result;
  } catch (e) {
    console.error(`[Angular Linker] Error linking ${filename}:`, e);
    throw e;
  }
};
