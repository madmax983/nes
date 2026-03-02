const DB_NAME = "nes-web-storage";
const DB_VERSION = 1;
const STORE_NAME = "roms";
const LAST_ROM_KEY = "last-rom";
const DEFAULT_ROM_NAME = "Unnamed ROM";
const DEFAULT_ROM_SOURCE = "file";

export function supportsRomStorage() {
  return typeof indexedDB !== "undefined";
}

export function cloneRomBytes(bytes) {
  if (!(bytes instanceof Uint8Array)) {
    throw new TypeError("expected Uint8Array ROM bytes");
  }
  const copy = new Uint8Array(bytes.byteLength);
  copy.set(bytes);
  return copy;
}

export function createSavedRomRecord(bytes, name, source, savedAtMs = Date.now()) {
  return {
    key: LAST_ROM_KEY,
    name: normalizeRomName(name),
    source: normalizeRomSource(source),
    savedAtMs: normalizeSavedAtMs(savedAtMs),
    bytes: cloneRomBytes(bytes),
  };
}

export function parseSavedRomRecord(rawRecord) {
  if (!rawRecord || typeof rawRecord !== "object") {
    return null;
  }
  if (!(rawRecord.bytes instanceof Uint8Array) || rawRecord.bytes.byteLength === 0) {
    return null;
  }
  return {
    name: normalizeRomName(rawRecord.name),
    source: normalizeRomSource(rawRecord.source),
    savedAtMs: normalizeSavedAtMs(rawRecord.savedAtMs),
    bytes: cloneRomBytes(rawRecord.bytes),
  };
}

export async function saveRomForOfflineUse(bytes, name, source = DEFAULT_ROM_SOURCE) {
  if (!supportsRomStorage()) {
    return false;
  }
  const database = await openRomStorageDatabase();
  try {
    const transaction = database.transaction(STORE_NAME, "readwrite");
    const store = transaction.objectStore(STORE_NAME);
    store.put(createSavedRomRecord(bytes, name, source));
    await waitForTransaction(transaction);
    return true;
  } finally {
    database.close();
  }
}

export async function loadSavedRom() {
  if (!supportsRomStorage()) {
    return null;
  }
  const database = await openRomStorageDatabase();
  try {
    const transaction = database.transaction(STORE_NAME, "readonly");
    const store = transaction.objectStore(STORE_NAME);
    const request = store.get(LAST_ROM_KEY);
    const rawRecord = await waitForRequestResult(request);
    await waitForTransaction(transaction);
    return parseSavedRomRecord(rawRecord);
  } finally {
    database.close();
  }
}

export async function clearSavedRom() {
  if (!supportsRomStorage()) {
    return false;
  }
  const database = await openRomStorageDatabase();
  try {
    const transaction = database.transaction(STORE_NAME, "readwrite");
    const store = transaction.objectStore(STORE_NAME);
    store.delete(LAST_ROM_KEY);
    await waitForTransaction(transaction);
    return true;
  } finally {
    database.close();
  }
}

function normalizeRomName(name) {
  if (typeof name !== "string") {
    return DEFAULT_ROM_NAME;
  }
  const trimmed = name.trim();
  return trimmed.length > 0 ? trimmed : DEFAULT_ROM_NAME;
}

function normalizeRomSource(source) {
  if (typeof source !== "string") {
    return DEFAULT_ROM_SOURCE;
  }
  const trimmed = source.trim();
  return trimmed.length > 0 ? trimmed : DEFAULT_ROM_SOURCE;
}

function normalizeSavedAtMs(savedAtMs) {
  return Number.isFinite(savedAtMs) ? Math.max(0, Math.trunc(savedAtMs)) : Date.now();
}

function openRomStorageDatabase() {
  return new Promise((resolve, reject) => {
    if (!supportsRomStorage()) {
      reject(new Error("IndexedDB unavailable"));
      return;
    }

    const request = indexedDB.open(DB_NAME, DB_VERSION);
    request.onupgradeneeded = () => {
      const database = request.result;
      if (!database.objectStoreNames.contains(STORE_NAME)) {
        database.createObjectStore(STORE_NAME, { keyPath: "key" });
      }
    };
    request.onsuccess = () => resolve(request.result);
    request.onerror = () => reject(request.error ?? new Error("failed to open rom storage"));
    request.onblocked = () => reject(new Error("rom storage open blocked"));
  });
}

function waitForRequestResult(request) {
  return new Promise((resolve, reject) => {
    request.onsuccess = () => resolve(request.result);
    request.onerror = () => reject(request.error ?? new Error("IndexedDB request failed"));
  });
}

function waitForTransaction(transaction) {
  return new Promise((resolve, reject) => {
    transaction.oncomplete = () => resolve();
    transaction.onerror = () => reject(transaction.error ?? new Error("IndexedDB transaction failed"));
    transaction.onabort = () => reject(transaction.error ?? new Error("IndexedDB transaction aborted"));
  });
}
