// IndexedDB 存储桥接，暴露 window.__storage 供 Rust WASM 调用

const DB_NAME = "3xlive";
const DB_VERSION = 1;
const STORES = ["save_data", "pending_uploads", "replay_archive"];

let db: IDBDatabase | null = null;

function getDb(): IDBDatabase {
  if (!db) throw new Error("[storage] DB not initialized");
  return db;
}

function openDb(): Promise<void> {
  return new Promise((resolve, reject) => {
    const req = indexedDB.open(DB_NAME, DB_VERSION);
    req.onupgradeneeded = () => {
      const d = req.result;
      for (const store of STORES) {
        if (!d.objectStoreNames.contains(store)) {
          d.createObjectStore(store);
        }
      }
    };
    req.onsuccess = () => {
      db = req.result;
      resolve();
    };
    req.onerror = () => reject(req.error);
  });
}

function get(store: string, key: string): Promise<Uint8Array | null> {
  return new Promise((resolve, reject) => {
    const tx = getDb().transaction(store, "readonly");
    const req = tx.objectStore(store).get(key);
    req.onsuccess = () => resolve(req.result ?? null);
    req.onerror = () => reject(req.error);
  });
}

function put(store: string, key: string, value: Uint8Array): Promise<void> {
  return new Promise((resolve, reject) => {
    const tx = getDb().transaction(store, "readwrite");
    const req = tx.objectStore(store).put(value, key);
    req.onsuccess = () => resolve();
    req.onerror = () => reject(req.error);
  });
}

function del(store: string, key: string): Promise<void> {
  return new Promise((resolve, reject) => {
    const tx = getDb().transaction(store, "readwrite");
    const req = tx.objectStore(store).delete(key);
    req.onsuccess = () => resolve();
    req.onerror = () => reject(req.error);
  });
}

function getAll(store: string): Promise<Uint8Array[]> {
  return new Promise((resolve, reject) => {
    const tx = getDb().transaction(store, "readonly");
    const req = tx.objectStore(store).getAll();
    req.onsuccess = () => resolve(req.result ?? []);
    req.onerror = () => reject(req.error);
  });
}

function getAllKeys(store: string): Promise<string[]> {
  return new Promise((resolve, reject) => {
    const tx = getDb().transaction(store, "readonly");
    const req = tx.objectStore(store).getAllKeys();
    req.onsuccess = () => resolve((req.result ?? []).map(String));
    req.onerror = () => reject(req.error);
  });
}

function clear(store: string): Promise<void> {
  return new Promise((resolve, reject) => {
    const tx = getDb().transaction(store, "readwrite");
    const req = tx.objectStore(store).clear();
    req.onsuccess = () => resolve();
    req.onerror = () => reject(req.error);
  });
}

// 初始化 DB 并挂载到 window.__storage
await openDb();

declare global {
  interface Window {
    __storage: typeof storageApi;
  }
}

const storageApi = { get, put, del, getAll, getAllKeys, clear };
window.__storage = storageApi;

console.log("[storage] IndexedDB bridge initialized");
