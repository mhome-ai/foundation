import { createHash } from "node:crypto";
import { readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { bundleToJSON } from "@sigstore/bundle";
import {
  CIContextProvider,
  DSSEBundleBuilder,
  FulcioSigner,
  RekorWitness,
} from "@sigstore/sign";
import npa from "npm-package-arg";
import { verify } from "sigstore";

import { PROTOCOL_PACKAGES } from "./protocol-packages.mjs";

export function protocolStatement(name, version, tarball, env) {
  const spec = npa.resolve(name, version);
  if (!Object.values(PROTOCOL_PACKAGES).some((p) => p.npmName === name) || spec.type !== "version") {
    throw new Error("Expected an exact Foundation protocol package version");
  }
  for (const key of ["GITHUB_REPOSITORY", "GITHUB_REF", "GITHUB_SHA", "GITHUB_RUN_ID", "GITHUB_RUN_ATTEMPT"]) {
    if (!env[key]) throw new Error(`Missing ${key} for release provenance`);
  }
  const workflowPath = ".github/workflows/publish-crate.yml";
  if (
    env.GITHUB_ACTIONS !== "true" ||
    env.GITHUB_SERVER_URL !== "https://github.com" ||
    env.RUNNER_ENVIRONMENT !== "github-hosted" ||
    !env.GITHUB_REF.startsWith("refs/tags/") ||
    !/^[a-f0-9]{40}$/.test(env.GITHUB_SHA) ||
    env.GITHUB_WORKFLOW_REF !== `${env.GITHUB_REPOSITORY}/${workflowPath}@${env.GITHUB_REF}`
  ) {
    throw new Error("Protocol provenance requires the tagged GitHub-hosted publication workflow");
  }
  const repository = `${env.GITHUB_SERVER_URL}/${env.GITHUB_REPOSITORY}`;
  // Use npm's GitHub Actions SLSA v1 layout, bound to the final tarball bytes.
  return {
    _type: "https://in-toto.io/Statement/v1",
    subject: [{ name: npa.toPurl(spec), digest: { sha512: createHash("sha512").update(tarball).digest("hex") } }],
    predicateType: "https://slsa.dev/provenance/v1",
    predicate: {
      buildDefinition: {
        buildType: "https://slsa-framework.github.io/github-actions-buildtypes/workflow/v1",
        externalParameters: { workflow: { ref: env.GITHUB_REF, repository, path: workflowPath } },
        internalParameters: { github: {
          event_name: env.GITHUB_EVENT_NAME,
          repository_id: env.GITHUB_REPOSITORY_ID,
          repository_owner_id: env.GITHUB_REPOSITORY_OWNER_ID,
        } },
        resolvedDependencies: [{ uri: `git+${repository}@${env.GITHUB_REF}`, digest: { gitCommit: env.GITHUB_SHA } }],
      },
      runDetails: {
        builder: { id: "https://github.com/actions/runner/github-hosted" },
        metadata: { invocationId: `${repository}/actions/runs/${env.GITHUB_RUN_ID}/attempts/${env.GITHUB_RUN_ATTEMPT}` },
      },
    },
  };
}

export function protocolBundleBuilder({ signer, rekorBaseURL = "https://rekor.sigstore.dev" } = {}) {
  const requests = { retry: { retries: 2 }, timeout: 10000 };
  return new DSSEBundleBuilder({
    signer: signer ?? new FulcioSigner({ identityProvider: new CIContextProvider("sigstore"), ...requests }),
    witnesses: [new RekorWitness({
      rekorBaseURL,
      entryType: "dsse",
      // A successful create can lose its response. Fetch the equivalent entry
      // after a 409, then verify the complete signed bundle before publishing.
      fetchOnConflict: true,
      ...requests,
    })],
  });
}

export async function protocolProvenance(name, version, tarball, {
  env = process.env,
  builder,
  verifyBundle = verify,
} = {}) {
  const statement = protocolStatement(name, version, tarball, env);
  const bundle = bundleToJSON(await (builder ?? protocolBundleBuilder()).create({
    type: "application/vnd.in-toto+json",
    data: Buffer.from(JSON.stringify(statement)),
  }));
  await verifyBundle(bundle, {
    certificateIdentityURI: `^${`${env.GITHUB_SERVER_URL}/${env.GITHUB_WORKFLOW_REF}`.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}$`,
    certificateIssuer: "https://token.actions.githubusercontent.com",
    ctLogThreshold: 1,
    tlogThreshold: 1,
  });
  return bundle;
}

async function main() {
  const [name, version, tarballPath, outputPath] = process.argv.slice(2);
  if (!name || !version || !tarballPath || !outputPath || process.argv.length !== 6) {
    throw new Error("usage: node scripts/protocol-provenance.mjs <name> <version> <tarball> <bundle>");
  }
  const bundle = await protocolProvenance(name, version, await readFile(tarballPath));
  await writeFile(outputPath, `${JSON.stringify(bundle)}\n`, { flag: "wx", mode: 0o600 });
  console.log(`Verified provenance for ${name}@${version}`);
}

if (path.resolve(process.argv[1] || "") === fileURLToPath(import.meta.url)) {
  main().catch((error) => { console.error(error.message); process.exitCode = 1; });
}
