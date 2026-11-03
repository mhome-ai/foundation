/** Authoring helpers only. Privileged host modules are installed by the runtime. */
import { call } from "@meowlink/script/host";
export const SDK_VERSION = "0.1.0";

// Preserve the existing platform's explicit domain-object shape. Values are in
// the stated unit; adapters perform any conversion to Core's canonical units.
export interface Temperature {
  readonly _type: "Temperature";
  readonly value: number;
  readonly unit: "C" | "F" | "K";
}

export interface Duration {
  readonly _type: "Duration";
  readonly value: number;
  readonly unit: "ms" | "sec" | "min" | "hour" | "day";
}

export interface Timestamp {
  readonly _type: "Timestamp";
  readonly value: number | bigint;
}

export interface Image {
  readonly uri: string;
}

export interface CalendarDate {
  readonly _type: "DateGeneral";
  readonly value: string;
}

export interface ClockTime {
  readonly _type: "TimeGeneral";
  readonly value: string;
}

export interface LocalDateTime {
  readonly _type: "LocalDateTime";
  readonly value: string;
}

/** Dynamic business objects remain objects; no generated nested struct is required. */
export interface ScriptObject {
  [key: string]: Value;
}

export type Value = null | boolean | number | bigint | string
  | Temperature | Duration | Timestamp | Image | CalendarDate | ClockTime | LocalDateTime
  | Uint8Array | ScriptObject | Value[];

export interface RuntimeContext {
  readonly tenantId: string;
  readonly scopeId: string;
  readonly integrationId: string | null;
  readonly connectId: string | null;
  readonly operationId: string | null;
  readonly stableId: string;
}

/** Identity is supplied by the host. Arguments cannot replace tenant or Space. */
export async function getRuntimeContext(): Promise<RuntimeContext> {
  return await call("runtime.context", []) as unknown as RuntimeContext;
}

/** Awaited request work: this never obtains a new deadline or launches a timer task. */
export async function sleep(milliseconds: number): Promise<void> {
  await call("time.sleep", [milliseconds]);
}

/** Bounded diagnostic logging, attributed by Core to the current invocation. */
export async function log(message: string, fields?: ScriptObject, level: "debug" | "info" | "warn" | "error" = "info"): Promise<void> {
  await call("log.write", [{ message, level, ...(fields === undefined ? {} : { fields }) }]);
}

export async function randomU64(): Promise<bigint> {
  return await call("random.u64", []) as bigint;
}

/** Process-local request IDs, isolated by the current Space/Integration/connection.
 * Reuse a stable namespace; counters reset on process restart, not per invocation.
 */
export async function nextSequence(namespace: string): Promise<number> {
  return await call("sequence.next", [namespace]) as number;
}

export interface EndpointKey { category: string; subscriptionId: string }
/** Store the opaque endpoint supplied by Core when subscribing. This is not a cache. */
export async function mapEndpoint(request: EndpointKey & { endpoint: string }): Promise<void> {
  await call("endpoint.map", [request as unknown as Value]);
}
export async function getEndpoint(key: EndpointKey): Promise<string | null> {
  return await call("endpoint.get", [key as unknown as Value]) as string | null;
}
/** Explicitly remove a mapping and return its previous endpoint, or null. */
export async function unmapEndpoint(key: EndpointKey): Promise<string | null> {
  return await call("endpoint.unmap", [key as unknown as Value]) as string | null;
}
/** The callback origin comes from Core; scripts cannot select another Host. */
export async function webhookUrl(endpoint: string): Promise<string> {
  return await call("webhook.url", [{ endpoint }]) as string;
}

/** Read an artifact owned by the invocation's current Space into Uint8Array. */
export async function readArtifact(reference: Image | string, maxBytes = 8 * 1024 * 1024): Promise<Uint8Array> {
  return await call("artifact.read", [{ uri: typeof reference === "string" ? reference : reference.uri, maxBytes }]) as Uint8Array;
}

