/**
 * Natsuzora Error Types
 */

export class NatsuzoraError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "NatsuzoraError";
  }
}

export class LexerError extends NatsuzoraError {
  line: number;
  column: number;

  constructor(message: string, line: number, column: number) {
    super(`${message} at line ${line}, column ${column}`);
    this.name = "LexerError";
    this.line = line;
    this.column = column;
  }
}

export class ParseError extends NatsuzoraError {
  line: number;
  column: number;

  constructor(message: string, line: number, column: number) {
    super(`${message} at line ${line}, column ${column}`);
    this.name = "ParseError";
    this.line = line;
    this.column = column;
  }
}

export class ReservedWordError extends ParseError {
  constructor(word: string, line: number, column: number) {
    super(`'${word}' is a reserved word and cannot be used as an identifier`, line, column);
    this.name = "ReservedWordError";
  }
}

export class RenderError extends NatsuzoraError {
  constructor(message: string) {
    super(message);
    this.name = "RenderError";
  }
}

export class UndefinedVariableError extends RenderError {
  variableName: string;

  constructor(name: string) {
    super(`Undefined variable: ${name}`);
    this.name = "UndefinedVariableError";
    this.variableName = name;
  }
}

export class TypeError extends RenderError {
  constructor(message: string) {
    super(message);
    this.name = "TypeError";
  }
}

export class IncludeError extends RenderError {
  constructor(message: string) {
    super(message);
    this.name = "IncludeError";
  }
}

export class NullValueError extends RenderError {
  constructor(message: string = "Cannot stringify null value") {
    super(message);
    this.name = "NullValueError";
  }
}

export class EmptyStringError extends RenderError {
  constructor(message: string = "Cannot stringify empty string") {
    super(message);
    this.name = "EmptyStringError";
  }
}

export class ShadowingError extends RenderError {
  variableName: string;

  constructor(name: string) {
    super(`Cannot shadow existing variable: ${name}`);
    this.name = "ShadowingError";
    this.variableName = name;
  }
}
