import { createServer } from 'rolldown-vite';
import path from 'path';
import fs from 'fs';
import { fileURLToPath } from 'url';
import { createRequire } from 'module';
import { Worker, isMainThread, parentPort, workerData } from 'worker_threads';
import angularRust from '../packages/vite-plugin-angular-rust/src/index.js';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const require = createRequire(import.meta.url);

/**
 * WORKER SECTION: Runs the actual bundling in a separate thread
 */
if (!isMainThread) {
  try {
    const { angularJsonPath, bindingPath, hmr } = workerData;
    const binding = require(bindingPath);
    const compiler = new binding.Compiler();
    console.error(`[Worker] Compiler instantiated. Listening for bundle requests...`);

    // Handle bundle requests from the main thread
    parentPort.on('message', (msg) => {
      if (msg.type === 'bundle') {
        const t0 = Date.now();
        try {
          const result = compiler.bundle(angularJsonPath, hmr);
          console.error(`[Worker] compiler.bundle took: ${Date.now() - t0}ms`);
          parentPort.postMessage({ type: 'success', result, reqId: msg.reqId });
        } catch (err) {
          parentPort.postMessage({ type: 'error', message: err.message, reqId: msg.reqId });
        }
      }
    });

    // Signal ready — do NOT call process.exit(), worker stays alive for rebundles
    parentPort.postMessage({ type: 'ready' });
  } catch (err) {
    parentPort.postMessage({ type: 'error', message: err.message });
    process.exit(1);
  }
  // Worker thread ends here — main thread code below is never reached in worker context
  // because isMainThread is false in the main() guard at the bottom of this file.
}

// --- Main Thread Logic Starts Here ---

// Auto-clear Vite cache AND dist folder on startup to prevent "Outdated Optimize Dep" errors
const viteCacheDir = path.resolve(__dirname, 'node_modules/.vite');
const distDir = path.resolve(__dirname, 'dist');

if (fs.existsSync(viteCacheDir)) {
  fs.rmSync(viteCacheDir, { recursive: true, force: true });
}
if (fs.existsSync(distDir)) {
  fs.rmSync(distDir, { recursive: true, force: true });
}

/**
 * Simple terminal spinner
 */
function createSpinner(text) {
  const frames = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
  let i = 0;
  const interval = setInterval(() => {
    process.stdout.write(`\r\x1b[36m${frames[i++ % frames.length]}\x1b[0m ${text}`);
  }, 100);
  return {
    stop: (finalText = '') => {
      clearInterval(interval);
      // \x1b[2K clears the entire line, \r moves cursor to beginning
      process.stdout.write(`\r\x1b[2K${finalText}${finalText ? '\n' : ''}`);
    },
  };
}

/**
 * Format bytes to readable string (kB, MB, etc.)
 */
