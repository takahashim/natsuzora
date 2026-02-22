/**
 * Value utilities unit tests
 */

import {
  test,
  assertEquals,
  assertTrue,
  assertFalse,
  assertThrows,
  runTests,
} from "./test_utils.ts";
import {
  isTruthy,
  stringify,
  stringifyNullable,
  stringifyRequired,
  ensureArray,
  normalizeData,
  normalizeRootData,
  isRecord,
  INTEGER_MIN,
  INTEGER_MAX,
} from "../src/value.ts";
import { TypeError, NullValueError, EmptyStringError } from "../src/errors.ts";

// =============================================================================
// isTruthy
// =============================================================================

test("isTruthy - falsy values", async (t) => {
  await t.step("false is falsy", () => {
    assertFalse(isTruthy(false));
  });

  await t.step("null is falsy", () => {
    assertFalse(isTruthy(null));
  });

  await t.step("undefined is falsy", () => {
    assertFalse(isTruthy(undefined));
  });

  await t.step("0 is falsy", () => {
    assertFalse(isTruthy(0));
  });

  await t.step("-0 is falsy", () => {
    assertFalse(isTruthy(-0));
  });

  await t.step("empty string is falsy", () => {
    assertFalse(isTruthy(""));
  });

  await t.step("empty array is falsy", () => {
    assertFalse(isTruthy([]));
  });

  await t.step("empty object is falsy", () => {
    assertFalse(isTruthy({}));
  });
});

test("isTruthy - truthy values", async (t) => {
  await t.step("true is truthy", () => {
    assertTrue(isTruthy(true));
  });

  await t.step("1 is truthy", () => {
    assertTrue(isTruthy(1));
  });

  await t.step("-1 is truthy", () => {
    assertTrue(isTruthy(-1));
  });

  await t.step("non-empty string is truthy", () => {
    assertTrue(isTruthy("hello"));
  });

  await t.step("string '0' is truthy", () => {
    assertTrue(isTruthy("0"));
  });

  await t.step("string 'false' is truthy", () => {
    assertTrue(isTruthy("false"));
  });

  await t.step("whitespace string is truthy", () => {
    assertTrue(isTruthy(" "));
  });

  await t.step("non-empty array is truthy", () => {
    assertTrue(isTruthy([1, 2, 3]));
  });

  await t.step("array with falsy element is truthy", () => {
    assertTrue(isTruthy([null]));
  });

  await t.step("non-empty object is truthy", () => {
    assertTrue(isTruthy({ a: 1 }));
  });

  await t.step("object with falsy value is truthy", () => {
    assertTrue(isTruthy({ a: null }));
  });
});

// =============================================================================
// stringify
// =============================================================================

test("stringify - valid values", async (t) => {
  await t.step("string returns as-is", () => {
    assertEquals(stringify("hello"), "hello");
  });

  await t.step("empty string returns as-is", () => {
    assertEquals(stringify(""), "");
  });

  await t.step("positive integer", () => {
    assertEquals(stringify(42), "42");
  });

  await t.step("negative integer", () => {
    assertEquals(stringify(-123), "-123");
  });

  await t.step("zero", () => {
    assertEquals(stringify(0), "0");
  });

  await t.step("max safe integer", () => {
    assertEquals(stringify(INTEGER_MAX), String(INTEGER_MAX));
  });

  await t.step("min safe integer", () => {
    assertEquals(stringify(INTEGER_MIN), String(INTEGER_MIN));
  });
});

