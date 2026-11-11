import assert from "node:assert/strict";
import { mkdtemp, mkdir, readFile, rm, symlink, writeFile } from "node:fs/promises";
import { createRequire } from "node:module";
import os from "node:os";
import path from "node:path";
import test from "node:test";

const require = createRequire(import.meta.url);
const signRequire = createRequire(require.resolve("@sigstore/sign"));
const fetchRequire = createRequire(signRequire.resolve("make-fetch-happen"));
const cacache = fetchRequire("cacache");
const sigstoreRequire = createRequire(require.resolve("sigstore"));
const tufRequire = createRequire(sigstoreRequire.resolve("@sigstore/tuf"));
const tufJsRequire = createRequire(tufRequire.resolve("tuf-js"));
const { Targets } = tufJsRequire("@tufjs/models");

// cacache 20 still requests glob 13, and TUF models 4 still requests minimatch
// 10. Guard the public behavior behind the cacache-only glob override and the
// minimatch 10/11 override shared by these two consumers. Older matcher API
// families are deliberately outside the override.
test("TUF delegation continues to enforce target names and path boundaries", () => {
  for (const [pattern, target, expected] of [
    ["*.json", "trusted_root.json", true],
    ["*.json", "nested/trusted_root.json", false],
    ["*.json", ".hidden.json", false],
    ["*.json", "trusted_root.json.exe", false],
    ["targets/*.json", "targets/root.json", true],
    ["targets/*.json", "other/root.json", false],
    ["targets/*.json", "targets/nested/root.json", false],
    ["targets/?.json", "targets/a.json", true],
    ["targets/?.json", "targets/ab.json", false],
    ["targets/[ab].json", "targets/a.json", true],
    ["targets/[ab].json", "targets/c.json", false],
    ["targets/{a,b}.json", "targets/b.json", true],
    ["targets/{a,b}.json", "targets/c.json", false],
    ["targets/@(public|root).json", "targets/public.json", true],
    ["targets/@(public|root).json", "targets/private.json", false],
    ["targets/root.json", "targets/ROOT.json", false],
    ["targets/设备.json", "targets/设备.json", true],
  ]) {
    const targets = Targets.fromJSON({
      _type: "targets",
      spec_version: "1.0.31",
      version: 1,
      expires: "2099-01-01T00:00:00Z",
      targets: {},
      delegations: {
        keys: {},
        roles: [{
          name: "release-targets",
          keyids: ["test-key"],
          threshold: 1,
          terminating: true,
          paths: [pattern],
        }],
      },
    });
    assert.deepEqual(
      [...targets.delegations.rolesForTarget(target)],
      expected ? [{ role: "release-targets", terminating: true }] : [],
      `${pattern}: ${target}`,
    );
  }
});

test("signing cache verification preserves live data and removes corrupt and orphaned data", async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), "foundation-cache-"));
  const cache = path.join(root, "cache with spaces");
  try {
    await cacache.put(cache, "live", Buffer.from("verified signing data"));
    const orphan = await cacache.put(cache, "orphan", Buffer.from("unreferenced data"));
    await cacache.rm.entry(cache, "orphan");
    await cacache.put(cache, "corrupt", Buffer.from("original data"));
    const corrupt = await cacache.get.info(cache, "corrupt");
    await writeFile(corrupt.path, "tampered data");
    const stats = await cacache.verify(cache);
    assert.equal(stats.verifiedContent, 1);
    assert.equal(stats.badContentCount, 1);
    assert.equal(stats.reclaimedCount, 2);
    assert.equal((await cacache.get(cache, "live")).data.toString(), "verified signing data");
    await assert.rejects(cacache.get(cache, "corrupt"), { code: "ENOENT" });
    await assert.rejects(cacache.get.byDigest(cache, orphan), { code: "ENOENT" });
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test("signing cache removal stays within cache directories and does not follow symlinks", async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), "foundation-cache-remove-"));
  const cache = path.join(root, "cache");
  const outside = path.join(root, "outside");
  try {
    await mkdir(outside);
    await writeFile(path.join(outside, "keep.json"), "outside data");
    await cacache.put(cache, "live", Buffer.from("cached data"));
    await writeFile(path.join(cache, "keep.json"), "unrelated cache-root file");
    await symlink(outside, path.join(cache, "content-link"), "dir");
    await cacache.rm.all(cache);
    assert.deepEqual(await cacache.ls(cache), {});
    assert.equal(await readFile(path.join(outside, "keep.json"), "utf8"), "outside data");
    assert.equal(await readFile(path.join(cache, "keep.json"), "utf8"), "unrelated cache-root file");
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});
