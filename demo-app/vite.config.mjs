import { defineConfig } from 'vite';
import { createRequire } from 'module';
import { fileURLToPath } from 'url';
import path from 'path';
import fs from 'fs';

const require = createRequire(import.meta.url);
const __dirname = path.dirname(fileURLToPath(import.meta.url));

// Resolve binding path relative to this file
// demo-app/vite.config.mjs -> packages/binding
const bindingPath = path.resolve(__dirname, '../packages/binding');
const { Compiler } = require(bindingPath);

const compiler = new Compiler();

// Esbuild plugin for Angular linker during pre-bundling
function angularLinkerEsbuildPlugin() {
  return {
    name: 'angular-linker-esbuild',
    setup(build) {
      // Handle all .mjs and .js files in @angular packages
      build.onLoad({ filter: /\/@angular\/.*\.(mjs|js)$/ }, async (args) => {
        console.log(`[Pre-bundle] Processing: ${args.path}`);
        const code = await fs.promises.readFile(args.path, 'utf8');

        // Check if file contains partial declarations
        if (!code.includes('ɵɵngDeclare')) {
          return { contents: code, loader: 'js' };
        }

        try {
          const result = compiler.linkFile(args.path, code);
          if (result.startsWith('/* Linker Error')) {
            console.error(`[Pre-bundle Linker Error] ${args.path}:\n${result}`);
            return { contents: code, loader: 'js' };
          }
          return { contents: result, loader: 'js' };
        } catch (e) {
          console.error(`[Pre-bundle Linker Failed] ${args.path}:`, e);
          return { contents: code, loader: 'js' };
        }
      });
    },
  };
}

function rustNgcPlugin() {
  return {
    name: 'rust-ngc-plugin',
    enforce: 'pre',
    transform(code, id) {
      // Skip pre-bundled dependencies (handled by esbuild plugin)
      if (id.includes('node_modules')) {
        // Only log if it's an Angular file that somehow wasn't pre-bundled
        if (id.includes('@angular') && id.includes('ɵɵngDeclare')) {
          console.log(`[Vite Plugin] Fallback linking for: ${id}`);
          const cleanId = id.split('?')[0];
          if (cleanId.endsWith('.mjs') || cleanId.endsWith('.js')) {
            try {
              const result = compiler.linkFile(id, code);
              if (result.startsWith('/* Linker Error')) {
                console.error(result);
                return null;
              }
              return { code: result, map: null };
            } catch (e) {
              console.error(`Linker failed for ${id}:`, e);
              return null;
            }
          }
        }
        return null;
      }

      if (!id.endsWith('.ts') || id.endsWith('.d.ts')) {
        return null;
      }

      try {
        const result = compiler.compile(id, code);

        if (result.startsWith('/* Error')) {
          console.error(result);
          throw new Error(`Rust Compilation Failed for ${id}`);
        }

        return {
          code: result,
          map: null,
        };
      } catch (err) {
        console.error('Compilation error:', err);
        throw err;
      }
    },
    handleHotUpdate({ file, server, modules }) {
      if (file.endsWith('.html')) {
        const tsFile = file.replace(/\.html$/, '.ts');
        console.log(`[HMR] HTML changed: ${file}`);
        console.log(`[HMR] Invalidate TS: ${tsFile}`);

        const mod = server.moduleGraph.getModuleById(tsFile);
        if (mod) {
          console.log(`[HMR] Found module, invalidating...`);
          server.moduleGraph.invalidateModule(mod);

          server.ws.send({
            type: 'full-reload',
            path: '*',
          });

          return [];
        } else {
          console.log(`[HMR] Module not found in graph`);
          server.ws.send({
            type: 'full-reload',
            path: '*',
          });
          return [];
        }
      }
    },
  };
}

export default defineConfig({
  plugins: [rustNgcPlugin()],
  resolve: {
    extensions: ['.ts', '.js', '.json'],
  },
  server: {
    port: 4300,
  },
  optimizeDeps: {
    // Include Angular packages in pre-bundling
    include: [
      '@angular/core',
      '@angular/common',
      '@angular/platform-browser',
      '@angular/router',
      'zone.js',
      'rxjs',
      'rxjs/operators',
    ],
    // Use esbuild plugin to run linker during pre-bundling
    esbuildOptions: {
      plugins: [angularLinkerEsbuildPlugin()],
    },
  },
  esbuild: false, // We handle TS compilation
});