test("stringify - error cases", async (t) => {
  await t.step("null throws NullValueError", () => {
    assertThrows(() => stringify(null), NullValueError);
  });

  await t.step("undefined throws NullValueError", () => {
    assertThrows(() => stringify(undefined), NullValueError);
  });

  await t.step("boolean true throws TypeError", () => {
    assertThrows(() => stringify(true), TypeError, "boolean");
  });

  await t.step("boolean false throws TypeError", () => {
    assertThrows(() => stringify(false), TypeError, "boolean");
  });

  await t.step("array throws TypeError", () => {
    assertThrows(() => stringify([1, 2, 3]), TypeError, "array");
  });

  await t.step("object throws TypeError", () => {
    assertThrows(() => stringify({ a: 1 }), TypeError, "object");
  });

  await t.step("floating point throws TypeError", () => {
    assertThrows(() => stringify(3.14), TypeError, "non-integer");
  });

  await t.step("integer out of range (positive) throws TypeError", () => {
    assertThrows(() => stringify(INTEGER_MAX + 1), TypeError, "out of range");
  });

  await t.step("integer out of range (negative) throws TypeError", () => {
    assertThrows(() => stringify(INTEGER_MIN - 1), TypeError, "out of range");
  });
});

// =============================================================================
// stringifyNullable
// =============================================================================

test("stringifyNullable", async (t) => {
  await t.step("null returns empty string", () => {
    assertEquals(stringifyNullable(null), "");
  });

  await t.step("undefined returns empty string", () => {
    assertEquals(stringifyNullable(undefined), "");
  });

  await t.step("empty string returns empty string", () => {
    assertEquals(stringifyNullable(""), "");
  });

  await t.step("string returns as-is", () => {
    assertEquals(stringifyNullable("hello"), "hello");
  });

  await t.step("integer returns string", () => {
    assertEquals(stringifyNullable(42), "42");
  });

  await t.step("zero returns '0'", () => {
    assertEquals(stringifyNullable(0), "0");
  });

  await t.step("boolean throws TypeError", () => {
    assertThrows(() => stringifyNullable(true), TypeError);
  });
});

// =============================================================================
// stringifyRequired
// =============================================================================

test("stringifyRequired", async (t) => {
  await t.step("null throws NullValueError", () => {
    assertThrows(() => stringifyRequired(null), NullValueError);
  });

  await t.step("undefined throws NullValueError", () => {
    assertThrows(() => stringifyRequired(undefined), NullValueError);
  });

  await t.step("empty string throws EmptyStringError", () => {
    assertThrows(() => stringifyRequired(""), EmptyStringError);
  });

  await t.step("string returns as-is", () => {
    assertEquals(stringifyRequired("hello"), "hello");
  });

  await t.step("integer returns string", () => {
    assertEquals(stringifyRequired(42), "42");
  });

  await t.step("zero returns '0'", () => {
    assertEquals(stringifyRequired(0), "0");
  });

  await t.step("boolean throws TypeError", () => {
    assertThrows(() => stringifyRequired(true), TypeError);
  });
});

// =============================================================================
// ensureArray
// =============================================================================

test("ensureArray", async (t) => {
  await t.step("returns array as-is", () => {
    const arr = [1, 2, 3];
    assertEquals(ensureArray(arr), arr);
  });

  await t.step("empty array returns as-is", () => {
    assertEquals(ensureArray([]), []);
  });

  await t.step("string throws TypeError", () => {
    assertThrows(() => ensureArray("not array"), TypeError);
  });

  await t.step("number throws TypeError", () => {
    assertThrows(() => ensureArray(42), TypeError);
  });

  await t.step("object throws TypeError", () => {
    assertThrows(() => ensureArray({ a: 1 }), TypeError);
  });

  await t.step("null throws TypeError", () => {
    assertThrows(() => ensureArray(null), TypeError);
  });
});

// =============================================================================
// normalizeData - number handling
// =============================================================================

