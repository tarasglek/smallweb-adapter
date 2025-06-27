export default {
  fetch(req) {
    return new Response("Hello world");
  }
} satisfies Deno.ServeDefaultExport;
