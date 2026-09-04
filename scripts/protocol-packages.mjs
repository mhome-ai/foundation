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
  appFacade: Object.freeze({
    crateDir: "crates/app-facade-api",
    npmName: "@mhome/app-facade-protocol",
    protocol: "mhome.app-facade",
    manifest: "manifest/app-facade.v1.json",
    additionalFiles: Object.freeze(["manifest/targets.v1.json"]),
    schemas: Object.freeze({
      call: "schema/facade-call.v1.schema.json",
      frame: "schema/messaging-frame.v3.schema.json",
      interactionFlow: "schema/interaction-flow-app.v1.schema.json",
    }),
    fixtures: Object.freeze({
      valid: "fixtures",
      invalid: "fixtures/invalid",
      interactionFlow: "fixtures/interaction-flow.conformance.json",
    }),
  }),
  core: Object.freeze({
    crateDir: "crates/core-api",
    npmName: "@mhome/core-protocol",
    protocol: "mhome.core",
    manifest: "manifest/core.v1.json",
    additionalFiles: Object.freeze([
      "contract/node-runtime-protocol-v1.json",
      "contract/node-service-protocol-v1.json",
    ]),
    schemas: Object.freeze({
      normalizedInbound: "schema/normalized-inbound.v4.schema.json",
      messagingCommands: "schema/messaging-commands.v1.schema.json",
      interactionFlowNode: "schema/interaction-flow-node.v1.schema.json",
    }),
    fixtures: Object.freeze({
      normalizedInbound: "fixtures/normalized-inbound.conformance.json",
      messagingCommands: "fixtures/messaging-commands.conformance.json",
      interactionFlowNode: "fixtures/interaction-flow-node.conformance.json",
    }),
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
