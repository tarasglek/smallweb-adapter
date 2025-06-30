`smallweb-adapter` is a Deno command-line adapter that allows launching non-Deno applications under SmallWeb, while retaining similar or stronger security guarantees via `bubblewrap`.

It works by being placed in the `PATH` as `deno`, allowing it to intercept commands intended for the Deno runtime. When executed, it inspects the command-line arguments to decide on one of two actions:

1.  **Execute a non-Deno application via `bubblewrap`**: This occurs if the Deno entrypoint is a file (e.g., `main.tsx`) that contains a JSON object with an `exec` key, like `{"exec": "your-command --port $PORT"}`. The adapter will execute the specified command inside a `bubblewrap` sandbox, mapping Deno's security flags to `bubblewrap` arguments. An example of this special entrypoint can be found [here](test/invoke_adapter/main.tsx).

2.  **Execute the original command with the real `deno`**: If the entrypoint is not a special JSON configuration file, the adapter assumes it's a standard Deno application. It finds the real `deno` executable in the system's `PATH` and re-invokes the original command, effectively passing control to the actual Deno runtime.

This logic allows `smallweb-adapter` to act as a transparent wrapper, either launching a sandboxed custom process or deferring to the standard Deno runtime as appropriate.

An example of how SmallWeb launches Deno:

```sh
/usr/local/bin/deno run --allow-net --allow-import --allow-env --allow-sys --allow-ffi --unstable-kv --unstable-otel --unstable-temporal --node-modules-dir=none --no-prompt --quiet --allow-read=/home/web/smallweb/post,/usr/local/bin/deno,/home/web/.cache/deno/npm/registry.npmjs.org --allow-write=/home/web/smallweb/post/data - '{"command":"fetch","entrypoint":"file:///home/web/smallweb/post/main.ts","port":38025}'
```


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
