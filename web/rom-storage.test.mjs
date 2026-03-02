import test from "node:test";
import assert from "node:assert/strict";

import {
  cloneRomBytes,
  createSavedRomRecord,
  parseSavedRomRecord,
} from "./rom-storage.mjs";

test("cloneRomBytes returns an independent copy", () => {
  const original = new Uint8Array([0x4e, 0x45, 0x53, 0x1a]);
  const copy = cloneRomBytes(original);
  assert.deepEqual(Array.from(copy), Array.from(original));
  original[0] = 0xff;
  assert.equal(copy[0], 0x4e);
});

test("createSavedRomRecord stores normalized metadata and copied bytes", () => {
  const bytes = new Uint8Array([1, 2, 3, 4]);
  const record = createSavedRomRecord(bytes, " SMB.nes ", "file", 1234.9);

  assert.equal(record.key, "last-rom");
  assert.equal(record.name, "SMB.nes");
  assert.equal(record.source, "file");
  assert.equal(record.savedAtMs, 1234);
  assert.deepEqual(Array.from(record.bytes), [1, 2, 3, 4]);

  bytes[1] = 9;
  assert.equal(record.bytes[1], 2);
});

test("parseSavedRomRecord rejects malformed records", () => {
  assert.equal(parseSavedRomRecord(null), null);
  assert.equal(parseSavedRomRecord({}), null);
  assert.equal(parseSavedRomRecord({ bytes: new Uint8Array() }), null);
  assert.equal(parseSavedRomRecord({ bytes: "not-bytes" }), null);
});

test("parseSavedRomRecord normalizes metadata for valid records", () => {
  const parsed = parseSavedRomRecord({
    bytes: new Uint8Array([9, 8, 7]),
    name: "  ",
    source: "",
    savedAtMs: Number.NaN,
  });

  assert.ok(parsed);
  assert.equal(parsed.name, "Unnamed ROM");
  assert.equal(parsed.source, "file");
  assert.ok(Number.isFinite(parsed.savedAtMs));
  assert.deepEqual(Array.from(parsed.bytes), [9, 8, 7]);
});
