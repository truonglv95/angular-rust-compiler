import { createServer } from 'vite';
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
    console.log(`[Worker] hmr flag received: ${hmr} (type: ${typeof hmr})`);
    const binding = require(bindingPath);
    const compiler = new binding.Compiler();
    const result = compiler.bundle(angularJsonPath, hmr);
    parentPort.postMessage({ type: 'success', result });
  } catch (err) {
    parentPort.postMessage({ type: 'error', message: err.message });
  }
  process.exit(0);
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

const externalImportsCachePath = path.resolve(
  __dirname,
  '.angular/cache/rust-compiler/external-imports.json',
);
let cachedExternalImports = [];
if (fs.existsSync(externalImportsCachePath)) {
  try {
    cachedExternalImports = JSON.parse(fs.readFileSync(externalImportsCachePath, 'utf-8'));
  } catch (e) {}
}

function angularLinkerEsbuildPlugin() {
  const getLinker = () => {
    const bindingPath = path.resolve(__dirname, '../packages/binding/index.js');
    return new (require(bindingPath).Compiler)();
  };
  let linker = null;

  return {
    name: 'angular-linker',
    setup(build) {
      build.onLoad({ filter: /node_modules\/.*\.(mjs|js)$/ }, async (args) => {
        if (
          args.path.includes('/cjs/') ||
          args.path.includes('/commonjs/') ||
          args.path.includes('/lib/')
        )
          return null;
        const contents = await fs.promises.readFile(args.path, 'utf8');
        if (!contents.includes('ɵɵngDeclare')) return null;
        try {
          if (!linker) linker = getLinker();
          const linked = linker.linkFile(args.path, contents);
          if (linked && linked !== contents && !linked.startsWith('/* Linker Error')) {
            return { contents: `/* LINKED BY ESBUILD PLUGIN */\n${linked}`, loader: 'js' };
          }
        } catch (e) {}
        return null;
      });
    },
  };
}

async function startServer(finalPackagesToPreBundle, bundleResult = null) {
  const isBundled = process.argv.includes('--bundled');
  const bold = (s) => `\x1b[1m${s}\x1b[22m`;
  const cyan = (s) => `\x1b[36m${s}\x1b[39m`;
  const green = (s) => `\x1b[32m${s}\x1b[39m`;
  const dim = (s) => `\x1b[2m${s}\x1b[22m`;

  const server = await createServer({
    configFile: false,
    root: __dirname,
    appType: 'custom',
    server: { port: 4300, host: true, strictPort: true, clearScreen: false },
    optimizeDeps: {
      include: finalPackagesToPreBundle,
      entries: [],
      exclude: ['primeicons'],
      esbuildOptions: { target: 'es2020', plugins: [angularLinkerEsbuildPlugin()] },
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
      angularRust({
        configFile: path.resolve(__dirname, 'angular.json'),
        skipStats: true,
        lazyCompile: !isBundled,
        bundleResult: bundleResult, // Pass pre-built bundle from Worker thread
      }),
    ],
  });

  await server.listen();
  console.log(`\n  ${green('➜')}  ${bold('Local')}:   ${cyan('http://localhost:4300/')}`);
  console.log(`  ${green('➜')}  ${dim('Network: use --host to expose')}\n`);
  console.log(`${dim('Watch mode enabled. Watching for file changes...')}`);
}

async function main() {
  const isColorSupported = !(process.env.NO_COLOR || process.env.TERM === 'dumb');
  const bold = (s) => (isColorSupported ? `\x1b[1m${s}\x1b[22m` : s);
  const isBundled = process.argv.includes('--bundled');

  if (isBundled) {
    console.log(`${bold('Starting Angular Rust Dev Server (bundled)...')}`);
    const spinner = createSpinner('Building...');
    const startTime = Date.now();

    try {
      const isHmr = process.argv.includes('--hmr');
      const worker = new Worker(__filename, {
        stdout: true,
        stderr: true,
        workerData: {
          angularJsonPath: path.resolve(__dirname, 'angular.json'),
          bindingPath: path.resolve(__dirname, '../packages/binding/index.js'),
          hmr: isHmr,
        },
      });
      worker.stdout.pipe(process.stdout);
      worker.stderr.pipe(process.stderr);

      const result = await new Promise((resolve, reject) => {
        worker.on('message', (msg) => {
          if (msg.type === 'success') resolve(msg.result);
          else reject(new Error(msg.message));
        });
        worker.on('error', reject);
        worker.on('exit', (code) => {
          if (code !== 0) reject(new Error(`Worker stopped with exit code ${code}`));
        });
      });

      const duration = Date.now() - startTime;
      spinner.stop();
      printBuildStats(result, duration);

      if (result && result.externalImports) {
        cachedExternalImports = result.externalImports;
        const cacheDir = path.dirname(externalImportsCachePath);
        if (!fs.existsSync(cacheDir)) fs.mkdirSync(cacheDir, { recursive: true });
        fs.writeFileSync(externalImportsCachePath, JSON.stringify(cachedExternalImports, null, 2));
      }

      const finalPackagesToPreBundle = [
        ...new Set([...packagesToPreBundle, ...cachedExternalImports]),
      ];
      await startServer(finalPackagesToPreBundle, result);
    } catch (err) {
      spinner.stop(`\x1b[31m✖\x1b[0m Build failed: ${err.message}`);
      process.exit(1);
    }
  } else {
    // Lazy compile: no full bundle upfront; server starts immediately, .ts compiled on-demand
    console.log(`${bold('Starting Angular Rust Dev Server (lazy compile)...')}`);
    const finalPackagesToPreBundle = [
      ...new Set([...packagesToPreBundle, ...cachedExternalImports]),
    ];
    await startServer(finalPackagesToPreBundle);
  }
}

main().catch((err) => {
  console.error('Fatal error:', err);
  process.exit(1);
});