function formatSize(bytes) {
  if (bytes === 0) return '0 B';
  const k = 1000;
  const sizes = ['B', 'kB', 'MB', 'GB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
}

/**
 * Print build statistics table (Angular CLI style)
 */
function printBuildStats(result, durationMs, options = {}) {
  const isColorSupported = !(process.env.NO_COLOR || process.env.TERM === 'dumb');
  const bold = (s) => (isColorSupported ? `\x1b[1m${s}\x1b[22m` : s);
  const green = (s) => (isColorSupported ? `\x1b[32m${s}\x1b[39m` : s);
  const cyan = (s) => (isColorSupported ? `\x1b[36m${s}\x1b[39m` : s);
  const dim = (s) => (isColorSupported ? `\x1b[2m${s}\x1b[22m` : s);
  const yellow = (s) => (isColorSupported ? `\x1b[33m${s}\x1b[39m` : s);

  let output = '\n';
  output += `${bold('Initial chunk files')} | ${bold('Names')}            |  ${bold('Raw size')}\n`;

  let totalSize = 0;
  const stylesSize = Buffer.byteLength(result.stylesCss || result.styles_css || '', 'utf8');
  if (stylesSize > 0) {
    output += `${green('styles.css'.padEnd(20))}| ${dim('styles'.padEnd(17))}| ${cyan(formatSize(stylesSize).padStart(9))} |\n`;
    totalSize += stylesSize;
  }

  const mainSize = Buffer.byteLength(result.bundleJs || result.bundle_js || '', 'utf8');
  output += `${green('main.js'.padEnd(20))}| ${dim('main'.padEnd(17))}| ${cyan(formatSize(mainSize).padStart(9))} |\n`;
  totalSize += mainSize;

  output += `\n${''.padEnd(20)}| ${bold('Initial total')}    | ${bold(formatSize(totalSize).padStart(9))}\n\n`;

  const chunks = result.chunks || {};
  const chunkKeys = Object.keys(chunks);
  if (chunkKeys.length > 0) {
    output += `${bold('Lazy chunk files')}    | ${bold('Names')}            |  ${bold('Raw size')}\n`;
    const chunkStats = chunkKeys
      .map((name) => {
        const size = Buffer.byteLength(chunks[name], 'utf8');
        let shortName =
          (result.chunkNames && result.chunkNames[name]) ||
          (result.chunk_names && result.chunk_names[name]) ||
          name.replace(/^chunk-/, '').replace(/\.js$/, '');
        shortName = shortName
          .split('/')
          .pop()
          .replace(/\.(ts|js)$/, '');
        if (shortName.length > 16) shortName = shortName.substring(0, 13) + '...';
        return { name, shortName, size };
      })
      .sort((a, b) => b.size - a.size);

    const maxLazy = options.verbose ? Infinity : 15;
    chunkStats.slice(0, maxLazy).forEach((s) => {
      output += `${green(s.name.padEnd(20))}| ${dim(s.shortName.padEnd(17))}| ${cyan(formatSize(s.size).padStart(9))} |\n`;
    });
    if (chunkStats.length > maxLazy) {
      output += `${dim(`...and ${chunkStats.length - maxLazy} more lazy chunk files. Use "--verbose" to show all the files.`)}\n`;
    }
  }

  const durationSeconds = (durationMs / 1000).toFixed(3);
  const timestamp = new Date().toISOString();
  output += `\n${green('Application bundle generation complete.')} [${cyan(durationSeconds + ' seconds')}] - ${timestamp}\n`;
  output += `\n${yellow('NOTE:')} Raw file sizes do not reflect development server per-request transformations.\n`;
  console.log(output);
}

const packageJson = JSON.parse(fs.readFileSync(path.resolve(__dirname, 'package.json'), 'utf-8'));
const dependencies = Object.keys(packageJson.dependencies || {});

function canPackageBeOptimized(pkgName) {
  try {
    const pkgDir = pkgName.startsWith('@')
      ? path.resolve(__dirname, 'node_modules', ...pkgName.split('/'))
      : path.resolve(__dirname, 'node_modules', pkgName);
    const pkgJsonPath = path.join(pkgDir, 'package.json');
    if (!fs.existsSync(pkgJsonPath)) return true;
    const pkgJson = JSON.parse(fs.readFileSync(pkgJsonPath, 'utf-8'));
    return !!(
      pkgJson.fesm2022 ||
      pkgJson.fesm2020 ||
      pkgJson.es2020 ||
      pkgJson.exports ||
      pkgJson.module ||
      pkgJson.main ||
      pkgJson.browser
    );
  } catch (e) {
    return true;
  }
}

const autoExcluded = dependencies.filter((pkg) => !canPackageBeOptimized(pkg));
const packagesToPreBundle = dependencies.filter((pkg) => !autoExcluded.includes(pkg));

function resolveCacheDir(projectRoot) {
  let version = 'unknown';
  let projectName = 'default-app';
  try {
    const pkgPath = path.resolve(projectRoot, 'node_modules/@angular/core/package.json');
    if (fs.existsSync(pkgPath)) {
      const pkg = JSON.parse(fs.readFileSync(pkgPath, 'utf8'));
      if (pkg.version) version = pkg.version;
    }
  } catch (e) {}

  try {
    const angularJsonPath = path.resolve(projectRoot, 'angular.json');
    if (fs.existsSync(angularJsonPath)) {
      const angularJson = JSON.parse(fs.readFileSync(angularJsonPath, 'utf8'));
      if (angularJson.projects) {
        const keys = Object.keys(angularJson.projects);
        if (keys.length > 0) projectName = keys[0];
      }
    }
  } catch (e) {}

  return path.resolve(projectRoot, `.angular/cache/${version}/${projectName}`);
}

const cacheDirBase = resolveCacheDir(__dirname);
const externalImportsCachePath = path.join(cacheDirBase, 'external-imports.json');
let cachedExternalImports = [];
if (fs.existsSync(externalImportsCachePath)) {
  try {
    cachedExternalImports = JSON.parse(fs.readFileSync(externalImportsCachePath, 'utf-8'));
  } catch (e) {}
}

function angularLinkerRolldownPlugin() {
  const getLinker = () => {
    const bindingPath = path.resolve(__dirname, '../packages/binding/index.js');
    return new (require(bindingPath).Compiler)();
  };
  let linker = null;

  return {
    name: 'angular-linker',
    enforce: 'pre',
    async transform(code, id) {
      // console.log(`[Linker] Checking: ${id}`);
      if (!id.includes('node_modules') || (!id.endsWith('.mjs') && !id.endsWith('.js'))) {
        return null;
      }
      if (id.includes('/cjs/') || id.includes('/commonjs/') || id.includes('/lib/')) {
        return null; // Skip CommonJS
      }
      if (!code.includes('ɵɵngDeclare')) {
        return null;
      }
      try {
        if (!linker) linker = getLinker();
        const linked = linker.linkFile(id, code);
        if (linked && linked !== code && !linked.startsWith('/* Linker Error')) {
          return { code: `/* LINKED BY ROLLDOWN PLUGIN */\n${linked}`, map: null };
        }
      } catch (e) {
        console.error('[Angular Linker Rolldown Error]', e);
      }
      return null;
    },
  };
}

async function startServer(finalPackagesToPreBundle, result, bundlePromise, rebundleFn) {
  const isBundled = process.argv.includes('--bundled');
  const isHmr = process.argv.includes('--hmr');
  const bold = (s) => `\x1b[1m${s}\x1b[22m`;
  const cyan = (s) => `\x1b[36m${s}\x1b[39m`;
  const green = (s) => `\x1b[32m${s}\x1b[39m`;
  const dim = (s) => `\x1b[2m${s}\x1b[22m`;

  // Base plugin options
  const pluginOptions = {
    configFile: path.resolve(__dirname, 'angular.json'),
    skipStats: true,
    hmr: isHmr,
    ...(rebundleFn ? { rebundleFn } : {}),
  };

  // If we already have a compiled result, pass it as precompiled data
  if (result) {
    Object.assign(pluginOptions, {
      precompiledFiles: result.files || result.compiled_files,
      rawFiles: result.rawFiles || result.raw_files,
      chunks: result.chunks,
      chunkNames: result.chunkNames || result.chunk_names,
      moduleToChunk: result.moduleToChunk || result.module_to_chunk,
      bundleJs: result.bundleJs || result.bundle_js,
      bundleName: result.bundleName || result.bundle_name,
      stylesCss: result.stylesCss || result.styles_css,
      polyfillsJs: result.polyfillsJs || result.polyfills_js,
      externalImports: result.externalImports || result.external_imports,
    });
  } else if (bundlePromise) {
    // Parallel mode: plugin waits on the promise, Vite starts immediately with correct deps
    pluginOptions.bundlePromise = bundlePromise.then((r) => ({
      files: r.files || r.compiled_files,
      rawFiles: r.rawFiles || r.raw_files,
      chunks: r.chunks,
      chunkNames: r.chunkNames || r.chunk_names,
      moduleToChunk: r.moduleToChunk || r.module_to_chunk,
      bundleJs: r.bundleJs || r.bundle_js,
      bundleName: r.bundleName || r.bundle_name,
      stylesCss: r.stylesCss || r.styles_css,
      polyfillsJs: r.polyfillsJs || r.polyfills_js,
      externalImports: r.externalImports || r.external_imports,
    }));
  }

  const server = await createServer({
    configFile: false,
    root: __dirname,
    cacheDir: path.join(cacheDirBase, 'vite'),
    appType: 'custom',
    server: { port: 4300, host: true, strictPort: true, clearScreen: false },
    optimizeDeps: {
      include: finalPackagesToPreBundle,
      entries: [],
      exclude: ['primeicons'],
      rolldownOptions: { plugins: [angularLinkerRolldownPlugin()] },
    },
    plugins: [
      {
        name: 'angular-html-serve',
        configureServer(server) {
          const angularJson = JSON.parse(
            fs.readFileSync(path.resolve(__dirname, 'angular.json'), 'utf-8'),
          );
          const buildOptions = angularJson.projects['demo-app'].architect.build.options;
          const srcMain = '/' + buildOptions.browser;
          const bundleName = path.basename(buildOptions.browser).replace(/\.ts$/, '.js');
          const indexFilePath = path.resolve(__dirname, buildOptions.index);

          return () => {
            server.middlewares.use(async (req, res, next) => {
              const cleanUrl = req.url.split('?')[0];
              const isHtmlRequest =
                cleanUrl === '/' ||
                cleanUrl === '/index.html' ||
                (!cleanUrl.includes('.') &&
                  !cleanUrl.startsWith('/@') &&
                  !cleanUrl.startsWith('/node_modules'));
              if (isHtmlRequest) {
                try {
                  let html = fs.readFileSync(indexFilePath, 'utf-8');
                  if (isBundled) {
                    html = html.replace(
                      '</head>',
                      `  <link rel="stylesheet" href="/styles.css">\n</head>`,
                    );
                    if (buildOptions.polyfills && buildOptions.polyfills.length > 0) {
                      html = html.replace(
                        '</body>',
                        `  <script type="module" src="/polyfills.js"></script>\n</body>`,
                      );
                    }
                    html = html.replace(
                      '</body>',
                      `  <script type="module" src="/${bundleName}"></script>\n</body>`,
                    );
                  } else {
                    html = html.replace(
                      '</body>',
                      `  <script type="module" src="${srcMain}"></script>\n</body>`,
                    );
                  }
                  html = await server.transformIndexHtml(req.url, html);
                  res.setHeader('Content-Type', 'text/html').end(html);
                  return;
                } catch (e) {}
              }
              next();
            });
          };
        },
      },
      angularRust(pluginOptions),
    ],
  });

  await server.listen();
  console.log(`\n  ${green('➜')}  ${bold('Local')}:   ${cyan('http://localhost:4300/')}`);
  console.log(`  ${green('➜')}  ${dim('Network: use --host to expose')}\n`);
  console.log(`${dim('Watch mode enabled. Watching for file changes...')}`);
}

// Persistent compiler worker singleton.
// Created once at startup; reused for all subsequent HMR re-bundles.
let _persistentWorker = null;
let _persistentWorkerPromise = null;
let _reqIdCounter = 0;
const _pendingRequests = new Map(); // reqId → { resolve, reject }

function getPersistentWorker(angularJsonPath, bindingPath, isHmr) {
  if (_persistentWorker) return _persistentWorker;

  _persistentWorker = new Worker(__filename, {
    stdout: true,
    stderr: true,
    workerData: { angularJsonPath, bindingPath, hmr: isHmr },
  });
  _persistentWorker.stdout.pipe(process.stdout);
  _persistentWorker.stderr.pipe(process.stderr);

  _persistentWorker.on('message', (msg) => {
    if (msg.type === 'ready') return; // Initial ready signal, ignore
    const pending = _pendingRequests.get(msg.reqId);
    if (!pending) return;
    _pendingRequests.delete(msg.reqId);
    if (msg.type === 'success') pending.resolve(msg.result);
    else pending.reject(new Error(msg.message));
  });

  _persistentWorker.on('error', (err) => {
    // Reject all pending and clear
    for (const [, p] of _pendingRequests) p.reject(err);
    _pendingRequests.clear();
    _persistentWorker = null;
  });

  _persistentWorker.on('exit', (code) => {
    for (const [, p] of _pendingRequests) p.reject(new Error(`Worker exited with code ${code}`));
    _pendingRequests.clear();
    _persistentWorker = null;
  });

  return _persistentWorker;
}

function sendBundleRequest(angularJsonPath, bindingPath, isHmr) {
  return new Promise((resolve, reject) => {
    const reqId = ++_reqIdCounter;
    _pendingRequests.set(reqId, { resolve, reject });
    const worker = getPersistentWorker(angularJsonPath, bindingPath, isHmr);
    worker.postMessage({ type: 'bundle', reqId });
  });
}

async function main() {
  const isColorSupported = !(process.env.NO_COLOR || process.env.TERM === 'dumb');
  const bold = (s) => (isColorSupported ? `\x1b[1m${s}\x1b[22m` : s);
  console.log(`${bold('Starting Angular Rust Dev Server...')}`);

  const angularJsonPath = path.resolve(__dirname, 'angular.json');
  const bindingPath = path.resolve(__dirname, '../packages/binding/index.js');
  const isHmr = process.argv.includes('--hmr');

  const spinner = createSpinner('Compiling...');
  const startTime = Date.now();

  try {
    const t_worker_init = Date.now();
    // Use the persistent worker for the initial bundle
    const result = await sendBundleRequest(angularJsonPath, bindingPath, isHmr);
    console.log(`[Main] Initial bundle done in ${Date.now() - t_worker_init}ms`);

    const duration = Date.now() - startTime;
    spinner.stop();
    printBuildStats(result, duration);

    // Use 100% accurate externalImports from the bundle result — no scan needed
    if (result && result.externalImports) {
      cachedExternalImports = result.externalImports;
      const cacheDir = path.dirname(externalImportsCachePath);
      if (!fs.existsSync(cacheDir)) fs.mkdirSync(cacheDir, { recursive: true });
      fs.writeFileSync(externalImportsCachePath, JSON.stringify(cachedExternalImports, null, 2));
    }

    const finalPackagesToPreBundle = [
      ...new Set([...packagesToPreBundle, ...cachedExternalImports]),
    ];

    // Pass a rebundle function to the server so HMR uses the SAME compiler instance
    await startServer(finalPackagesToPreBundle, result, null, () =>
      sendBundleRequest(angularJsonPath, bindingPath, isHmr),
    );
  } catch (err) {
    spinner.stop(`\x1b[31m✖\x1b[0m Build failed: ${err.message}`);
    process.exit(1);
  }
}

if (isMainThread) {
  main().catch((err) => {
    console.error('Fatal error:', err);
    process.exit(1);
  });
}
