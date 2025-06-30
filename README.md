`smallweb-adapter` is a Deno command-line adapter that allows launching non-Deno applications under SmallWeb, while retaining the same or stronger security guarantees via `bubblewrap`.

It works by being placed in the `PATH` as `deno`. It inspects the command-line arguments intended for Deno and can decide to run a different command if the application's entrypoint is named main.tsx and is actually {exec:"some cmdline"} json ([example](test/invoke_adapter/main.tsx)).

smallweb launches deno like:

```sh
/usr/local/bin/deno run --allow-net --allow-import --allow-env --allow-sys --allow-ffi --unstable-kv --unstable-otel --unstable-temporal --node-modules-dir=none --no-prompt --quiet --allow-read=/home/taras/smallweb/post,/usr/local/bin/deno,/home/taras/.cache/deno/npm/registry.npmjs.org --allow-write=/home/taras/smallweb/post/data - '{"command":"fetch","entrypoint":"file:///home/taras/smallweb/post/main.ts","port":38025}'
```

Our rust smallweb-adapter will also be named deno and be first in path:
- Parse the json in last arg
- check that entrypoint ends in main.tsx
- read first byte of that file, if it's '{', attempt to parse it as json
- if main.tsx fails to parse as json:
 * look at PATH..remove first element of it(using ; separator), set it as env var
 * exec deno  as last thing it does
- if main.tsx does parse
 * schema of it is {exec: "bash cmd with $PORT"}
 * execute the command in `exec`, with `$PORT` available as an environment variable.


# Security

To enhance security, `smallweb-adapter` *always* uses [bubblewrap](https://github.com/containers/bubblewrap) to create a sandboxed environment for executing non-Deno applications. It starts with a restrictive baseline configuration and translates Deno's permission flags into additional `bubblewrap` arguments to selectively relax restrictions.

Here's how the flags are mapped:

- `--allow-net` is translated to `bwrap --share-net`.
- `--allow-read=<path>` is translated to `bwrap --ro-bind <path> <path>`.
- `--allow-write=<path>` is translated to `bwrap --bind <path> <path>`.


# Debugging

This application logs to `$SMALLWEB_APP_DIR/logs/smallweb-wrapper.log`. To enable logging, you must first create the `logs` directory inside your SmallWeb application directory, for example:

```sh
mkdir -p /path/to/your/smallweb-app/logs
```

If the `logs` directory does not exist, or if the log file cannot be written to, logging will be silently disabled.

The debug logs will start with the current working directory and the command-line arguments quoted for easy shell reuse.


# Tests

```
cargo test -- --nocapture
```
