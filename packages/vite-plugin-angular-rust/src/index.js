/**
 * Vite Plugin for Angular Rust Compiler
 * 
 * ESM Mode: Serves individual compiled files directly from memory.
 * Entry point: src/main.ts (resolved to compiled dist/src/main.js)
 */
import fs from 'node:fs';
import path from 'node:path';
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

        preamble += `
let hmrStoredArgs = null;

async function __hmrBootstrap(...args) {
  hmrStoredArgs = args;
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

window.__hmrReload = async () => {
  if (hmrStoredArgs) {
    console.log('[HMR] Re-bootstrapping app...');
    await __hmrBootstrap(...hmrStoredArgs);
  }
};

if (import.meta.hot) {
  import.meta.hot.accept();
}
`;
    }

    return preamble + code;
}

/**
 * Rewrite Angular/RxJS imports from bare specifiers to Vite's optimized deps paths.
 * This ensures lazy-loaded chunks use the same module instances as the main bundle.
 * 
 * Input patterns (bare specifiers in bundleCache):
 *   '@angular/core' → '/node_modules/.vite/deps/@angular_core.js'
 *   '@angular/material/checkbox' → '/node_modules/.vite/deps/@angular_material_checkbox.js'
 *   'rxjs' → '/node_modules/.vite/deps/rxjs.js'
 */
function rewriteToViteDeps(code, metadata) {
    // Rewrite Angular bare specifiers: '@angular/<pkg>' or '@angular/<pkg>/<subpkg>'
    code = code.replace(

        /['"]@angular\/([^'"]+)['"]/g,
        (match, suffix) => {
            const pkgName = `@angular/${suffix}`;
            if (metadata?.optimized?.[pkgName]) {
                 const entry = metadata.optimized[pkgName];
                 const hash = metadata.browserHash || entry.fileHash;
                 console.log('[Plugin] Injecting hash for', pkgName, hash);
                 return `'/node_modules/.vite/deps/${entry.file}?ver=${hash}'`;
            }
            console.log('[Plugin] Metadata MISSING for', pkgName);
            // Fallback (risky without hash)
            const parts = suffix.split('/');
            const viteName = `@angular_${parts.join('_')}`;
            return `'/node_modules/.vite/deps/${viteName}.js'`;
        }
    );
    
    // Rewrite RxJS imports
    code = code.replace(
        /['"]rxjs(?:\/([^'"]+))?['"]/g,
        (match, suffix) => {
             const pkgName = suffix ? `rxjs/${suffix}` : 'rxjs';
             if (metadata?.optimized?.[pkgName]) {
                 const entry = metadata.optimized[pkgName];
                 const hash = metadata.browserHash || entry.fileHash;
                 console.log('[Plugin] Injecting hash for', pkgName, hash);
                 return `'/node_modules/.vite/deps/${entry.file}?ver=${hash}'`;
            }
            return match; // fallback to original if not found
        }
    );
    
    return code;
}

