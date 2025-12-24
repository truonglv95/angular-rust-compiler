import { defineConfig } from 'vite';
import { angularCompilerVitePlugin, angularLinkerRolldownPlugin } from 'angular-rust-plugins';

export default defineConfig({
  plugins: [angularLinkerRolldownPlugin(), angularCompilerVitePlugin()],
  resolve: {
    extensions: ['.ts', '.js', '.json'],
  },
  server: {
    port: 4300,
  },
  optimizeDeps: {
    // Exclude Angular packages from pre-bundling so linker plugin can process them
    exclude: [
      '@angular/core',
      '@angular/common',
      '@angular/platform-browser',
      '@angular/router',
      '@angular/forms',
    ],
    // Still include zone.js and rxjs which don't need linking
    include: ['zone.js', 'rxjs', 'rxjs/operators'],
  },
});
