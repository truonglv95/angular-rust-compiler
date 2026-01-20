/**
 * Vite Plugin for Angular Rust Compiler
 * 
 * ESM Mode: Serves individual compiled files directly from memory.
 * Entry point: src/main.ts (resolved to compiled dist/src/main.js)
 */
import fs from 'fs';
import path from 'path';
import { createRequire } from 'module';

const require = createRequire(import.meta.url);

// Helper function to inject styles and HMR code into main.js
function injectMainPreamble(code, projectRoot, globalStyles) {
    let preamble = `
(function() {
  const originalWarn = console.warn;
  console.warn = function(...args) {
    if (typeof args[0] === 'string' && args[0].includes('NG0912')) return;
    originalWarn.apply(console, args);
  };
})();
`;

    // Inject global styles
    try {
        const configPath = path.resolve(projectRoot, 'angular.json');
        if (fs.existsSync(configPath)) {
            const config = JSON.parse(fs.readFileSync(configPath, 'utf8'));
            const projectKey = Object.keys(config.projects)[0];
            const project = config.projects[projectKey];
            const styles = project?.architect?.build?.options?.styles || [];
            
            styles.forEach(style => {
                let stylePath = typeof style === 'string' ? style : style.input;
                if (stylePath.startsWith('node_modules/')) {
                    let currentDir = projectRoot;
                    let foundPath = null;
                    let depth = 0;
                    while (depth < 10) {
                        const tryPath = path.resolve(currentDir, stylePath);
                        if (fs.existsSync(tryPath)) {
                            foundPath = tryPath;
                            break;
                        }
                        const parent = path.dirname(currentDir);
                        if (parent === currentDir) break;
                        currentDir = parent;
                        depth++;
                    }
                    if (foundPath) {
                        preamble += `import '${foundPath}';\n`;
                    }
                } else {
                    preamble += `import '/${stylePath}';\n`;
                }
            });
        }
    } catch (e) {
        // Ignore style injection errors
    }

    // Add HMR bootstrap wrapper
    if (!code.includes('const __hmrBootstrap')) {
        code = code.replace(/bootstrapApplication\s*\(/, '__hmrBootstrap(');
        code += `
async function __hmrBootstrap(...args) {
  if (window.__ngAppRef) {
    try {
      const ref = await window.__ngAppRef;
      if (ref) {
        console.log('[HMR] Destroying old app...');
        ref.destroy();
      }
    } catch(e) { console.error('[HMR] Cleanup error:', e); }
  }
  
  let root = document.querySelector('app-root');
  if (!root) {
    root = document.createElement('app-root');
    document.body.appendChild(root);
  } else {
    root.innerHTML = '';
  }
  
  const promise = bootstrapApplication(...args);
  window.__ngAppRef = promise;
  return promise;
}

if (import.meta.hot) {
  import.meta.hot.accept();
}
`;
    }

    return preamble + code;
}

export default function angularRustPlugin(options = {}) {
    let bundleCache = null;
    let compiler = null;
    let projectRoot = process.cwd();
    let globalStyles = [];
    let isBundling = false;

    const getBundle = async () => {
        if (bundleCache) return bundleCache;
        if (isBundling) {
            while (isBundling) await new Promise(r => setTimeout(r, 50));
            return bundleCache;
        }

        isBundling = true;
        try {
            if (!compiler) {
                // Default to relative path from plugin location to binding
                const defaultBindingPath = path.resolve(
                    path.dirname(new URL(import.meta.url).pathname),
                    '../../binding/index.js'
                );
                const bindingPath = options.bindingPath || defaultBindingPath;
                // console.log(`[Plugin Debug] Resolved binding path: ${bindingPath}`);
                compiler = require(bindingPath);
                compiler = new compiler.Compiler();
            }

            let configFile = options.configFile || path.resolve(projectRoot, 'angular.json');
            if (!fs.existsSync(configFile)) {
                throw new Error(`Angular config not found: ${configFile}`);
            }
            projectRoot = path.dirname(configFile);

            // console.log(`[rustBundlePlugin] Compiling project...`);
            const startTime = Date.now();
            const result = compiler.bundle(configFile);

            // Stats Output
            // Stats Output
            printBuildStats(result, Date.now() - startTime);

            const files = result.files || {};
            const fileCount = Object.keys(files).length;

            if (fileCount === 0) {
                const bundle = result.bundleJs || result.bundle_js || '';
                if (bundle.startsWith('/* Bundle Error')) {
                    console.error(bundle);
                    throw new Error('Compilation failed');
                }
            }

            bundleCache = result;
            return result;
        } finally {
            isBundling = false;
        }
    };

    return {
        name: 'vite-plugin-angular-rust',
        enforce: 'pre',

        configureServer(server) {
            globalStyles = [];
            try {
                const configPath = path.resolve(projectRoot, 'angular.json');
                if (fs.existsSync(configPath)) {
                    const config = JSON.parse(fs.readFileSync(configPath, 'utf8'));
                    const project = Object.values(config.projects)[0];
                    const styles = project?.architect?.build?.options?.styles || [];
                    globalStyles = styles.map(s => typeof s === 'string' ? s : s.input);
                }
            } catch (e) {
                console.warn('Failed to parse angular.json for styles:', e);
            }
        },

        async handleHotUpdate({ file, server, modules }) {
            if (file.endsWith('.ts') || file.endsWith('.html') || file.endsWith('.css') || file.endsWith('.scss')) {
                // Check if global style changed - needs full reload
                if (globalStyles.some(style => file.endsWith(style))) {
                    server.ws.send({ type: 'full-reload', path: '*' });
                    return [];
                }

                // Incremental compilation
                if (bundleCache) {
                    const relPath = path.relative(projectRoot, file);
                    let targetTsFile = file;

                    // For html/css, find parent .ts (component)
                    if (file.endsWith('.html') || file.endsWith('.css') || file.endsWith('.scss')) {
                        const baseName = file.replace(/\.(html|css|scss)$/, '.ts');
                        if (fs.existsSync(baseName)) {
                            targetTsFile = baseName;
                        }
                    }

                    if (fs.existsSync(targetTsFile)) {
                        const content = fs.readFileSync(targetTsFile, 'utf8');
                        try {
                            const result = compiler.compile(targetTsFile, content);
                            if (result.code && !result.code.includes('/* Error')) {
                                const key = 'dist/' + path.relative(projectRoot, targetTsFile).replace(/\\/g, '/').replace(/\.ts$/, '.js');
                                
                                if (bundleCache.files) {
                                    bundleCache.files[key] = result.code;
                                }

                                const virtualId = '\0' + key;
                                const updatedModules = [];

                                const mod = server.moduleGraph.getModuleById(virtualId);
                                if (mod) {
                                    server.moduleGraph.invalidateModule(mod);
                                    updatedModules.push(mod);
                                }

                                if (updatedModules.length > 0) {
                                    return updatedModules;
                                }
                            }
                        } catch (e) {
                            console.error(`[HMR] Compile error:`, e);
                        }
                    }
                }

                // Full rebuild fallback
                const oldFiles = bundleCache?.files || {};
                bundleCache = null;
                await getBundle();
                const newFiles = bundleCache?.files || {};

                const updatedModules = [];
                Object.keys(newFiles).forEach(key => {
                    if (oldFiles[key] !== newFiles[key]) {
                        const virtualId = '\0' + key;
                        const mod = server.moduleGraph.getModuleById(virtualId);
                        if (mod) {
                            server.moduleGraph.invalidateModule(mod);
                            updatedModules.push(mod);
                        }
                    }
                });

                if (updatedModules.length > 0) {
                    return updatedModules;
                } else {
                    server.ws.send({ type: 'full-reload', path: '*' });
                    return [];
                }
            }
        },

        async resolveId(id, importer) {
            const cleanId = id.split('?')[0];

            // Virtual modules are self-resolving
            if (cleanId.startsWith('\0')) return cleanId;

            if (!bundleCache) await getBundle();

            // Map .ts files to compiled .js in cache
            let resolvedPath = id;

            if (importer && importer.startsWith('\0')) {
                const virtualImporterPath = importer.slice(1);
                const importerDir = path.dirname(path.resolve(projectRoot, virtualImporterPath));
                resolvedPath = path.resolve(importerDir, id);
            } else if (importer) {
                resolvedPath = path.resolve(path.dirname(importer), id);
            } else {
                resolvedPath = path.resolve(projectRoot, id);
            }

            // Get relative key
            let key = path.relative(projectRoot, resolvedPath);
            
            // Handle src/main.ts -> dist/src/main.js
            // Handle src/main.ts -> dist/src/main.js
            // Also handle internal dependencies like ./src/components/... resolved relative to dist/
            let jsKey;
            
            // If resolved against dist/... importer, key will start with dist/
            // If resolved against physical file, key will be src/...
            if (key.startsWith('dist/')) {
                 jsKey = key.endsWith('.js') ? key : key + '.js';
            } else {
                 // Check if it's a TS file we need to compile
                 if (key.endsWith('.ts') || resolvedPath.endsWith('.ts')) {
                     jsKey = 'dist/' + key.replace(/\.ts$/, '.js');
                 }
            }

            if (jsKey) { 
                 // Lazy Compilation: If not in cache but exists on disk, compile it now!
                 if (!bundleCache?.files?.[jsKey]) {
                     // Try to find source file
                     // Reverse map: dist/path/to/file.js -> path/to/file.ts
                     let sourceRelPath = jsKey;
                     if (sourceRelPath.startsWith('dist/')) {
                         sourceRelPath = sourceRelPath.substring(5); // remove dist/
                     }
                     sourceRelPath = sourceRelPath.replace(/\.js$/, '.ts');

                     const sourcePath = path.resolve(projectRoot, sourceRelPath);

                     if (fs.existsSync(sourcePath)) {
                         // Lazy Compile Instrumentation
                         const lazyStartTime = Date.now();
                         try {
                             const content = fs.readFileSync(sourcePath, 'utf8');
                             const result = compiler.compile(sourcePath, content);
                             if (result.code && !result.code.includes('/* Error')) {
                                 if (bundleCache.files) {
                                     bundleCache.files[jsKey] = result.code;
                                 }

                                 // Log Lazy Compile Stats
                                 const durationMs = Date.now() - lazyStartTime;
                                 const durationSeconds = (durationMs / 1000).toFixed(3);
                                 const size = Buffer.byteLength(result.code, 'utf8');
                                 const formattedSize = formatSize(size);
                                 const timestamp = new Date().toISOString();
                                 
                                 // ANSI Colors (reusing logic if possible, or re-defining for safety in this scope)
                                 const env = process.env;
                                 const isColorSupported = !env.NO_COLOR && (env.FORCE_COLOR || (process.stdout.isTTY && env.TERM !== 'dumb'));
                                 const cyan = (s) => isColorSupported ? `\x1b[36m${s}\x1b[39m` : s;
                                 const dim = (s) => isColorSupported ? `\x1b[2m${s}\x1b[22m` : s;
                                 const green = (s) => isColorSupported ? `\x1b[32m${s}\x1b[39m` : s;

                                 console.log(`\nLazy chunk compiled: ${green(path.basename(jsKey))} | ${dim(formattedSize)} | [${cyan(durationSeconds + ' s')}] - ${timestamp}`);
                             }
                         } catch (e) {
                             console.error(`[rustBundlePlugin] Lazy compile failed for ${sourceRelPath}:`, e);
                         }
                     }
                 }

                if (bundleCache?.files?.[jsKey]) {
                    return '\0' + jsKey;
                }
            }

            // Try direct match in files
            if (bundleCache?.files) {
                // Try with dist/ prefix
                const distKey = key.startsWith('dist/') ? key : 'dist/' + key;
                if (bundleCache.files[distKey]) return '\0' + distKey;
                if (bundleCache.files[distKey + '.js']) return '\0' + distKey + '.js';

                // Try exact key
                if (bundleCache.files[key]) return '\0' + key;
                if (bundleCache.files[key + '.js']) return '\0' + key + '.js';
            }

            // Try match in chunks (for lazy loaded modules)
            if (bundleCache?.chunks) {
                // key might be "chunk-name.js" or "dist/chunk-name.js"
                const chunkKey = key.replace(/^dist\//, '');
                if (bundleCache.chunks[chunkKey]) return '\0Chunk:' + chunkKey;
            }

            return null;
        },

        async transform(code, id) {
             // ... existing transform logic ... (omitted for brevity in replacement if unchanged)
             if (!global.transformCount) global.transformCount = 0;
            if (global.transformCount < 100) {
                // console.log('[Vite Transform] ID:', id);
                global.transformCount++;
            }
            if (id.includes('node_modules') && id.includes('@angular')) {
                 // console.log('[Vite Transform] Saw Angular file:', id);
            }
            // Link Angular libraries from node_modules
            if (id.includes('node_modules') && !id.endsWith('.css') && !id.endsWith('.scss')) {
                 if (code.includes('ɵɵngDeclare')) {
                     try {
                        let result = compiler.linkFile(id, code);
                        if (result.startsWith('/* Linker Error')) {
                            console.error(`[Linker] Linker Error for ${id}:`, result);
                            return null;
                        }
                        if (result !== code) {
                            if (id.includes('template-outlet-test.component.ts')) {
                                console.log('[Debugging ReferenceError] Transformed code for:', id);
                                console.log(result);
                            }
                            return `/* LINKED BY RUST LINKER */\n${result}`;
                        }
                    } catch (e) {
                         console.error(`[Linker] Exception for ${id}:`, e);
                    }
                }
            }
            return null;
        },

        async load(id) {
            if (!bundleCache) await getBundle();

            // Handle absolute path .ts files - intercept before Vite's native transform
            if (id.endsWith('.ts') && !id.includes('node_modules') && fs.existsSync(id)) {
                // ... same as before
                const relPath = path.relative(projectRoot, id);
                const jsKey = 'dist/' + relPath.replace(/\.ts$/, '.js');
                
                if (bundleCache?.files?.[jsKey]) {
                    let code = bundleCache.files[jsKey];
                    // For main.js, inject styles and HMR bootstrap
                    if (jsKey.endsWith('main.js')) {
                        code = injectMainPreamble(code, projectRoot, globalStyles);
                    }
                    return { code, map: null };
                }
            }

            if (id.startsWith('\0')) {
                // Check if it's a chunk
                if (id.startsWith('\0Chunk:')) {
                    const chunkKey = id.slice(7); // Remove '\0Chunk:'
                    if (bundleCache?.chunks?.[chunkKey]) {
                         // ANSI Colors
                         const env = process.env;
                         const isColorSupported = !env.NO_COLOR && (env.FORCE_COLOR || (process.stdout.isTTY && env.TERM !== 'dumb'));
                         const cyan = (s) => isColorSupported ? `\x1b[36m${s}\x1b[39m` : s;
                         const dim = (s) => isColorSupported ? `\x1b[2m${s}\x1b[22m` : s;
                         const green = (s) => isColorSupported ? `\x1b[32m${s}\x1b[39m` : s;
                        
                         // Fake duration for served chunks (since they are pre-compiled)
                         // We just log that it's "compiled" (served)
                         const timestamp = new Date().toISOString();
                         const size = Buffer.byteLength(bundleCache.chunks[chunkKey], 'utf8');
                         const formattedSize = formatSize(size);
                         
                         // We mock a small "compile" time to indicate it's served instantly
                         console.log(`\nLazy chunk compiled: ${green(chunkKey)} | ${dim(formattedSize)} | [${cyan('0.001 s')}] - ${timestamp}`);

                         return bundleCache.chunks[chunkKey];
                    }
                }

                const key = id.slice(1);
                
                if (bundleCache?.files?.[key]) {
                    let code = bundleCache.files[key];

                    // For main.js, inject styles and HMR bootstrap
                    if (key.endsWith('main.js')) {
                        let preamble = '';

                        // Suppress NG0912 warnings
                        preamble += `
(function() {
  const originalWarn = console.warn;
  console.warn = function(...args) {
    if (typeof args[0] === 'string' && args[0].includes('NG0912')) return;
    originalWarn.apply(console, args);
  };
})();
`;

                        // Inject global styles
                        try {
                            const configPath = path.resolve(projectRoot, 'angular.json');
                            if (fs.existsSync(configPath)) {
                                const config = JSON.parse(fs.readFileSync(configPath, 'utf8'));
                                const projectKey = Object.keys(config.projects)[0];
                                const project = config.projects[projectKey];
                                const styles = project?.architect?.build?.options?.styles || [];
                                
                                styles.forEach(style => {
                                    let stylePath = typeof style === 'string' ? style : style.input;
                                    if (stylePath.startsWith('node_modules/')) {
                                        let currentDir = projectRoot;
                                        let foundPath = null;
                                        let depth = 0;
                                        while (depth < 10) {
                                            const tryPath = path.resolve(currentDir, stylePath);
                                            if (fs.existsSync(tryPath)) {
                                                foundPath = tryPath;
                                                break;
                                            }
                                            const parent = path.dirname(currentDir);
                                            if (parent === currentDir) break;
                                            currentDir = parent;
                                            depth++;
                                        }
                                        if (foundPath) {
                                            preamble += `import '${foundPath}';\n`;
                                        }
                                    } else {
                                        preamble += `import '/${stylePath}';\n`;
                                    }
                                });
                            }
                        } catch (e) {
                            // Ignore style injection errors
                        }

                        // Add HMR bootstrap wrapper
                        if (!code.includes('const __hmrBootstrap')) {
                            code = code.replace(/bootstrapApplication\s*\(/, '__hmrBootstrap(');
                            code += `
async function __hmrBootstrap(...args) {
  if (window.__ngAppRef) {
    try {
      const ref = await window.__ngAppRef;
      if (ref) {
        console.log('[HMR] Destroying old app...');
        ref.destroy();
      }
    } catch(e) { console.error('[HMR] Cleanup error:', e); }
  }
  
  let root = document.querySelector('app-root');
  if (!root) {
    root = document.createElement('app-root');
    document.body.appendChild(root);
  } else {
    root.innerHTML = '';
  }
  
  const promise = bootstrapApplication(...args);
  window.__ngAppRef = promise;
  return promise;
}

if (import.meta.hot) {
  import.meta.hot.accept();
}
`;
                        }

                        return preamble + code;
                    }

                    return code;
                }
            }

            return null;
        },

        async transformIndexHtml(html) {
            await getBundle();

            // Inject main.ts script if not present
            if (!html.includes('src/main.ts')) {
                const scriptTag = `<script src="/src/main.ts" type="module"></script>`;
                if (html.includes('</body>')) {
                    html = html.replace('</body>', `${scriptTag}\n</body>`);
                } else {
                    html += scriptTag;
                }
            }

            return html;
        },
    };
}
function formatSize(bytes) {
  if (bytes === 0) return "0 B";
  const k = 1000; // Angular CLI uses 1000, not 1024
  const sizes = ["B", "kB", "MB", "GB"];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + " " + sizes[i];
}

function printBuildStats(result, durationMs) {
  // ANSI Color Codes
  const env = process.env;
  const isColorSupported = !env.NO_COLOR && (env.FORCE_COLOR || (process.stdout.isTTY && env.TERM !== 'dumb'));
  
  const bold = (s) => isColorSupported ? `\x1b[1m${s}\x1b[22m` : s;
  const green = (s) => isColorSupported ? `\x1b[32m${s}\x1b[39m` : s;
  const cyan = (s) => isColorSupported ? `\x1b[36m${s}\x1b[39m` : s;
  const dim = (s) => isColorSupported ? `\x1b[2m${s}\x1b[22m` : s;

  const tableHeader = `
${bold('Initial chunk files')} | ${bold('Names')}            |  ${bold('Raw size')}
----------------------------------------------------------------`;

  let output = '\n' + tableHeader + '\n';
  let totalSize = 0;

  const stylesText = result.stylesCss || result.styles_css || '';
  const stylesSize = stylesText ? Buffer.byteLength(stylesText, "utf8") : 0;
  if (stylesSize > 0) {
    output += `${green('styles.css')}${' '.repeat(10)} | ${dim('styles')}           | ${dim(formatSize(stylesSize).padStart(9))} |\n`;
    totalSize += stylesSize;
  }

  const mainText = result.bundleJs || result.bundle_js || '';
  const mainSize = Buffer.byteLength(mainText, "utf8");
  output += `${green('main.js')}${' '.repeat(13)} | ${dim('main')}             | ${dim(formatSize(mainSize).padStart(9))} |\n`;
  totalSize += mainSize;

  output += `\n                    | ${bold('Initial total')}    | ${bold(formatSize(totalSize).padStart(9))}\n\n`;

  output += `${bold('Lazy chunk files')}    | ${bold('Names')}            |  ${bold('Raw size')}\n`;
  const chunks = result.chunks || {};
  const chunkKeys = Object.keys(chunks);

  if (chunkKeys.length > 0) {
    for (const chunkName of chunkKeys) {
      const size = Buffer.byteLength(chunks[chunkName], "utf8");
      let shortName = chunkName.replace(/^chunk-/, "").replace(/\.js$/, "");
      if (shortName.length > 16) shortName = shortName.substring(0, 13) + "...";

      output += `${chunkName.padEnd(19)} | ${dim(shortName.padEnd(16))} | ${dim(formatSize(size).padStart(9))} |\n`;
    }
  }

  const durationSeconds = (durationMs / 1000).toFixed(3);
  const timestamp = new Date().toISOString();

  output += `\nApplication bundle generation complete. [${cyan(durationSeconds + ' seconds')}] - ${timestamp}\n`;

  console.log(output);
}