/** Storage identity and retention use the platform Artifact policy. */
export interface ArtifactPolicy {
  identity?: "auto" | "man";
  retention?: "std" | "tmp";
  creationId?: string;
}
export interface StoredArtifact extends Image {
  identity: "auto" | "man";
  retention: "std" | "tmp";
  kind: "IMAGE" | "AUDIO" | "FILE";
  mimeType: string;
  sizeBytes: number;
  expiresAtUnixMs?: number;
  width?: number;
  height?: number;
  durationMillis?: number;
}
export interface PutArtifact extends ArtifactPolicy {
  kind: "IMAGE" | "AUDIO" | "FILE";
  mimeType: string;
  data: Uint8Array;
  durationMillis?: number;
}
export async function putArtifact(request: PutArtifact): Promise<StoredArtifact> {
  return await call("artifact.put", [request as unknown as Value]) as unknown as StoredArtifact;
}
export interface ImportArtifact extends ArtifactPolicy {
  kind: "IMAGE" | "AUDIO" | "FILE";
  sourceUrl: string;
  mimeType?: string;
}
export async function importArtifact(request: ImportArtifact): Promise<StoredArtifact> {
  return await call("artifact.import", [request as unknown as Value]) as unknown as StoredArtifact;
}
/** Resolve a scoped reference to a temporary delivery URL; no storage credentials or local paths. */
export async function resolveArtifact(request: Image & { destination?: "thirdParty" }): Promise<{
  uri: string; mimeType: string; url: string; expiresAtUnixMs: number; hostIdentity: Value;
}> {
  return await call("artifact.resolve", [{ uri: request.uri, ...(request.destination === undefined ? {} : { destination: request.destination }) }]) as {
    uri: string; mimeType: string; url: string; expiresAtUnixMs: number; hostIdentity: Value;
  };
}
/** Explicit deletion only. A failed script does not roll back committed artifacts. */
export async function deleteArtifact(reference: Image | string): Promise<void> {
  await call("artifact.delete", [{ uri: typeof reference === "string" ? reference : reference.uri }]);
}

/** Per-call limits can shorten, but never extend, the invocation deadline. */
export interface HttpRequest {
  method: string;
  url: string;
  headers?: Record<string, string>;
  body?: { kind: "text" | "json" | "base64"; value: string };
  timeoutMillis?: number;
  connectTimeoutMillis?: number;
  followRedirects?: boolean;
  maxBodyBytes?: number;
  tls?: { verify?: boolean };
}
export interface HttpResponse {
  status: number;
  headers: Record<string, string>;
  body: { kind: "text" | "json" | "base64"; value: string };
  truncated: boolean;
}
/** HTTP error status is a response; connection/host failure rejects the Promise. */
export async function httpRequest(request: HttpRequest): Promise<HttpResponse> {
  return await call("http.request", [request as unknown as Value]) as unknown as HttpResponse;
}
export interface HttpReadBatch {
  requests: (Omit<HttpRequest, "method" | "body" | "maxBodyBytes"> & { method: "GET"; maxBodyBytes: number })[];
  concurrency?: number;
  cacheTtlMillis?: number;
}
export type HttpReadResult = ({ ok: true } & HttpResponse) | { ok: false; error: HostError };
/** Ordered read results; bounded cache belongs to the current Space/connection. */
export async function httpReadBatch(request: HttpReadBatch): Promise<HttpReadResult[]> {
  return await call("http.batch", [request as unknown as Value]) as unknown as HttpReadResult[];
}

export interface NodeRequest {
  nodeType: string;
  target: string;
  payload?: Value;
  timeoutMillis?: number;
}
/** Routes only to the Node bound to this Integration connection. */
export async function requestNode(request: NodeRequest): Promise<Value> {
  return await call("node.request", [request as unknown as Value]);
}

/** Connect-scoped IDs come from runtime data; entity targets also supply their entity ID. */
export interface ChannelTarget { id: string; entityId?: string }
export interface ChannelRequest extends ChannelTarget { payload: Value }
/** The host enforces the current tenant, Space, Integration and connection. */
export async function requestChannel(request: ChannelRequest): Promise<Value> {
  return await call("channel.request", [request as unknown as Value]);
}
export async function sendChannel(request: ChannelRequest): Promise<void> {
  await call("channel.send", [request as unknown as Value]);
}
export async function isChannelReady(target: ChannelTarget): Promise<boolean> {
  return await call("channel.status", [target as unknown as Value]) as boolean;
}
/** Explicit business action. A timeout never implicitly disconnects a channel. */
export async function disconnectChannel(target: ChannelTarget): Promise<void> {
  await call("channel.disconnect", [target as unknown as Value]);
}
/** Available only when Core invokes a channel hook. */
export async function getChannelContext(): Promise<Value> {
  return await call("channel.context", []);
}

export interface DnsRequest {
  host: string;
  port?: number;
  family?: "any" | "ipv4" | "ipv6";
  maxResults?: number;
}
export interface DnsResult {
  host: string;
  addresses: { ip: string; family: "ipv4" | "ipv6" }[];
}
export async function dnsLookup(request: DnsRequest): Promise<DnsResult> {
  return await call("dns.lookup", [request as unknown as Value]) as unknown as DnsResult;
}
export async function resolveMdns(request: Omit<DnsRequest, "port"> & { timeoutMillis?: number }): Promise<DnsResult> {
  return await call("mdns.resolve", [request as unknown as Value]) as unknown as DnsResult;
}

