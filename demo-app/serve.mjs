import { createServer } from 'vite';
import path from 'path';
import fs from 'fs';
import { fileURLToPath } from 'url';
import angularRust from '../packages/vite-plugin-angular-rust/src/index.js';

const __dirname = path.dirname(fileURLToPath(import.meta.url));

const packageJson = JSON.parse(fs.readFileSync(path.resolve(__dirname, 'package.json'), 'utf-8'));
const optimizeDepsInclude = ['rxjs', 'tslib', 'zone.js', 'rxjs/operators'];
const dependencies = Object.keys(packageJson.dependencies || {});
const angularPackages = dependencies.filter(
  (pkg) => !optimizeDepsInclude.some((include) => pkg === include || pkg.startsWith(include + '/')),
);

console.log('[serve.mjs] Angular packages excluded from optimization:', angularPackages);
fs.writeFileSync('/tmp/excludes.json', JSON.stringify(angularPackages, null, 2));

// Stats plugin (inline for now, or move to separate file/package)
function angularStatsPlugin() {
  return {
    name: 'angular-stats',
    generateBundle(options, bundle) {
      // ... (simplified or same logic)
      // Keeping it empty/simple for now to focus on serving
    },
  };
}

async function startServer() {
  console.log('Starting Angular Rust Dev Server...');

  try {
    const server = await createServer({
      // We are defining configuration programmatically
      configFile: false,
      root: __dirname,
      server: {
        port: 4300,
      },
      resolve: {
        extensions: ['.ts', '.js', '.json'],
      },
      optimizeDeps: {
        exclude: angularPackages,
        include: optimizeDepsInclude,
      },
      plugins: [
        // Map root to configured index.html and inject main script
        {
          name: 'index-html-redirect',
          configureServer(server) {
            server.middlewares.use((req, res, next) => {
              const cleanUrl = req.url.split('?')[0];

              // Check if request is for root or index.html
              if (cleanUrl === '/' || cleanUrl === '/index.html') {
                try {
                  // Read angular.json to find index and main files
                  const angularJsonPath = path.resolve(__dirname, 'angular.json');
                  const angularJson = JSON.parse(fs.readFileSync(angularJsonPath, 'utf-8'));
                  const projectConfig = angularJson.projects['demo-app']; // Assuming single project or hardcoded for demo
                  const buildOptions = projectConfig.architect.build.options;

                  const indexFilePath = path.resolve(__dirname, buildOptions.index);
                  const mainFilePath = '/' + buildOptions.browser; // e.g., /src/main.ts

                  let html = fs.readFileSync(indexFilePath, 'utf-8');

                  // Inject main script if not present
                  if (!html.includes(mainFilePath)) {
                    html = html.replace(
                      '</body>',
                      `  <script type="module" src="${mainFilePath}"></script>\n  </body>`,
                    );
                  }

                  res.setHeader('Content-Type', 'text/html');
                  res.end(html);
                  return;
                } catch (err) {
                  console.error('Error serving index.html:', err);
                  next(err);
                  return;
                }
              }

              // SPA Fallback: serve index.html for non-file requests
              // Exclude Vite internals (/@...), node_modules, and existing files/extensions
              if (
                !cleanUrl.includes('.') &&
                !cleanUrl.startsWith('/@') &&
                !cleanUrl.startsWith('/node_modules')
              ) {
                // Reuse the same logic for SPA fallback
                try {
                  const angularJsonPath = path.resolve(__dirname, 'angular.json');
                  const angularJson = JSON.parse(fs.readFileSync(angularJsonPath, 'utf-8'));
                  const projectConfig = angularJson.projects['demo-app'];
                  const buildOptions = projectConfig.architect.build.options;

                  const indexFilePath = path.resolve(__dirname, buildOptions.index);
                  const mainFilePath = '/' + buildOptions.browser;

                  let html = fs.readFileSync(indexFilePath, 'utf-8');

                  if (!html.includes(mainFilePath)) {
                    html = html.replace(
                      '</body>',
                      `  <script type="module" src="${mainFilePath}"></script>\n  </body>`,
                    );
                  }

                  res.setHeader('Content-Type', 'text/html');
                  res.end(html);
                  return;
                } catch (err) {
                  console.error('SPA Fallback Error:', err);
                  // fallthrough
                }
              }
              next();
            });
          },
        },
        // Use our local plugin
        angularRust({ project: path.resolve(__dirname, 'angular.json') }),
        angularStatsPlugin(),
      ],
    });

    await server.listen();

    server.printUrls();
  } catch (e) {
    console.error('Failed to start server:', e);
    process.exit(1);
  }
}

startServer();
