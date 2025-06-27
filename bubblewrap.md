bwrap \
  --proc /proc \
  --dev /dev \
  --tmpfs /tmp \
  --ro-bind /usr /usr \
  --ro-bind /etc/resolv.conf /etc/resolv.conf \
  --share-net \
  --ro-bind /home/taras/smallweb/taras-glek-net /home/taras/smallweb/taras-glek-net \
  --bind /home/taras/smallweb/taras-glek-net/data /home/taras/smallweb/taras-glek-net/data \
  --ro-bind /home/taras/Documents/smallweb-adapter/target/release/deno /home/taras/Documents/smallweb-adapter/target/release/deno \
  --ro-bind /home/taras/.cache/d /home/taras/.cache/d \
  -- \
  /home/taras/Documents/smallweb-adapter/target/release/deno run \
    --allow-net \
    --allow-import \
    --allow-env \
    --allow-sys \
    --allow-ffi \
    --unstable-kv \
    --unstable-otel \
    --unstable-temporal \
    --node-modules-dir=none \
    --no-prompt \
    --quiet \
    --allow-read=/home/taras/smallweb/taras-glek-net,/home/taras/Documents/smallweb-adapter/target/release/deno,/home/taras/.cache/d \
    --allow-write=/home/taras/smallweb/taras-glek-net/data \
    - \
    '{"command":"fetch","entrypoint":"file:///home/taras/smallweb/taras-glek-net/main.tsx","port":46791}'
