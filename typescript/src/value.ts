/**
 * Natsuzora Value Utilities
 *
 * Truthiness evaluation and stringification.
 */

import { TypeError, NullValueError, EmptyStringError } from "./errors.ts";

export const INTEGER_MIN = -9007199254740991;
export const INTEGER_MAX = 9007199254740991;

/**
 * Check if a value is truthy according to Natsuzora rules.
 *
 * Falsy values:
 * - false
 * - null
 * - 0
 * - "" (empty string)
 * - [] (empty array)
 * - {} (empty object)
 */
export function isTruthy(value: unknown): boolean {
  if (value === false || value === null || value === undefined) {
    return false;
  }

  if (typeof value === "number") {
    return value !== 0;
  }

  if (typeof value === "string") {
    return value.length > 0;
  }

  if (Array.isArray(value)) {
    return value.length > 0;
  }

  if (typeof value === "object") {
    return Object.keys(value).length > 0;
  }

  return true;
}

/**
 * Convert a value to a string according to Natsuzora rules.
 *
 * - String: returned as-is
 * - Integer: converted to string
 * - null/undefined: NullValueError (use stringifyNullable for null -> "")
 * - Boolean: ERROR
 * - Array/Object: ERROR
 */
export function stringify(value: unknown): string {
  if (typeof value === "string") {
    return value;
  }

  if (typeof value === "number") {
    if (!Number.isInteger(value)) {
      throw new TypeError(`Cannot stringify non-integer number: ${value}`);
    }
    if (value < INTEGER_MIN || value > INTEGER_MAX) {
      throw new TypeError(`Integer out of range: ${value}`);
    }
    return String(value);
  }

  if (value === null || value === undefined) {
    throw new NullValueError("Cannot stringify null value without '?' modifier");
  }

  if (typeof value === "boolean") {
    throw new TypeError("Cannot stringify boolean value");
  }

  if (Array.isArray(value)) {
    throw new TypeError("Cannot stringify array");
  }

  if (typeof value === "object") {
    throw new TypeError("Cannot stringify object");
  }

  throw new TypeError(`Cannot stringify value of type ${typeof value}`);
}

/**
 * Stringify with nullable modifier (null -> empty string).
 */
export function stringifyNullable(value: unknown): string {
  if (value === null || value === undefined) {
    return "";
  }
  return stringify(value);
}

/**
 * Stringify with required modifier (null -> NullValueError, "" -> EmptyStringError).
 */
export function stringifyRequired(value: unknown): string {
  if (value === null || value === undefined) {
    throw new NullValueError("Cannot stringify null value with '!' modifier");
  }
  if (value === "") {
    throw new EmptyStringError("Cannot stringify empty string with '!' modifier");
  }
  return stringify(value);
}

/**
 * Ensure a value is an array.
 */
export function ensureArray(value: unknown): unknown[] {
  if (!Array.isArray(value)) {
    throw new TypeError(`Expected array, got ${typeof value}`);
  }
  return value;
}

/**
 * Normalize data for use in templates.
 * - Convert float whole numbers to integers
 * - Convert object keys to strings
 * - Recursively normalize nested structures
 */
export function normalizeData(data: unknown): unknown {
  if (data === null || data === undefined) {
    return data;
  }

  if (typeof data === "number") {
    return normalizeNumber(data);
  }

  if (Array.isArray(data)) {
    return data.map((item) => normalizeData(item));
  }

  if (typeof data === "object") {
    const result: Record<string, unknown> = {};
    for (const [key, value] of Object.entries(data)) {
      result[key] = normalizeData(value);
    }
    return result;
  }

  return data;
}

/**
 * Type guard to check if a value is a Record<string, unknown>.
 */
export function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

/**
 * Normalize data and assert it's a record.
 * Throws if the input is not an object.
 */
export function normalizeRootData(data: unknown): Record<string, unknown> {
  if (!isRecord(data)) {
    throw new TypeError("Root data must be an object");
  }
  return normalizeData(data) as Record<string, unknown>;
}

/**
 * Normalize bindings data.
 */
export function normalizeBindings(bindings: Record<string, unknown>): Record<string, unknown> {
  return normalizeData(bindings) as Record<string, unknown>;
}

function normalizeNumber(value: number): number {
  // Reject NaN and Infinity
  if (!Number.isFinite(value)) {
    throw new TypeError(`Invalid number: ${value}`);
  }

  // Number.isInteger returns true for whole-number floats like 3.0
  // In JavaScript, 3.0 === 3 and Number.isInteger(3.0) === true
  if (!Number.isInteger(value)) {
    throw new TypeError(`Floating point numbers are not supported: ${value}`);
  }

  // Check safe integer range
  if (value < INTEGER_MIN || value > INTEGER_MAX) {
    throw new TypeError(`Integer out of range: ${value}`);
  }

  return value;
}
