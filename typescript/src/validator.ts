/**
 * Natsuzora Validator
 *
 * Validates identifiers and include names.
 */

import { ParseError } from "./errors.ts";
import { RESERVED_WORDS } from "./token.ts";

const IDENT_START_REGEX = /^[A-Za-z]$/;
const IDENT_CONT_REGEX = /^[A-Za-z0-9_]$/;

/**
 * Check if a character can start an identifier.
 */
export function isIdentStart(char: string | null): boolean {
  if (char === null) return false;
  return IDENT_START_REGEX.test(char);
}

/**
 * Check if a character can continue an identifier.
 */
export function isIdentCont(char: string | null): boolean {
  if (char === null) return false;
  return IDENT_CONT_REGEX.test(char);
}

/**
 * Validate an identifier.
 * - Must start with a letter
 * - Can contain letters, digits, and underscores
 * - Cannot start with underscore
 * - Cannot be a reserved word
 */
export function validateIdentifier(
  name: string,
  line: number,
  column: number,
  allowReserved = false
): void {
  if (name.length === 0) {
    throw new ParseError("Empty identifier", line, column);
  }

  if (name.startsWith("_")) {
    throw new ParseError(
      `Identifier '${name}' cannot start with underscore`,
      line,
      column
    );
  }

  if (!allowReserved && RESERVED_WORDS.has(name)) {
    throw new ParseError(
      `'${name}' is a reserved word and cannot be used as an identifier`,
      line,
      column
    );
  }
}

/**
 * Validate an include name.
 * - Must start with '/'
 * - Each segment must be a valid identifier
 * - Cannot contain '..' or '//' or '\' or ':'
 */
export function validateIncludeName(
  name: string,
  line: number,
  column: number
): void {
  if (!name.startsWith("/")) {
    throw new ParseError(
      `Include name must start with '/': ${name}`,
      line,
      column
    );
  }

  if (name.includes("//")) {
    throw new ParseError(
      `Include name cannot contain '//': ${name}`,
      line,
      column
    );
  }

  if (name.includes("\\")) {
    throw new ParseError(
      `Include name cannot contain '\\': ${name}`,
      line,
      column
    );
  }

  if (name.includes(":")) {
    throw new ParseError(
      `Include name cannot contain ':': ${name}`,
      line,
      column
    );
  }

  if (name.includes("..")) {
    throw new ParseError(
      `Include name cannot contain '..': ${name}`,
      line,
      column
    );
  }

  // Validate each segment
  const segments = name.slice(1).split("/");
  for (const segment of segments) {
    if (segment.length === 0) {
      throw new ParseError(
        `Include name has empty segment: ${name}`,
        line,
        column
      );
    }

    if (!isIdentStart(segment[0])) {
      throw new ParseError(
        `Include segment must start with a letter: ${segment}`,
        line,
        column
      );
    }

    for (let i = 1; i < segment.length; i++) {
      if (!isIdentCont(segment[i])) {
        throw new ParseError(
          `Invalid character in include segment: ${segment}`,
          line,
          column
        );
      }
    }
  }
}