export interface MdnsBrowseRequest {
  serviceTypes: string[];
  timeoutMillis?: number;
  maxResults?: number;
}
/** Discovery returns the native record fields, including provider-specific TXT records. */
export async function browseMdns(request: MdnsBrowseRequest): Promise<ScriptObject> {
  return await call("mdns.browse", [request as unknown as Value]) as ScriptObject;
}
export interface GoogleCastRequest {
  host: string;
  port?: number;
  action: "playMedia" | "getStatus" | "setVolume" | "setMuted" | "mediaControl";
  timeoutMillis?: number;
  tls?: { verify?: boolean };
  media?: { url: string; contentType?: string; streamType?: string; metadata?: Value };
  volume?: number;
  command?: string;
  muted?: boolean;
}
/** Native Cast transport; provider schemas and functions own the business mapping. */
export async function requestGoogleCast(request: GoogleCastRequest): Promise<ScriptObject> {
  return await call("googleCast.request", [request as unknown as Value]) as ScriptObject;
}

export interface HostError { code: string; message: string }
export type ProbeResult<T, F> = { ok: true; data: T }
  | { ok: false; error: HostError; data: F };
export interface TcpProbeRequest {
  host: string;
  port: number;
  timeoutMillis?: number;
  readBanner?: boolean;
  bannerBytes?: number;
}
export interface TcpProbeData {
  host: string;
  port: number;
  remoteAddr: string;
  reachable: true;
  connectMillis: number;
  banner: string | null;
}
export interface TcpProbeFailure {
  host: string;
  port: number;
  reachable: false;
  remoteAddr?: string;
  connectMillis?: number;
}
/** Unreachable devices return diagnostics; request cancellation still rejects. */
export async function probeTcp(request: TcpProbeRequest): Promise<ProbeResult<TcpProbeData, TcpProbeFailure>> {
  return await call("tcp.probe", [request as unknown as Value]) as unknown as ProbeResult<TcpProbeData, TcpProbeFailure>;
}
export interface RtspProbeRequest {
  url?: string;
  urls?: string[];
  username?: string;
  password?: string;
  timeoutMillis?: number;
  concurrency?: number;
}
export interface RtspProbeData {
  url: string;
  streamCount: number;
  videoCodecs: string[];
  audioCodecs: string[];
}
export type RtspProbeResult = ProbeResult<RtspProbeData, { url: string }>;
/** A single distinct URL returns one outcome; multiple URLs return data.results. */
export async function probeRtsp(request: RtspProbeRequest): Promise<RtspProbeResult | { ok: true; data: { results: RtspProbeResult[] } }> {
  return await call("rtsp.probe", [request as unknown as Value]) as unknown as RtspProbeResult | { ok: true; data: { results: RtspProbeResult[] } };
}

export interface EncodedBytes { encoding?: "utf8" | "text" | "hex" | "base64"; value: string }
export interface CryptoRequest {
  op: "digest" | "hmac" | "aeadEncrypt" | "aeadDecrypt" | "cipherEncrypt" | "cipherDecrypt" | "generateRsaKey" | "certificateSigningRequest";
  algorithm?: string;
  input?: EncodedBytes;
  key?: EncodedBytes;
  nonce?: EncodedBytes;
  iv?: EncodedBytes;
  output?: "utf8" | "text" | "hex" | "base64";
  combinedInput?: string;
  combinedOutput?: string;
  nonceLengthBytes?: number;
  ivLengthBytes?: number;
  padding?: string;
  commonName?: string;
}
/** Bounded native computation, awaited within the caller's deadline. */
export async function cryptoRequest(request: CryptoRequest): Promise<{ encoding: string; value: string }> {
  return await call("crypto.request", [request as unknown as Value]) as { encoding: string; value: string };
}
export interface QrCodeRequest {
  value: string;
  encoding?: "text";
  output?: "svg" | "svgDataUrl";
  ecc?: "low" | "medium" | "quartile" | "high";
  size?: number;
  margin?: number;
  darkColor?: string;
  lightColor?: string;
}
export type QrCodeResult = { mime: "image/svg+xml"; width: number; height: number }
  & ({ output: "svg"; svg: string } | { output: "svgDataUrl"; dataUrl: string });
export async function renderQrCode(request: QrCodeRequest): Promise<QrCodeResult> {
  return await call("qr.render", [request as unknown as Value]) as unknown as QrCodeResult;
}

