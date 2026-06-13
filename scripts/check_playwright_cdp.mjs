const endpoint = process.env.PLAYWRIGHT_CDP_ENDPOINT ?? "http://127.0.0.1:9222";
const url = new URL("/json/version", endpoint.endsWith("/") ? endpoint : `${endpoint}/`);

const response = await fetch(url);
if (!response.ok) {
  throw new Error(`CDP endpoint ${url.href} returned HTTP ${response.status}`);
}

const payload = await response.json();
console.log(
  JSON.stringify(
    {
      endpoint,
      browser: payload.Browser,
      protocolVersion: payload["Protocol-Version"],
      webSocketDebuggerUrl: payload.webSocketDebuggerUrl,
    },
    null,
    2,
  ),
);