export default function angularRustPlugin(options = {}) {
    // Use pre-built bundle from serve.mjs Worker thread if provided
    let bundleCache = options.bundleResult || null;
    let compiler = null;
    let projectRoot = options.configFile && fs.existsSync(options.configFile)
        ? path.dirname(options.configFile)
        : process.cwd();
    let globalStyles = [];
    let isBundling = false;

    /** When true: no full bundle upfront; compile each .ts on-demand when Vite requests it. */
    const lazyCompile = options.lazyCompile === true;
    /** Cache for lazy compile: absolute path -> compiled JS code. */
    const lazyCompileCache = new Map();

    let lastChunkMap = {}; // Cache for handling stale chunk requests during HMR
    let hmrPendingUpdates = new Set(); // Track components with pending HMR updates

    /** Init compiler and projectRoot from config (no full bundle). Used in lazy mode. */
    function ensureCompiler() {
        if (compiler) return;
        const defaultBindingPath = path.resolve(
            path.dirname(new URL(import.meta.url).pathname),
            '../../binding/index.js'
        );
        const bindingPath = options.bindingPath || defaultBindingPath;
        compiler = require(bindingPath);
        compiler = new compiler.Compiler();
        if (options.configFile && fs.existsSync(options.configFile)) {
            projectRoot = path.dirname(options.configFile);
        }
    }

    const getBundle = async (force = false) => {
        if (bundleCache && !force) return bundleCache;
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
            // Force HMR enabled for dev server bundle
            console.time('RustBundle');
            const result = compiler.bundle(configFile, true);
            console.timeEnd('RustBundle');

            // Stats Output - check for --verbose flag
            const isVerbose = process.argv.includes('--verbose');
            if (!options.skipStats) {
                printBuildStats(result, Date.now() - startTime, { verbose: isVerbose });
            }

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
            bundleCache = result;

            // Save chunk map for HMR fallback
            if (result.chunkNames && Object.keys(result.chunkNames).length > 0) {
                 // chunkNames: { chunkHash: sourcePath }
                 // We want to merge new mappings so we can resolve any previous hash
                 lastChunkMap = { ...lastChunkMap, ...result.chunkNames };
            }

            // Save external imports to cache file for Vite optimization
            const externalImports = result.externalImports || result.external_imports || [];
            if (externalImports.length > 0) {
                const cacheDir = path.resolve(projectRoot, '.angular/cache/rust-compiler');
                if (!fs.existsSync(cacheDir)) {
                    fs.mkdirSync(cacheDir, { recursive: true });
                }
                const cachePath = path.join(cacheDir, 'external-imports.json');
                fs.writeFileSync(cachePath, JSON.stringify(externalImports, null, 2));
                // console.log(`[Plugin] External imports cached (${externalImports.length} packages) → .angular/cache/rust-compiler/external-imports.json`);
            }

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

            // Custom watcher removed - handleHotUpdate is enough if it returns [] or updated modules
            // and we've verified that bundle.js has import.meta.hot.accept()


            // Middleware to serve styles.css as raw CSS (bypassing Vite transformation)
            server.middlewares.use(async (req, res, next) => {
                const url = req.url;
                // Match /styles.css, allow query params but skip ?import
                if ((url === '/styles.css' || url.startsWith('/styles.css?')) && !url.includes('?import')) {
                    try {
                        if (lazyCompile) {
                            // Lazy mode: no pre-built styles; let Vite serve or next()
                            next();
                            return;
                        }
                        if (!bundleCache) await getBundle();
                        const content = bundleCache.stylesCss || bundleCache.styles_css;
                        if (content) {
                            res.setHeader('Content-Type', 'text/css');
                            res.end(content);
                            return;
                        }
                    } catch (e) {
                        console.error('[Middleware] Failed to serve styles.css:', e);
                    }
                }

                // Serve lazy-loaded chunks at root path (like Angular CLI)
                // In lazy mode there are no pre-built chunks; dynamic imports are .ts compiled on-demand
                // Serve lazy-loaded chunks at root path (like Angular CLI)
                // REMOVED: Middleware logic moved to load() hook to allow Vite transformation
                // This fix ensures that imports in chunks (e.g. from @angular/core) are correctly
                // versioned by Vite's import analysis plugin, preventing 504 errors.

                // Handle @ng/component metadata URL for HMR
                // This is called by ɵɵgetReplaceMetadataURL(id, t, import.meta.url) from @angular/core
                // URL format: /@ng/component?c=<encoded_component_id>&t=<timestamp>
                // Vite may prefix with /@id/, so we handle both patterns
                // [REMOVED] @ng/component middleware - moved to resolveId/load hooks
                // to allow Vite to transform imports (e.g. @angular/core -> /@fs/.../node_modules/...)
                /*
                if (url.includes('@ng/component?') || url.includes('@ng/component%3F')) {
                     // ... logic moved to load() ...
                     next();
                     return;
                }
                */

                // polyfills.js is now handled as virtual module via resolveId/load hooks
                // This allows Vite to properly resolve and cache dependencies like zone.js

                // Serve assets from node_modules (rewritten by bundler)
                if (url.startsWith('/__node_modules/') && !url.includes('?import')) {
                     // /__node_modules/pkg/foo.woff -> pkg/foo.woff
                     const relativePath = url.replace(/^\/__node_modules\//, '').split('?')[0];
                     
                     const possiblePaths = [
                         path.resolve(projectRoot, 'node_modules', relativePath),
                         path.resolve(process.cwd(), 'node_modules', relativePath),
                         path.resolve(projectRoot, '../node_modules', relativePath)
                     ];

                     for (const tryPath of possiblePaths) {
                         if (fs.existsSync(tryPath)) {
                             try {
                                 const content = fs.readFileSync(tryPath);
                                 const ext = path.extname(tryPath).toLowerCase();
                                 if (ext === '.woff2') res.setHeader('Content-Type', 'font/woff2');
                                 else if (ext === '.woff') res.setHeader('Content-Type', 'font/woff');
                                 else if (ext === '.ttf') res.setHeader('Content-Type', 'font/ttf');
                                 else if (ext === '.eot') res.setHeader('Content-Type', 'application/vnd.ms-fontobject');
                                 else if (ext === '.svg') res.setHeader('Content-Type', 'image/svg+xml');
                                 else if (ext === '.png') res.setHeader('Content-Type', 'image/png');
                                 else if (ext === '.jpg' || ext === '.jpeg') res.setHeader('Content-Type', 'image/jpeg');
                                 else if (ext === '.gif') res.setHeader('Content-Type', 'image/gif');
                                 
                                 res.end(content);
                                 return;
                             } catch (e) {
                                 console.error('[Middleware] Failed to serve node_modules asset:', e);
                             }
                         }
                     }
                     console.warn(`[Middleware] Asset not found: ${relativePath}`);
                }
                // Handle @ng/component HMR requests (middleware workaround)
                if (req.url && (req.url.includes('@ng/component') || req.url.includes('angular:component'))) {
                     try {
                         // Parse query params
                         // req.url might be full path or relative
                         const urlStr = 'http://localhost' + req.url;
                         const urlObj = new URL(urlStr);
                         const componentId = urlObj.searchParams.get('c');
                         
                         if (componentId) {
                             ensureCompiler();
                             if (!lazyCompile && !bundleCache) await getBundle();
                             
                             const decodedId = decodeURIComponent(componentId);
                             const [filePath, className] = decodedId.split('@');
                             
                             if (compiler) {
                                  // Find original file
                                  let targetTsFile = path.resolve(projectRoot, filePath);
                                  if (!fs.existsSync(targetTsFile)) {
                                      // path.resolve might fail if filePath is relative to src but projected differently? 
                                      // Try removing src/app prefix if needed or rely on projectRoot
                                  }
                                  
                                  if (fs.existsSync(targetTsFile)) {
                                      const content = fs.readFileSync(targetTsFile, 'utf8');
                                      const result = compiler.compile(targetTsFile, content, { 
                                          hmr: true,
                                          hmrId: decodedId, 
                                          rootDir: projectRoot 
                                      });
                                      
                                      if (result.code && !result.code.includes('/* Error')) {
                                          let finalCode = result.code;
                                          
                                          // Transform imports to be browser-compatible
                                          // Resolve relative imports to absolute URL paths
                                          // Use /@id/ prefix for bare imports - this is Vite's convention
                                          
                                          // Only rewrite relative paths so browser can resolve; leave bare
                                          // specifiers (@angular/core, etc.) so Vite resolves once (one URL with ?v=).
                                          finalCode = finalCode.replace(/((?:import|export)\s+[\s\S]*?from\s+['"])([^'"]+)(['"])/g, (match, prefix, specifier, suffix) => {
                                              if (specifier.startsWith('.')) {
                                                  const rawDir = path.dirname(filePath);
                                                  const relativeBase = rawDir.startsWith('/') ? rawDir : '/' + rawDir;
                                                  const resolved = path.posix.join(relativeBase, specifier);
                                                  return `${prefix}${resolved}${suffix}`;
                                              }
                                              return match;
                                          });

                                          res.setHeader('Content-Type', 'application/javascript');
                                          res.end(finalCode);
                                          return;
                                      }
                                  }
                             }
                         }
                     } catch (e) {
                         console.error('[Middleware] HMR Serve Error:', e);
                     }
                }

                next();
            });
        },

        async handleHotUpdate({ file, server, modules }) {
            // Ignore compiler cache or metadata files
            if (file.includes('.angular/cache') || file.endsWith('.json')) {
                return [];
            }


            
            if (file.endsWith('.ts') || file.endsWith('.html') || file.endsWith('.css') || file.endsWith('.scss')) {
                // Check if global style changed - needs full reload
                if (globalStyles.some(style => file.endsWith(style))) {

                    server.ws.send({ type: 'full-reload', path: '*' });
                    return [];
                }

                // Lazy mode: invalidate cache for this file (and parent .ts if html/css) so next request re-compiles
                if (lazyCompile) {
                    const normalizedFile = path.normalize(file);
                    lazyCompileCache.delete(normalizedFile);
                    if (file.endsWith('.html') || file.endsWith('.css') || file.endsWith('.scss')) {
                        const baseTs = file.replace(/\.(html|css|scss)$/, '.ts');
                        if (fs.existsSync(baseTs)) lazyCompileCache.delete(path.normalize(baseTs));
                    }
                    // Let Vite invalidate the module so browser re-requests and we re-compile
                    return modules;
                }

                // Incremental compilation (bundled mode)
                if (bundleCache) {
                    const mainBundleName = bundleCache.bundleName || bundleCache.bundle_name || 'bundle.js';
                    const mainVirtualId = `\0${mainBundleName}`;
                    const relPath = path.relative(projectRoot, file);
                    let targetTsFile = file;

                    // For html/css, find parent .ts (component)
                    if (file.endsWith('.html') || file.endsWith('.css') || file.endsWith('.scss')) {
                        const baseName = file.replace(/\.(html|css|scss)$/, '.ts');

                        if (fs.existsSync(baseName)) {
                            targetTsFile = baseName;

                        } else {

                        }
                    }


                    if (fs.existsSync(targetTsFile)) {
                        const content = fs.readFileSync(targetTsFile, 'utf8');
                        try {

                            const result = compiler.compile(targetTsFile, content, { 
                                hmr: true,
                                root_dir: projectRoot 
                            });

                            
                            if (result.code && !result.code.includes('/* Error')) {
                                fs.appendFileSync('/tmp/debug_hmr.log', '\n[HMR DEBUG] Raw Rust Output:\n' + result.code + '\n-----------------------------------\n');
                                const relTsPath = path.relative(projectRoot, targetTsFile).replace(/\\/g, '/');
                                const key = 'dist/' + relTsPath.replace(/\.ts$/, '.js');
                                
                                if (bundleCache && bundleCache.files) {
                                    bundleCache.files[key] = result.code;
                                }

                                // Update raw_files if it exists (for dev mode consistency)
                                if (bundleCache && bundleCache.raw_files) {
                                  bundleCache.raw_files[key] = result.code;
                                }

                                // Check for HMR ID in the compiled code
                                const hmrIdMatch = result.code.match(/const id = ["']([^"']+)["'];/);
                                if (hmrIdMatch) {
                                    const hmrId = hmrIdMatch[1];
                                    // Decode the ID for tracking (matches what @ng/component handler uses)
                                    const decodedHmrId = decodeURIComponent(hmrId);

                                    
                                    // Track pending update - this allows @ng/component handler to know there's a real change
                                    hmrPendingUpdates.add(decodedHmrId);
                                    
                                    // Dispatch surgical update event
                                    server.ws.send({
                                        type: 'custom',
                                        event: 'angular:component-update',
                                        data: {
                                            id: hmrId,
                                            timestamp: Date.now()
                                        }
                                    });
                                    
                                    // UPDATE CHUNK CONTENT DIRECTLY instead of clearing cache
                                    // This allows the browser to fetch the old chunk hash with NEW content

                                    
                                    // Find which chunk this file belongs to
                                    const moduleToChunk = bundleCache.moduleToChunk || {};
                                    const chunkName = moduleToChunk[relTsPath];
                                    
                                    if (chunkName && bundleCache.chunks) {
                                        // Update the chunk with new compiled code
                                        bundleCache.chunks[chunkName] = result.code;

                                        
                                        // Invalidate only this specific chunk in Vite's module graph
                                        const chunkVirtualId = '\0Chunk:' + chunkName;
                                        const chunkMod = server.moduleGraph.getModuleById(chunkVirtualId);
                                        if (chunkMod) {
                                            server.moduleGraph.invalidateModule(chunkMod);

                                        }
                                    }
                                    
                                    // DO NOT clear bundleCache! Keep it so chunk requests can be served.
                                    // The updated files[key] and chunks[chunkName] have the new content.

                                    // Return empty array to SILENTLY update
                                    // Vite will NOT trigger module invalidation or page reload

                                    return [];
                                }
                            }
                        } catch (e) {
                            console.error(`[HMR] Compile error:`, e);
                        }
                    }
                }
            } // Close if (file.endsWith)
        }, // Close handleHotUpdate

        async load(id) {
            console.log('[Plugin] load hook called for id:', id);
            // Handle /chunk-*.js at root path (lazy-loaded chunks). In lazy mode there are no pre-built chunks.
            // Match both absolute paths (from resolveId) and relative paths
            const chunkMatch = id.match(/(?:^|[\/\\])chunk-([A-F0-9]+)\.js$/i);
            if (chunkMatch) {
                if (lazyCompile) return null;
                const chunkName = 'chunk-' + chunkMatch[1] + '.js';
                console.log('[Plugin] Loading chunk:', chunkName, 'from id:', id);
                if (!bundleCache) await getBundle();
                if (bundleCache?.chunks?.[chunkName]) {
                    let content = bundleCache.chunks[chunkName];
                    content = content.replace(/(\?v=[a-f0-9]+)/g, '');
                    
                    let metadata = null;
                    try {
                        const metadataPath = path.resolve(projectRoot, 'node_modules/.vite/deps/_metadata.json');
                        if (fs.existsSync(metadataPath)) {
                            metadata = JSON.parse(fs.readFileSync(metadataPath, 'utf-8'));
                        }
                    } catch (e) {
                        console.warn('[Plugin] Failed to read metadata.json:', e);
                    }

                    // Rewrite Angular/RxJS imports to use Vite's optimized deps
                    // This ensures chunks use the same module instances as main bundle
                    content = rewriteToViteDeps(content, metadata);
                    return content;
                }
                console.error('[Plugin] Chunk not found:', chunkName);
                console.error('[Plugin] bundleCache:', !!bundleCache, 'chunks:', !!bundleCache?.chunks);
                if (bundleCache?.chunks) console.error('[Plugin] Available:', Object.keys(bundleCache.chunks).slice(0, 5));
                return null;
            }

            // Handle @ng/component HMR virtual module
            if (id.startsWith('\0virtual:angular-component')) {
                try {
                    ensureCompiler();
                    // Extract query from ID: \0virtual:angular-component?c=...
                    const queryStr = id.split('?')[1] || '';
                    const params = new URLSearchParams(queryStr);
                    const componentId = params.get('c');
                    
                    if (componentId) {
                        const decodedId = decodeURIComponent(componentId);
                        const [filePath, className] = decodedId.split('@');

                        if (compiler) {
                            // Find the original TS file
                            let targetTsFile = path.resolve(projectRoot, filePath);
                            if (!fs.existsSync(targetTsFile)) {
                                targetTsFile = path.resolve(projectRoot, filePath.replace(/^src\/app\//, 'src/'));
                            }
                            
                            if (fs.existsSync(targetTsFile)) {
                                const content = fs.readFileSync(targetTsFile, 'utf8');
                                
                                // Compile with hmrId to generate the UpdateMetadata callback
                                const result = compiler.compile(targetTsFile, content, { 
                                    hmr: true,
                                    hmrId: decodedId, 
                                    rootDir: projectRoot 
                                });
                                
                                if (result.code && !result.code.includes('/* Error')) {
                                    hmrPendingUpdates.delete(decodedId);
                                    return result.code;
                                }
                            }
                        }
                        return '// HMR compile fallback (compiler error or file not found)\nexport default function() {}';
                    }
                } catch (e) {
                    console.error('[Plugin] Failed to load @ng/component:', e);
                    return '// HMR error \nexport default function() {}';
                }
            }

            // Handle surgical HMR update request
            if (id.includes('hmr_id=')) {
                const [cleanId, query] = id.split('?');
                const params = new URLSearchParams(query);
                const hmrId = params.get('hmr_id');
                
                // Path normalization
                let filePath = cleanId;
                if (filePath.startsWith('/@fs/')) {
                    filePath = filePath.slice(5);
                } else if (!path.isAbsolute(filePath)) {
                    filePath = path.resolve(projectRoot, filePath.startsWith('/') ? filePath.slice(1) : filePath);
                }
                
                ensureCompiler();
                if (fs.existsSync(filePath)) {
                    const content = fs.readFileSync(filePath, 'utf8');
                    const result = compiler.compile(filePath, content, { 
                        hmr: true, 
                        hmrId: hmrId,
                        root_dir: projectRoot
                    });
                    if (result.code) {
                        return result.code;
                    }
                }
            }

            // --- Lazy compile: compile .ts on-demand, no full bundle ---
            if (lazyCompile && id.endsWith('.ts') && !id.includes('node_modules')) {
                ensureCompiler();
                const filePath = path.isAbsolute(id) ? id : path.resolve(projectRoot, id.startsWith('/') ? id.slice(1) : id);
                const normalizedPath = path.normalize(filePath);
                let code = lazyCompileCache.get(normalizedPath);
                if (!code && fs.existsSync(filePath)) {
                    const content = fs.readFileSync(filePath, 'utf8');
                    const result = compiler.compile(filePath, content, { hmr: true, root_dir: projectRoot });
                    if (result.code && !result.code.includes('/* Error')) {
                        code = result.code.replace(/(\?v=[a-f0-9]+)/g, '');
                        lazyCompileCache.set(normalizedPath, code);
                    }
                }
                if (code) {
                    const relPath = path.relative(projectRoot, filePath).replace(/\\/g, '/');
                    if (relPath.endsWith('main.ts')) {
                        code = injectMainPreamble(code, projectRoot, globalStyles);
                    }
                    return { code, map: null };
                }
            }

            // Polyfills in lazy mode: build from angular.json (no full bundle)
            if (lazyCompile && id === '\0angular:polyfills') {
                ensureCompiler();
                try {
                    const configPath = path.resolve(projectRoot, 'angular.json');
                    if (fs.existsSync(configPath)) {
                        const config = JSON.parse(fs.readFileSync(configPath, 'utf8'));
                        const project = Object.values(config.projects)[0];
                        const polyfills = project?.architect?.build?.options?.polyfills || [];
                        if (Array.isArray(polyfills) && polyfills.length > 0) {
                            const lines = polyfills.map((p) => `import '${typeof p === 'string' ? p : p.input}';`).join('\n');
                            return lines;
                        }
                    }
                } catch (e) {}
                return '// No polyfills configured';
            }

            if (!lazyCompile && !bundleCache) await getBundle();

            // Handle polyfills virtual module - Vite will resolve zone.js etc to cached deps (bundled mode)
            if (id === '\0angular:polyfills') {
                const content = bundleCache?.polyfillsJs || bundleCache?.polyfills_js;
                if (content) {
                    // Return raw imports - Vite will transform them to use optimized deps
                    // e.g., import 'zone.js' -> import '/@fs/.../vite/deps/zone__js.js?v=...'
                    return content;
                }
                return '// No polyfills configured';
            }

            // Handle .ts files - intercept before Vite's native transform
            // id can be absolute path (from Vite) or request path
            if (id.endsWith('.ts') && !id.includes('node_modules')) {
                // Normalize to relative path from project root
                let relPath;
                if (path.isAbsolute(id)) {
                    // Absolute path like /Users/.../demo-app/src/main.ts
                    relPath = path.relative(projectRoot, id);
                } else if (id.startsWith('/')) {
                    // Request path like /src/app/app.ts
                    relPath = id.slice(1); // Remove leading /
                } else {
                    relPath = id;
                }
                
                // Try multiple key formats to find compiled code
                // In dev mode, prefer rawFiles (has imports intact) over files (imports stripped for bundling)
                const jsKey = 'dist/' + relPath.replace(/\.ts$/, '.js');
                const tsKey = relPath; // source key like src/app/app.ts
                const jsKeyNoPrefix = relPath.replace(/\.ts$/, '.js');
                
                // rawFiles preserves imports for dev mode ES module resolution
                const rawFiles = bundleCache?.rawFiles || bundleCache?.raw_files;
                const processedFiles = bundleCache?.files;
                
                // Prefer raw files (imports intact) for dev mode, else fall back to processed files
                let code = rawFiles?.[jsKey] || 
                           rawFiles?.[tsKey] || 
                           rawFiles?.[jsKeyNoPrefix] ||
                           processedFiles?.[jsKey] || 
                           processedFiles?.[tsKey] || 
                           processedFiles?.[jsKeyNoPrefix];
                
                
                if (code) {
                    // Strip version hashes - Vite will add fresh ones
                    code = code.replace(/(\?v=[a-f0-9]+)/g, '');
                    // For main.js, inject styles and HMR bootstrap
                    if (relPath.endsWith('main.ts')) {
                        code = injectMainPreamble(code, projectRoot, globalStyles);
                    }
                    return { code, map: null };
                }
            }


            // Handle styles.css (served as raw file)
            if (id.endsWith('styles.css')) {
                const key = path.relative(projectRoot, id);
                if (key === 'styles.css') {
                    const content = bundleCache.stylesCss || bundleCache.styles_css;
                    if (content) {
                        return content;
                    }
                }
            }

            if (id.startsWith('\0')) {
                // Check if it's a chunk
                if (id.startsWith('\0Chunk:')) {
                    const chunkKeyRaw = id.slice(7); // Remove '\0Chunk:'
                    // Strip query params (e.g. &t=...) introduced by dynamic imports
                    const chunkKey = chunkKeyRaw.split('&')[0].split('?')[0]; 
                    
                    if (bundleCache?.chunks?.[chunkKey]) {
                         // Strip version hashes - Vite will add fresh ones
                         let chunkContent = bundleCache.chunks[chunkKey];
                         chunkContent = chunkContent.replace(/(\?v=[a-f0-9]+)/g, '');
                         return chunkContent;
                    } else {
                        // Fallback: Check if this is a stale hash and find the new chunk
                        const sourcePath = lastChunkMap[chunkKey];
                        if (sourcePath) {

                            // Find the new chunk hash for this source path
                            if (bundleCache.moduleToChunk) {
                                let newChunkHash = bundleCache.moduleToChunk[sourcePath];
                                if (!newChunkHash) {
                                     // Try normalizing source path (sometimes relative vs absolute issues in cache keys)
                                     for(let [newHash, src] of Object.entries(bundleCache.chunkNames || {})) {
                                         if (src === sourcePath) {
                                             newChunkHash = newHash;
                                             break;
                                         }
                                     }
                                }

                                if (newChunkHash && bundleCache.chunks[newChunkHash]) {

                                    let chunkContent = bundleCache.chunks[newChunkHash];
                                    chunkContent = chunkContent.replace(/(\?v=[a-f0-9]+)/g, '');
                                    return chunkContent;
                                }
                            }
                        }
                        
                        console.error(`[Plugin] Chunk not found in cache: ${chunkKey}`);
                    }
                }

                const key = id.slice(1);

                // Handle dynamic bundle (monolithic bundle)
                const bundleName = bundleCache.bundleName || bundleCache.bundle_name || 'bundle.js';
                if (key === bundleName) {
                    let content = bundleCache.bundleJs || bundleCache.bundle_js;
                    if (content) {
                         // Strip version hashes from import paths - Vite will add fresh ones
                         // This prevents stale hashes from previous builds causing duplicate module loads
                         content = content.replace(/(\?v=[a-f0-9]+)/g, '');
                         return content;
                    }
                }

                if (key === 'styles.css' || cleanId === '/styles.css' || cleanId === 'styles.css') {
                    if (bundleCache.stylesCss || bundleCache.styles_css) return path.resolve(projectRoot, 'styles.css');
                }

                return null;
            }
        },



        async resolveId(id, importer) {

            const cleanId = id.split('?')[0];

            // Virtual modules are self-resolving
            if (cleanId.startsWith('\0')) return cleanId;

            // Handle chunk imports at root path (served via middleware)
            // These are lazy-loaded chunks like /chunk-XXXXXX.js
            if (cleanId.match(/^\/chunk-[a-zA-Z0-9]+\.js$/i)) {
                // Return as absolute path (pseudo-file) so Vite can resolve node_modules relative to project root
                const fakePath = path.resolve(projectRoot, cleanId.slice(1));
                console.log('[Plugin] resolveId resolving chunk:', cleanId, '->', fakePath);
                return { id: fakePath, external: false };
            }

            // Handle @ng/component HMR requests (virtual module)
            if (id.includes('@ng/component')) {
                // Use a cleaner virtual ID: \0angular:component
                // Extract query params from the original ID
                const queryIndex = id.indexOf('?');
                const query = queryIndex !== -1 ? id.slice(queryIndex) : '';
                return {
                    id: '\0virtual:angular-component' + query,
                    moduleSideEffects: true
                };
            }

            // Handle polyfills.js as virtual module - let Vite resolve dependencies like zone.js
            if (cleanId === '/polyfills.js' || cleanId === 'polyfills.js') {
                return '\0angular:polyfills';
            }

            // Lazy mode: resolve .ts to absolute path so load() can compile on-demand (no getBundle)
            if (lazyCompile) {
                ensureCompiler();
                const rawId = id.split('?')[0];
                let resolvedPath;
                if (importer) {
                    const importerPath = importer.startsWith('\0') ? path.join(projectRoot, importer.slice(1).replace(/^dist\//, '').replace(/\.js$/, '.ts')) : importer;
                    resolvedPath = path.resolve(path.dirname(importerPath), rawId);
                } else {
                    resolvedPath = path.resolve(projectRoot, rawId.startsWith('/') ? rawId.slice(1) : rawId);
                }
                let normalized = path.normalize(resolvedPath);
                if (!normalized.endsWith('.ts') && fs.existsSync(normalized + '.ts')) {
                    normalized = normalized + '.ts';
                }
                if (normalized.endsWith('.ts') && fs.existsSync(normalized) && !normalized.includes('node_modules')) {
                    const query = id.includes('?') ? '?' + id.split('?')[1] : '';
                    return normalized + query;
                }
            }

            if (!lazyCompile && !bundleCache) await getBundle();

            // Handle dynamic bundle name (e.g., main.js) explicitly
            const bundleName = bundleCache.bundleName || bundleCache.bundle_name || 'bundle.js';
            if (cleanId === `/${bundleName}` || cleanId === bundleName) {
                if (bundleCache.bundleJs || bundleCache.bundle_js) return '\0' + bundleName;
            }

            // Map .ts files to compiled .js in cache
            let resolvedPath = id;

            if (importer && importer.startsWith('\0')) {
                const virtualImporterPath = importer.slice(1);
                let importerDir;
                
                // Handle chunk imports - chunks use their chunkNames mapping for source path
                if (virtualImporterPath.startsWith('Chunk:')) {
                    const chunkKey = virtualImporterPath.slice(6); // Remove 'Chunk:'
                    // Try to find the source path from chunkNames
                    if (bundleCache?.chunkNames?.[chunkKey]) {
                        // chunkNames[chunkKey] is the source path like 'src/app/feature.module.ts'
                        const sourcePath = bundleCache.chunkNames[chunkKey];
                        importerDir = path.dirname(path.resolve(projectRoot, sourcePath));
                    } else {
                        // Fallback: assume chunk is in src/app/
                        importerDir = path.resolve(projectRoot, 'src/app');
                    }
                } else {
                    importerDir = path.dirname(path.resolve(projectRoot, virtualImporterPath));
                }
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
                             const result = compiler.compile(sourcePath, content, { 
                                 hmr: true,
                                 root_dir: projectRoot
                             });
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
                             }
                         } catch (e) {
                             console.error(`[rustBundlePlugin] Lazy compile failed for ${sourceRelPath}:`, e);
                         }
                     }
                 }

                if (bundleCache?.files?.[jsKey]) {
                    const query = id.includes('?') ? '?' + id.split('?')[1] : '';
                    return '\0' + jsKey + query;
                }
            }

            // Try direct match in files
            if (bundleCache?.files) {
                const query = id.includes('?') ? '?' + id.split('?')[1] : '';
                // Try with dist/ prefix
                const distKey = key.startsWith('dist/') ? key : 'dist/' + key;
                if (bundleCache.files[distKey]) return '\0' + distKey + query;
                if (bundleCache.files[distKey + '.js']) return '\0' + distKey + '.js' + query;

                // Try exact key
                if (bundleCache.files[key]) return '\0' + key + query;
                if (bundleCache.files[key + '.js']) return '\0' + key + '.js' + query;
            }

            // Try match in chunks (for lazy loaded modules)
            if (bundleCache?.chunks) {
                // key might be "chunk-name.js" or "dist/chunk-name.js"
                const chunkKey = key.replace(/^dist\//, '');
                if (bundleCache.chunks[chunkKey]) return '\0Chunk:' + chunkKey;
                
                // Reverse lookup: find chunk by source path
                // chunkNames is { hashedName: sourcePath }, we need sourcePath -> hashedName
                if (bundleCache.chunkNames || bundleCache.chunk_names) {
                    const chunkNamesMap = bundleCache.chunkNames || bundleCache.chunk_names;
                    
                    // Normalize the key for comparison (remove dist/, handle .js/.ts extensions)
                    let normalizedKey = key.replace(/^dist\//, '');
                    if (!normalizedKey.endsWith('.ts') && !normalizedKey.endsWith('.js')) {
                        normalizedKey = normalizedKey + '.ts'; // Dynamic imports usually omit extension
                    }
                    
                    // Search for matching chunk
                    for (const [hashedName, sourcePath] of Object.entries(chunkNamesMap)) {
                        // sourcePath might be like "src/app/src/components/materials/card/card.ts"
                        // normalizedKey might be like "src/app/src/components/materials/card/card.ts" 
                        // or "src/app/src/components/materials/card/card.js"
                        const sourcePathNoExt = sourcePath.replace(/\.(ts|js)$/, '');
                        const keyNoExt = normalizedKey.replace(/\.(ts|js)$/, '');
                        
                        if (sourcePathNoExt === keyNoExt || sourcePath === normalizedKey) {
                            return '\0Chunk:' + hashedName;
                        }
                    }
                }
            }

            // Handle dynamic bundle
            const bName = bundleCache.bundleName || bundleCache.bundle_name || 'bundle.js';
            if (key === bName) {
                if (bundleCache.bundleJs || bundleCache.bundle_js) return '\0' + bName;
            }

            if (key === 'styles.css' || cleanId === '/styles.css' || cleanId === 'styles.css') {
                if (bundleCache.stylesCss || bundleCache.styles_css) return path.resolve(projectRoot, 'styles.css');
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

            // Bypass Vite's strict version check for optimized deps by renaming ?v= to ?ver=
            // This fixes 504 errors when server state doesn't match disk metadata
            if (id.includes('node_modules/.vite/deps')) {
                code = code.replace(/(\?v=[a-f0-9]+)/g, (match) => match.replace('?v=', '?ver='));
            }

            // Link .mjs files (Raw Angular modules)
            if (id.endsWith('.mjs') && (id.includes('@angular') || id.includes('rxjs'))) {
                 if (code.includes('ɵɵngDeclare')) {
                     try {
                        ensureCompiler();
                        console.time('RustLink: ' + id);
                        let result = compiler.linkFile(id, code);
                        console.timeEnd('RustLink: ' + id);
                        if (result.startsWith('/* Linker Error')) {
                            console.error(`[Linker] Linker Error for ${id}:`, result);
                            return null;
                        }
                        if (result !== code) {
                            return `/* LINKED BY RUST LINKER */\n${result}`;
                        }
                    } catch (e) {
                         console.error(`[Linker] Exception for ${id}:`, e);
                    }
                }
            }
            if (id.includes('node_modules') && id.includes('@angular')) {
                 // console.log('[Vite Transform] Saw Angular file:', id);
            }
            // Link Angular libraries from node_modules
            if (id.includes('node_modules') && !id.endsWith('.css') && !id.endsWith('.scss')) {
                 if (code.includes('ɵɵngDeclare')) {
                     try {
                        ensureCompiler();
                        console.time('RustLink: ' + id);
                        let result = compiler.linkFile(id, code);
                        console.timeEnd('RustLink: ' + id);
                        if (result.startsWith('/* Linker Error')) {
                            console.error(`[Linker] Linker Error for ${id}:`, result);
                            return null;
                        }
                        if (result !== code) {
                            return `/* LINKED BY RUST LINKER */\n${result}`;
                        }
                    } catch (e) {
                         console.error(`[Linker] Exception for ${id}:`, e);
                    }
                }
            }


            // Manually resolve bare Angular imports in lazy chunks (fixes "Failed to resolve module specifier" error)
            if (id.includes('chunk-') && code.includes('@angular')) {
                // Regex matches:
                // 1. from "@angular/..."
                // 2. import "@angular/..."
                const regex = /(?:import|from)\s+['"]@angular\/([^'"]+)['"]/g;
                
                const newCode = code.replace(regex, (match, pkg) => {
                     try {
                         // Resolve to absolute path on disk using projectRoot to find node_modules
                         const resolved = require.resolve(`@angular/${pkg}`, { paths: [projectRoot] });
                         // Prefix with /@fs for Vite to serve it
                         const prefix = match.split(/['"]/)[0];
                         return `${prefix}'/@fs${resolved}'`;
                     } catch (e) {
                         // console.warn('[Plugin] Failed to manually resolve:', `@angular/${pkg}`, 'from', projectRoot);
                         return match;
                     }
                });
                
                if (newCode !== code) {
                    return { code: newCode, map: null };
                } else {
                     // Log if we missed it
                     const idx = code.indexOf('@angular');
                     if (idx !== -1) {
                         console.log('[Plugin] Failed to regex match @angular in:', id);
                         console.log('Snippet:', code.substring(Math.max(0, idx - 50), idx + 50));
                     }
                }
            }

            return null;
        },


        async transformIndexHtml(html) {
            await getBundle();

            // Inject polyfills.js BEFORE main bundle if present in angular.json config
            const hasPolyfills = bundleCache?.polyfillsJs || bundleCache?.polyfills_js;
            if (hasPolyfills && !html.includes('polyfills.js')) {
                const polyfillsTag = `<script src="/polyfills.js" type="module"></script>`;
                if (html.includes('</body>')) {
                    html = html.replace('</body>', `${polyfillsTag}\n</body>`);
                } else {
                    html += polyfillsTag;
                }
            }

            // Inject main.ts script if not present
            // Also check if any other module script is present (like main.js from bundled mode)
            const hasModuleScript = html.includes('type="module"');
            if (!html.includes('src/main.ts') && !hasModuleScript) {
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

function printBuildStats(result, durationMs, options = {}) {
  const maxLazyChunks = options.verbose ? Infinity : 15;
  
  // ANSI Color Codes - always enable colors (like Angular CLI does)
  // Angular CLI uses chalk which has more sophisticated detection, but we'll just enable by default
  const env = process.env;
  const forceNoColor = env.NO_COLOR || env.TERM === 'dumb';
  const isColorSupported = !forceNoColor;
  
  const bold = (s) => isColorSupported ? `\x1b[1m${s}\x1b[22m` : s;
  const green = (s) => isColorSupported ? `\x1b[32m${s}\x1b[39m` : s;
  const cyan = (s) => isColorSupported ? `\x1b[36m${s}\x1b[39m` : s;
  const dim = (s) => isColorSupported ? `\x1b[2m${s}\x1b[22m` : s;
  const yellow = (s) => isColorSupported ? `\x1b[33m${s}\x1b[39m` : s;

  let output = '\n';
  output += `${bold('Initial chunk files')} | ${bold('Names')}            |  ${bold('Raw size')}\n`;
  
  let totalSize = 0;

  // Styles
  const stylesText = result.stylesCss || result.styles_css || '';
  const stylesSize = stylesText ? Buffer.byteLength(stylesText, "utf8") : 0;
  if (stylesSize > 0) {
    output += `${green('styles.css'.padEnd(20))}| ${dim('styles'.padEnd(17))}| ${cyan(formatSize(stylesSize).padStart(9))} |\n`;
    totalSize += stylesSize;
  }

  // Main bundle
  const mainText = result.bundleJs || result.bundle_js || '';
  const mainSize = Buffer.byteLength(mainText, "utf8");
  output += `${green('main.js'.padEnd(20))}| ${dim('main'.padEnd(17))}| ${cyan(formatSize(mainSize).padStart(9))} |\n`;
  totalSize += mainSize;

  // Polyfills (if present)
  const polyfillsText = result.polyfillsJs || result.polyfills_js || '';
  const polyfillsSize = polyfillsText ? Buffer.byteLength(polyfillsText, "utf8") : 0;
  if (polyfillsSize > 0) {
    output += `${green('polyfills.js'.padEnd(20))}| ${dim('polyfills'.padEnd(17))}| ${cyan(formatSize(polyfillsSize).padStart(9))} |\n`;
    totalSize += polyfillsSize;
  }

  output += `\n${''.padEnd(20)}| ${bold('Initial total')}    | ${bold(formatSize(totalSize).padStart(9))}\n\n`;

  // Lazy chunks
  output += `${bold('Lazy chunk files')}    | ${bold('Names')}            |  ${bold('Raw size')}\n`;
  const chunks = result.chunks || {};
  const chunkKeys = Object.keys(chunks);

  if (chunkKeys.length > 0) {
    // Sort chunks by size descending
    const chunkStats = chunkKeys.map(chunkName => {
      const size = Buffer.byteLength(chunks[chunkName], "utf8");
      
      let shortName = chunkName.replace(/^chunk-/, "").replace(/\.js$/, "");
      
      // Use mapped name if available (extract just the component/module name)
      if (result.chunk_names && result.chunk_names[chunkName]) {
         shortName = result.chunk_names[chunkName];
      } else if (result.chunkNames && result.chunkNames[chunkName]) {
         shortName = result.chunkNames[chunkName];
      }
      
      // Extract just the last part of path (e.g., "src/app/src/components/card/card.ts" -> "card")
      shortName = shortName.replace(/\.(ts|js)$/, '').split('/').pop() || shortName;
      
      if (shortName.length > 16) shortName = shortName.substring(0, 13) + "...";

      return {
        chunkName,
        shortName,
        size,
        formattedSize: formatSize(size)
      };
    });

    chunkStats.sort((a, b) => b.size - a.size);

    const displayChunks = chunkStats.slice(0, maxLazyChunks);
    const hiddenCount = chunkStats.length - displayChunks.length;

    for (const stat of displayChunks) {
      output += `${green(stat.chunkName.padEnd(20))}| ${dim(stat.shortName.padEnd(17))}| ${cyan(stat.formattedSize.padStart(9))} |\n`;
    }
    
    if (hiddenCount > 0) {
      output += `${dim(`...and ${hiddenCount} more lazy chunk files. Use "--verbose" to show all the files.`)}\n`;
    }
  }

  const durationSeconds = (durationMs / 1000).toFixed(3);
  const timestamp = new Date().toISOString();

  output += `\n${green('Application bundle generation complete.')} [${cyan(durationSeconds + ' seconds')}] - ${timestamp}\n`;
  output += `\n${yellow('NOTE:')} Raw file sizes do not reflect development server per-request transformations.\n`;

  console.log(output);
}