/** Temporary transport handles belong to this invocation, never a later call. */
export interface TransportSession { sessionId: string }
export interface WebSocketOpen {
  url: string;
  headers?: Record<string, string>;
  protocols?: string[];
  connectTimeoutMillis?: number;
  /** Hard receive allocation limit, set when opening (1 KiB–16 MiB). */
  maxMessageBytes?: number;
  tls?: { verify?: boolean };
}
export interface WebSocketOpened extends TransportSession {
  status: number;
  headers: Record<string, string>;
  protocol: string;
}
export type WebSocketMessage = { type: "text" | "base64"; value: string }
  | { type: "json"; value: Value };
export async function openWebSocket(request: WebSocketOpen): Promise<WebSocketOpened> {
  return await call("websocket.open", [request as unknown as Value]) as unknown as WebSocketOpened;
}
/** Await operations on each handle sequentially. Concurrent use rejects. */
export async function sendWebSocket(request: TransportSession & { message: WebSocketMessage; timeoutMillis?: number }): Promise<void> {
  await call("websocket.send", [request as unknown as Value]);
}
export async function receiveWebSocket(request: TransportSession & { timeoutMillis?: number }): Promise<{ sessionId: string; message: { type: "text" | "base64"; value: string } }> {
  return await call("websocket.receive", [request as unknown as Value]) as { sessionId: string; message: { type: "text" | "base64"; value: string } };
}
export async function closeWebSocket(request: TransportSession & { code?: number; reason?: string }): Promise<void> {
  await call("websocket.close", [request as unknown as Value]);
}
export interface TlsLineOpen {
  host: string;
  port: number;
  connectTimeoutMillis?: number;
  /** Private-CA TLS 1.2 for devices; address/name matching is not required. */
  tls: { caPem: string; certPem?: string; keyPem?: string };
}
export async function openTlsLine(request: TlsLineOpen): Promise<TransportSession> {
  return await call("tlsLine.open", [request as unknown as Value]) as unknown as TransportSession;
}
export async function sendTlsLine(request: TransportSession & { line: string; timeoutMillis?: number }): Promise<void> {
  await call("tlsLine.send", [request as unknown as Value]);
}
/** A local read timeout preserves partial lines for polling; the caller deadline still applies. */
export async function receiveTlsLine(request: TransportSession & { timeoutMillis?: number }): Promise<{ sessionId: string; line: string }> {
  return await call("tlsLine.receive", [request as unknown as Value]) as { sessionId: string; line: string };
}
export async function closeTlsLine(request: TransportSession): Promise<void> {
  await call("tlsLine.close", [request as unknown as Value]);
}

export interface ScalarValues {
  bool: boolean;
  number: number;
  string: string;
  timestamp: Timestamp;
  duration: Duration;
  temperature: Temperature;
  image: Image;
  object: ScriptObject;
  value: Value;
}

export type Descriptor = keyof ScalarValues | { readonly list: Descriptor };
export type Fields = Readonly<Record<string, Descriptor>>;
export type Infer<D extends Descriptor> = D extends keyof ScalarValues
  ? ScalarValues[D]
  : D extends { readonly list: infer E extends Descriptor }
    ? Infer<E>[]
    : never;
export type InferFields<F extends Fields> = {
  -readonly [K in keyof F]: Infer<F[K]>;
};

export const t = Object.freeze({
  bool: "bool" as const,
  number: "number" as const,
  string: "string" as const,
  timestamp: "timestamp" as const,
  duration: "duration" as const,
  temperature: "temperature" as const,
  image: "image" as const,
  object: "object" as const,
  value: "value" as const,
  list: <const D extends Descriptor>(element: D): { readonly list: D } =>
    Object.freeze({ list: element }),
});

export interface ScriptDefinition<I extends Fields, O extends Fields> {
  readonly input: I;
  readonly output: O;
  readonly run: (input: InferFields<I>) =>
    NoInfer<InferFields<O>> | Promise<NoInfer<InferFields<O>>>;
}

/**
 * One declaration drives TS inference and build-generated runtime metadata.
 * The build tool reads input/output statically; this helper does not execute run
 * to discover types, register metadata globally or start background work.
 */
export function defineScript<const I extends Fields, const O extends Fields>(
  definition: ScriptDefinition<I, O>,
): (input: InferFields<I>) => InferFields<O> | Promise<InferFields<O>> {
  const run = definition.run;
  return (input) => run(input);
}

export class ScriptError extends Error {
  readonly code: string;

  constructor(code: string, message: string) {
    super(message);
    this.name = "ScriptError";
    this.code = code;
  }
}
