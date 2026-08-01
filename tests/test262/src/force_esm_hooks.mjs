// E19.83.03: Force ESM evaluation for Test262 `.js` fixtures under dynamic
// `import()`. Node otherwise treats files without `import`/`export` as CJS
// (synthetic `default` / `module.exports`), breaking empty-module namespace tests.
import { registerHooks } from "node:module";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

registerHooks({
  load(url, context, nextLoad) {
    if (
      url.startsWith("file:") &&
      url.endsWith(".js") &&
      url.includes("/test262/")
    ) {
      const source = readFileSync(fileURLToPath(url), "utf8");
      return { format: "module", source, shortCircuit: true };
    }
    return nextLoad(url, context);
  },
});
