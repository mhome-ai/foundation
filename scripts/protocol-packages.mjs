export const PROTOCOL_PACKAGES = Object.freeze({
  artifact: Object.freeze({
    crateDir: "crates/artifact",
    npmName: "@mhome/artifact-protocol",
    protocol: "mhome.artifact",
    manifest: "manifest/artifact.v1.json",
    schemas: Object.freeze({
      artifactMetadata: "schema/artifact-metadata.v1.schema.json",
      artifactResolve: "schema/artifact-resolve.v1.schema.json",
      artifactPut: "schema/artifact-put.v1.schema.json",
    }),
    fixtures: Object.freeze({
      artifactReference: "fixtures/artifact-reference.conformance.json",
      artifactResolve: "fixtures/artifact-resolve.conformance.json",
      artifactPut: "fixtures/artifact-put.conformance.json",
    }),
  }),
  conversation: Object.freeze({
    crateDir: "crates/conversation-api",
    npmName: "@mhome/conversation-protocol",
    protocol: "mhome.conversation",
    manifest: "manifest/targets.v1.json",
    schemas: Object.freeze({
      frame: "schema/conversation-frame.v2.schema.json",
    }),
    fixtures: Object.freeze({
      valid: "fixtures",
    }),
  }),
  messaging: Object.freeze({
    crateDir: "crates/messaging-api",
    npmName: "@mhome/messaging-protocol",
    protocol: "mhome.messaging",
    manifest: "manifest/targets.v1.json",
    schemas: Object.freeze({
      frame: "schema/messaging-frame.v3.schema.json",
      normalizedInbound: "schema/normalized-inbound.v4.schema.json",
      messagingCommands: "schema/messaging-commands.v1.schema.json",
    }),
    fixtures: Object.freeze({
      valid: "fixtures",
      invalid: "fixtures/invalid",
      normalizedInbound: "fixtures/normalized-inbound.conformance.json",
      messagingCommands: "fixtures/messaging-commands.conformance.json",
    }),
  }),
  plugin: Object.freeze({
    crateDir: "crates/plugin-api",
    npmName: "@mhome/plugin-protocol",
    protocol: "mhome.plugin",
    manifest: "contract/node-protocol-v1.json",
    schemas: Object.freeze({}),
    fixtures: Object.freeze({}),
  }),
});

export function protocolPackage(name) {
  const config = PROTOCOL_PACKAGES[name];
  if (!config) {
    throw new Error(
      `Unknown protocol package ${JSON.stringify(name)}; expected one of ${Object.keys(
        PROTOCOL_PACKAGES
      ).join(", ")}`
    );
  }
  return config;
}
