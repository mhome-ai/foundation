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

export async function randomU64(): Promise<bigint> {
  return await call("random.u64", []) as bigint;
}

/** Read an artifact owned by the invocation's current Space into Uint8Array. */
export async function readArtifact(reference: Image | string, maxBytes = 8 * 1024 * 1024): Promise<Uint8Array> {
  return await call("artifact.read", [{ uri: typeof reference === "string" ? reference : reference.uri, maxBytes }]) as Uint8Array;
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
