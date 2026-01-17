import { defineConfig } from 'vite';
import angularRust from '../packages/vite-plugin-angular-rust/src/index.js';
import fs from 'node:fs';
import path from 'node:path';

const packageJson = JSON.parse(fs.readFileSync(path.resolve('package.json'), 'utf-8'));
const optimizeDepsInclude = ['rxjs', 'tslib', 'zone.js', 'rxjs/operators'];
const dependencies = Object.keys(packageJson.dependencies || {});
const angularPackages = dependencies.filter(
  (pkg) => !optimizeDepsInclude.some((include) => pkg === include || pkg.startsWith(include + '/')),
);

console.log('[vite.config.mjs] Packages to exclude from optimization:', angularPackages);
console.log('[vite.config.mjs] CWD:', process.cwd());

// Plugin to output stats similar to Angular CLI
function angularStatsPlugin() {
  return {
    name: 'angular-stats',
    generateBundle(options, bundle) {
      const initialChunks = [];
      const otherChunks = [];
      let initialTotalSize = 0;

      for (const [fileName, chunk] of Object.entries(bundle)) {
        if (chunk.type === 'chunk' && chunk.isEntry) {
          initialChunks.push({
            file: fileName,
            name: chunk.name,
            size: chunk.code.length,
          });
          initialTotalSize += chunk.code.length;
        } else if (
          chunk.type === 'asset' &&
          (fileName.endsWith('.css') || fileName === 'styles.css')
        ) {
          // Treat CSS as initial if it's main styles
          initialChunks.push({
            file: fileName,
            name: chunk.name || fileName.replace(/\.[^/.]+$/, ''),
            size: chunk.source.length,
          });
          initialTotalSize += chunk.source.length;
        } else {
          otherChunks.push(chunk);
        }
      }

      // Helper to format size
      const formatSize = (bytes) => {
        if (bytes < 1024) return `${bytes} bytes`;
        else if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(2)} kB`;
        else return `${(bytes / (1024 * 1024)).toFixed(2)} MB`;
      };

      const padd = (str, len) => str + ' '.repeat(Math.max(0, len - str.length));

      console.log(
        '\n\x1b[1m' +
          padd('Initial chunk files', 30) +
          ' | ' +
          padd('Names', 20) +
          ' | ' +
          'Raw size' +
          '\x1b[0m',
      );
      initialChunks.forEach((chunk) => {
        const sizeStr = formatSize(chunk.size);
        console.log(`${padd(chunk.file, 30)} | ${padd(chunk.name, 20)} | ${sizeStr}`);
      });

      console.log(
        '\n\x1b[1m' +
          padd(' ', 30) +
          ' | ' +
          padd('Initial total', 20) +
          ' | ' +
          formatSize(initialTotalSize) +
          '\x1b[0m\n',
      );
    },
  };
}

export default defineConfig({
  plugins: [angularRust(), angularStatsPlugin()],
  resolve: {
    extensions: ['.ts', '.js', '.json'],
  },
  server: {
    port: 4300,
  },
  optimizeDeps: {
    exclude: angularPackages,
    include: optimizeDepsInclude,
  },
});