test("normalizeData - number handling", async (t) => {
  await t.step("integer passes through", () => {
    assertEquals(normalizeData(42), 42);
  });

  await t.step("whole number float (3.0) converts to integer", () => {
    assertEquals(normalizeData(3.0), 3);
  });

  await t.step("negative whole number float (-5.0) converts to integer", () => {
    assertEquals(normalizeData(-5.0), -5);
  });

  await t.step("zero float (0.0) converts to integer", () => {
    assertEquals(normalizeData(0.0), 0);
  });

  await t.step("fractional float (3.14) throws TypeError", () => {
    assertThrows(() => normalizeData(3.14), TypeError, "not supported");
  });

  await t.step("very small decimal (0.0001) throws TypeError", () => {
    assertThrows(() => normalizeData(0.0001), TypeError, "not supported");
  });

  await t.step("NaN throws TypeError", () => {
    assertThrows(() => normalizeData(NaN), TypeError, "Invalid number");
  });

  await t.step("Infinity throws TypeError", () => {
    assertThrows(() => normalizeData(Infinity), TypeError, "Invalid number");
  });

  await t.step("-Infinity throws TypeError", () => {
    assertThrows(() => normalizeData(-Infinity), TypeError, "Invalid number");
  });

  await t.step("integer at max boundary", () => {
    assertEquals(normalizeData(INTEGER_MAX), INTEGER_MAX);
  });

  await t.step("integer at min boundary", () => {
    assertEquals(normalizeData(INTEGER_MIN), INTEGER_MIN);
  });

  await t.step("integer beyond max throws TypeError", () => {
    assertThrows(() => normalizeData(INTEGER_MAX + 1), TypeError, "out of range");
  });

  await t.step("integer beyond min throws TypeError", () => {
    assertThrows(() => normalizeData(INTEGER_MIN - 1), TypeError, "out of range");
  });
});

test("normalizeData - nested structures", async (t) => {
  await t.step("normalizes nested object with float", () => {
    const input = { a: { b: 3.0 } };
    const expected = { a: { b: 3 } };
    assertEquals(normalizeData(input), expected);
  });

  await t.step("normalizes array elements with floats", () => {
    const input = [1.0, 2.0, 3.0];
    const expected = [1, 2, 3];
    assertEquals(normalizeData(input), expected);
  });

  await t.step("normalizes deeply nested structure", () => {
    const input = { arr: [{ val: 42.0 }] };
    const expected = { arr: [{ val: 42 }] };
    assertEquals(normalizeData(input), expected);
  });

  await t.step("throws on nested fractional float", () => {
    assertThrows(() => normalizeData({ a: { b: 3.14 } }), TypeError);
  });

  await t.step("throws on array with fractional float", () => {
    assertThrows(() => normalizeData([1, 2.5, 3]), TypeError);
  });
});

test("normalizeData - preserves other types", async (t) => {
  await t.step("string unchanged", () => {
    assertEquals(normalizeData("hello"), "hello");
  });

  await t.step("boolean unchanged", () => {
    assertEquals(normalizeData(true), true);
    assertEquals(normalizeData(false), false);
  });

  await t.step("null unchanged", () => {
    assertEquals(normalizeData(null), null);
  });

  await t.step("undefined unchanged", () => {
    assertEquals(normalizeData(undefined), undefined);
  });
});

// =============================================================================
// isRecord
// =============================================================================

test("isRecord", async (t) => {
  await t.step("plain object returns true", () => {
    assertTrue(isRecord({}));
    assertTrue(isRecord({ a: 1 }));
  });

  await t.step("null returns false", () => {
    assertFalse(isRecord(null));
  });

  await t.step("array returns false", () => {
    assertFalse(isRecord([]));
    assertFalse(isRecord([1, 2, 3]));
  });

  await t.step("primitives return false", () => {
    assertFalse(isRecord("string"));
    assertFalse(isRecord(42));
    assertFalse(isRecord(true));
    assertFalse(isRecord(undefined));
  });
});

// =============================================================================
// normalizeRootData
// =============================================================================

test("normalizeRootData", async (t) => {
  await t.step("accepts object", () => {
    const result = normalizeRootData({ a: 1 });
    assertEquals(result, { a: 1 });
  });

  await t.step("rejects string", () => {
    assertThrows(() => normalizeRootData("string" as unknown), TypeError, "must be an object");
  });

  await t.step("rejects number", () => {
    assertThrows(() => normalizeRootData(42 as unknown), TypeError, "must be an object");
  });

  await t.step("rejects null", () => {
    assertThrows(() => normalizeRootData(null as unknown), TypeError, "must be an object");
  });

  await t.step("rejects array", () => {
    assertThrows(() => normalizeRootData([] as unknown), TypeError, "must be an object");
  });
});

// Run tests
runTests();
