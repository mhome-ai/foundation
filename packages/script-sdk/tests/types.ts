import { defineScript, t, cryptoRequest, probeTcp, renderQrCode, httpRequest,
  requestChannel, sendChannel, isChannelReady, disconnectChannel, getChannelContext,
  type Value, type Temperature } from "../src/index.js";

async function typedHostResults() {
  const tcp = await probeTcp({host: "127.0.0.1", port: 80});
  if (tcp.ok) {
    const banner: string | null = tcp.data.banner;
    void banner;
  } else {
    const code: string = tcp.error.code;
    void code;
    // @ts-expect-error failed probes do not promise a banner
    tcp.data.banner;
  }
  const qr = await renderQrCode({value: "fixture"});
  if (qr.output === "svg") { const svg: string = qr.svg; void svg; }
  const digest = await cryptoRequest({op: "digest", algorithm: "sha256", input: {value: "abc"}});
  const value: string = digest.value;
  void value;
  // @ts-expect-error host network ports are numbers
  await probeTcp({host: "example.invalid", port: "80"});
  // @ts-expect-error requests cannot replace trusted host identity
  await httpRequest({method: "GET", url: "https://example.invalid", tenantId: "foreign"});
  const reply: Value = await requestChannel({id: "runtime-id", payload: 9007199254740993n});
  const ready: boolean = await isChannelReady({id: "runtime-id"});
  const context: Value = await getChannelContext();
  await sendChannel({id: "runtime-id", entityId: "device", payload: {power: true}});
  await disconnectChannel({id: "runtime-id"});
  void [reply, ready, context];
  // @ts-expect-error ownership comes from the host, never from guest parameters
  await requestChannel({id: "runtime-id", payload: null, connectId: "foreign"});
  // @ts-expect-error an explicit payload, including null, is required
  await sendChannel({id: "runtime-id"});
}

async function nativeCastAndDiscovery() {
  const sdk = await import("../src/index.js");
  const found: Value = await sdk.browseMdns({serviceTypes: ["_googlecast._tcp.local."], timeoutMillis: 4000, maxResults: 64});
  const state: Value = await sdk.requestGoogleCast({host: "receiver.local", action: "getStatus", port: 8009});
  await sdk.log("Album art import failed", {code: "NOT_FOUND"}, "warn");
  void [found, state];
  // @ts-expect-error Cast actions are explicit supported protocol operations
  await sdk.requestGoogleCast({host: "receiver.local", action: "execute"});
  // @ts-expect-error trusted scope cannot be replaced by a host request
  await sdk.browseMdns({serviceTypes: [], scopeId: "foreign"});
}

async function subscriptionRouting() {
  const sdk = await import("../src/index.js");
  const key = {category: "provider.events", subscriptionId: "device|state"};
  await sdk.mapEndpoint({...key, endpoint: "core-provided-route"});
  const endpoint: string | null = await sdk.getEndpoint(key);
  if (endpoint !== null) { const url: string = await sdk.webhookUrl(endpoint); void url; }
  const removed: string | null = await sdk.unmapEndpoint(key);
  const sequence: number = await sdk.nextSequence("device-requests");
  void [removed, sequence];
  // @ts-expect-error mapping values are opaque routes, not arbitrary provider cache objects
  await sdk.mapEndpoint({...key, endpoint: {sleepy: true}});
  // @ts-expect-error Space identity comes from Core
  await sdk.getEndpoint({...key, scopeId: "foreign"});
  // @ts-expect-error webhook origins cannot be selected by a script
  await sdk.webhookUrl({endpoint: "route", callbackBase: "http://foreign.test"});
}

const script = defineScript({
  input: {
    samples: t.list(t.number),
    matrix: t.list(t.list(t.number)),
    settings: t.object,
    dynamic: t.value,
    temperature: t.temperature,
  },
  output: { average: t.number, temperature: t.temperature },
  run({ samples, matrix, settings, dynamic, temperature }) {
    const sample: number | undefined = samples[0];
    const row: number[] | undefined = matrix[0];
    const mode: Value | undefined = settings["mode"];
    const anything: Value = dynamic;
    const original: Temperature = temperature;
    void [sample, row, mode, anything, original];
    return { average: samples.reduce((sum, value) => sum + value, 0), temperature };
  },
});

script({ samples: [1, 2], matrix: [[3]], settings: { nested: { x: 1 } }, dynamic: null,
  temperature: { _type: "Temperature", value: 20, unit: "C" } });

const domain: Temperature = { _type: "Temperature", value: 68, unit: "F" };
const dynamicDomain: Value = { nested: { temperature: domain } };
void dynamicDomain;

// @ts-expect-error named fields, including dynamic Value, are required
script({ samples: [1], matrix: [[2]], settings: {} });
// @ts-expect-error lists must retain element types and nesting
script({ samples: ["bad"], matrix: [1], settings: {}, dynamic: null, temperature: null });

defineScript({ input: { x: t.number }, output: { answer: t.number },
  // @ts-expect-error return type must follow output, not broaden its inference
  run: ({ x }) => ({ answer: String(x) }),
});
defineScript({ input: {}, output: { answer: t.number },
  // @ts-expect-error async output has the same contract
  run: async () => ({ answer: "bad" }),
});
defineScript({ input: {}, output: { items: t.list(t.number) },
  // @ts-expect-error root lists are represented by a named field
  run: () => [1, 2],
});
defineScript({ input: {}, output: { items: t.list(t.number) },
  run: async () => ({ items: [1, 2] }),
});

// @ts-expect-error this SDK does not provide ambient Node APIs
process.env.SECRET;
// @ts-expect-error browser DOM APIs are not part of the script environment
document.cookie;

async function temporaryTransportsAndArtifacts() {
  const sdk = await import("../src/index.js");
  const ws = await sdk.openWebSocket({url: "ws://example.test", maxMessageBytes: 1024});
  await sdk.sendWebSocket({...ws, message: {type: "text", value: "hello"}});
  const received: string = (await sdk.receiveWebSocket(ws)).message.value;
  await sdk.closeWebSocket(ws);
  const tls = await sdk.openTlsLine({host: "example.test", port: 8081, tls: {caPem: "test"}});
  await sdk.sendTlsLine({...tls, line: "hello"});
  const line: string = (await sdk.receiveTlsLine(tls)).line;
  await sdk.closeTlsLine(tls);
  const saved = await sdk.putArtifact({kind: "FILE", mimeType: "text/plain", data: new Uint8Array([1]), retention: "tmp"});
  await sdk.resolveArtifact({...saved, destination: "thirdParty"});
  await sdk.deleteArtifact(saved);
  void [received, line];
  // @ts-expect-error byte data stays binary at the JS boundary
  await sdk.putArtifact({kind: "FILE", mimeType: "text/plain", data: "base64"});
  // @ts-expect-error caller cannot supply a different Space
  await sdk.importArtifact({kind: "FILE", sourceUrl: "https://example.test", scopeId: "foreign"});
  // @ts-expect-error set the hard receive allocation limit when opening
  await sdk.receiveWebSocket({sessionId: "x", maxMessageBytes: 1});
}
