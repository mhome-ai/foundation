import assert from "node:assert/strict";
import fs from "node:fs";
import test from "node:test";

import { renderAppFacadeRouting } from "../generate-app-facade-routing.mjs";

test("App Facade routing regenerates the checked-in Rust bindings exactly", () => {
  const manifest = JSON.parse(fs.readFileSync(new URL(
    "../../crates/app-facade-api/manifest/routing.v1.json", import.meta.url
  ), "utf8"));
  const generated = fs.readFileSync(new URL(
    "../../crates/app-facade-api/src/routing.generated.rs", import.meta.url
  ), "utf8");
  assert.equal(renderAppFacadeRouting(manifest), generated);
});
