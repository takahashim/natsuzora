/**
 * Platform Compatibility Layer
 *
 * Provides unified APIs for Deno, Node.js, and Bun.
 */

// Declare global types for runtime detection
declare const Deno: {
  readTextFileSync(path: string): string;
} | undefined;

/**
 * Detect the current runtime.
 */
export type Runtime = "deno" | "node" | "bun";

export function detectRuntime(): Runtime {
  if (typeof Deno !== "undefined") {
    return "deno";
  }
  // @ts-ignore - Bun detection
  if (typeof Bun !== "undefined") {
    return "bun";
  }
  return "node";
}

// Lazy-loaded modules for Node.js/Bun
let _fs: typeof import("node:fs") | undefined;
let _path: typeof import("node:path") | undefined;

function getFs(): typeof import("node:fs") {
  if (_fs === undefined) {
    // eslint-disable-next-line @typescript-eslint/no-require-imports
    _fs = require("node:fs") as typeof import("node:fs");
  }
  return _fs;
}

function getPath(): typeof import("node:path") {
  if (_path === undefined) {
    // eslint-disable-next-line @typescript-eslint/no-require-imports
    _path = require("node:path") as typeof import("node:path");
  }
  return _path;
}

/**
 * Read a file synchronously as UTF-8 text.
 */
export function readFileSync(path: string): string {
  if (typeof Deno !== "undefined") {
    return Deno.readTextFileSync(path);
  }
  return getFs().readFileSync(path, "utf-8");
}

/**
 * Check if a file exists.
 */
export function fileExists(path: string): boolean {
  try {
    if (typeof Deno !== "undefined") {
      Deno.readTextFileSync(path);
      return true;
    }
    getFs().accessSync(path);
    return true;
  } catch {
    return false;
  }
}

/**
 * Join path segments.
 */
export function joinPath(...segments: string[]): string {
  if (typeof Deno !== "undefined") {
    // Simple join for Deno (POSIX-style)
    return segments.join("/").replace(/\/+/g, "/");
  }
  return getPath().join(...segments);
}

/**
 * Check if a path is absolute.
 */
export function isAbsolutePath(p: string): boolean {
  if (typeof Deno !== "undefined") {
    return p.startsWith("/");
  }
  return getPath().isAbsolute(p);
}

/**
 * Normalize a path (resolve . and .. segments).
 */
export function normalizePath(p: string): string {
  const isAbsolute = p.startsWith("/");
  const parts = p.split("/").filter((s) => s !== "" && s !== ".");
  const result: string[] = [];

  for (const part of parts) {
    if (part === "..") {
      result.pop();
    } else {
      result.push(part);
    }
  }

  const normalized = result.join("/");
  return isAbsolute ? "/" + normalized : normalized;
}
