import { build } from "esbuild";
import { builtinModules } from "node:module";

const nodeBuiltins = builtinModules.flatMap((mod) => [mod, `node:${mod}`]);

await build({
  entryPoints: ["src/cli.tsx"],
  bundle: true,
  platform: "node",
  format: "esm",
  outfile: "dist/goose-text.js",
  banner: {
    js: "#!/usr/bin/env node",
  },
  external: [...nodeBuiltins],
  alias: Object.fromEntries(builtinModules.map((mod) => [mod, `node:${mod}`])),
  plugins: [
    {
      name: "stub-react-devtools",
      setup(build) {
        build.onResolve({ filter: /^react-devtools-core$/ }, () => ({
          path: "react-devtools-core",
          namespace: "stub",
        }));
        build.onLoad({ filter: /.*/, namespace: "stub" }, () => ({
          contents: "export default {}",
        }));
      },
    },
  ],
});

console.log("✅ Built dist/goose-text.js");
