smallweb launches deno like:

/usr/local/bin/deno run --allow-net --allow-import --allow-env --allow-sys --allow-ffi --unstable-kv --unstable-otel --unstable-temporal --node-modules-dir=none --no-prompt --quiet --allow-read=/home/taras/smallweb/post,/usr/local/bin/deno,/home/taras/.cache/deno/npm/registry.npmjs.org --allow-write=/home/taras/smallweb/post/data - {"command":"fetch","entrypoint":"file:///home/taras/smallweb/post/main.ts","port":38025}\n

Our rust smallweb-adapter will also be named deno and be first in path:
- Parse the json in last arg
- check that entrypoint ends in main.tsx
- read first byte of that file, if it's '{', attempt to parse it as json
- if main.tsx fails to parse as json:
 * look at PATH..remove first element of it(using ; separator), set it as env var
 * exec deno  as last thing it does
- if main.tsx does parse
 * schema of it is {watchpattern: string, exec:"bash cmd with $PORT", build?:"build cmd to run if watchpattern changes"}
 * print the json


# Tests

```
cargo test -- --nocapture
```
