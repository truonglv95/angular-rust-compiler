const path = require('path');
const rspack = require('@rspack/core');
const { spawnSync } = require('child_process');

// class RustNgcPlugin {
//   apply(compiler) {
//     // Use beforeCompile to ensure it runs before any compilation starts (both run and watch)
//     compiler.hooks.beforeCompile.tap('RustNgcPlugin', (params) => {
//       console.log('[RustNgcPlugin] Running Rust Angular Compiler...');
//       const ngcPath = path.resolve(__dirname, '../target/debug/ngc');

//       // Ensure out-tsc exists
//       spawnSync('mkdir', ['-p', 'out-tsc'], { cwd: __dirname });

//       const result = spawnSync(ngcPath, ['-p', 'tsconfig.app.json'], {
//         cwd: __dirname,
//         stdio: 'inherit',
//       });

//       if (result.status !== 0) {
//         throw new Error('[RustNgcPlugin] Compilation failed');
//       }
//       console.log('[RustNgcPlugin] Compilation successful');
//     });
//   }
// }

module.exports = {
  mode: 'development',
  entry: './src/main.ts',
  // entry: './out-tsc/main.js',
  output: {
    path: path.resolve(__dirname, 'dist/rspack'),
    filename: 'main.js',
  },
  resolve: {
    extensions: ['.ts', '.js', '.mjs'],
  },
  module: {
    rules: [
      // {
      //   test: /\.mjs$/,
      //   use: [
      //     {
      //       loader: path.resolve(__dirname, 'linker-loader.js'),
      //     },
      //   ],
      //   type: 'javascript/auto',
      // },
      {
        test: /\.ts$/,
        use: [
          {
            loader: path.resolve(__dirname, 'rust-ngc-loader.js'),
          },
        ],
        // Exclude node_modules to avoid trying to compile libs with our custom loader
        // (unless we want to processing libs too, but usually libs are already JS/d.ts)
        exclude: /node_modules/,
      },
    ],
  },
  plugins: [
    // new RustNgcPlugin(), // Disabled in favor of NAPI loader
    new rspack.HtmlRspackPlugin({
      template: './src/index.html',
    }),
  ],
  watchOptions: {
    ignored: ['**/out-tsc/**', '**/rust-output/**'],
  },
};
