import { defineConfig } from 'vite';
import { createRequire } from 'module';
import { fileURLToPath } from 'url';
import path from 'path';

const require = createRequire(import.meta.url);
const __dirname = path.dirname(fileURLToPath(import.meta.url));

// Resolve binding path relative to this file
// demo-app/vite.config.mjs -> packages/binding
const bindingPath = path.resolve(__dirname, '../packages/binding');
const { Compiler } = require(bindingPath);

const compiler = new Compiler();

function rustNgcPlugin() {
  return {
    name: 'rust-ngc-plugin',
    enforce: 'pre',
    transform(code, id) {
      if (id.includes('node_modules')) {
        if (id.includes('@angular')) {
          console.log(`[Vite Plugin] Angular file detected: ${id}`);
        }
        // Linker logic for Angular packages
        const cleanId = id.split('?')[0];
        if ((cleanId.endsWith('.mjs') || cleanId.endsWith('.js')) && id.includes('@angular')) {
          console.log(`[Linker] Processing: ${id}`);
          try {
            const result = compiler.linkFile(id, code);
            if (result.startsWith('/* Linker Error')) {
              console.error(result);
              return null;
            }
            return {
              code: result,
              map: null,
            };
          } catch (e) {
            console.error(`Linker failed for ${id}:`, e);
            return null;
          }
        }
        return null; // Skip other node_modules
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
          // Force reload anyway as fallback
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
    port: 4200,
  },
  optimizeDeps: {
    // Exclude Angular packages that might fail optimization or need specific handling
    exclude: ['@angular/core', '@angular/common', '@angular/platform-browser', '@angular/router'],
  },
  esbuild: false, // We handle TS compilation
});
