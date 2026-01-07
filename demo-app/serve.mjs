import { createServer } from 'vite';
import path from 'path';
import fs from 'fs';
import { fileURLToPath } from 'url';
import angularRust from '../packages/vite-plugin-angular-rust/src/index.js';

const __dirname = path.dirname(fileURLToPath(import.meta.url));

const packageJson = JSON.parse(fs.readFileSync(path.resolve(__dirname, 'package.json'), 'utf-8'));
const angularPackages = Object.keys(packageJson.dependencies || {}).filter((pkg) =>
  pkg.startsWith('@angular/'),
);

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
        include: ['zone.js', 'rxjs', 'rxjs/operators'],
      },
      plugins: [
        // Map root to src/index.html
        {
          name: 'index-html-redirect',
          configureServer(server) {
            server.middlewares.use((req, res, next) => {
              const cleanUrl = req.url.split('?')[0];
              // SPA Fallback: serve index.html for non-file requests
              // Exclude Vite internals (/@...), node_modules, and existing files/extensions
              if (
                !cleanUrl.includes('.') &&
                !cleanUrl.startsWith('/@') &&
                !cleanUrl.startsWith('/node_modules')
              ) {
                req.url = '/src/index.html';
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
