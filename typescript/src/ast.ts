/**
 * Natsuzora AST Node Types
 */

export interface Path {
  segments: string[];
}

export function createPath(segments: string[]): Path {
  return { segments };
}

export function pathToString(path: Path): string {
  return path.segments.join(".");
}

// Node types
export interface TextNode {
  type: "text";
  value: string;
}

export type VariableModifier = "nullable" | "required" | null;

export interface VariableNode {
  type: "variable";
  path: Path;
  modifier: VariableModifier;
}

export interface IfBlockNode {
  type: "if_block";
  condition: Path;
  thenBranch: Node[];
  elseBranch: Node[] | null;
}

export interface UnlessBlockNode {
  type: "unless_block";
  condition: Path;
  body: Node[];
}

export interface EachBlockNode {
  type: "each_block";
  collection: Path;
  itemName: string;
  body: Node[];
}

export interface UnsecureOutputNode {
  type: "unsecure_output";
  path: Path;
}

export interface IncludeArg {
  key: string;
  value: Path;
}

export interface IncludeNode {
  type: "include";
  name: string;
  args: IncludeArg[];
}

export type Node =
  | TextNode
  | VariableNode
  | IfBlockNode
  | UnlessBlockNode
  | EachBlockNode
  | UnsecureOutputNode
  | IncludeNode;

export interface Template {
  nodes: Node[];
}

// Node constructors
export function textNode(value: string): TextNode {
  return { type: "text", value };
}

export function variableNode(path: Path, modifier: VariableModifier = null): VariableNode {
  return { type: "variable", path, modifier };
}

export function ifBlockNode(
  condition: Path,
  thenBranch: Node[],
  elseBranch: Node[] | null
): IfBlockNode {
  return { type: "if_block", condition, thenBranch, elseBranch };
}

export function unlessBlockNode(condition: Path, body: Node[]): UnlessBlockNode {
  return { type: "unless_block", condition, body };
}

export function eachBlockNode(
  collection: Path,
  itemName: string,
  body: Node[]
): EachBlockNode {
  return { type: "each_block", collection, itemName, body };
}

export function unsecureOutputNode(path: Path): UnsecureOutputNode {
  return { type: "unsecure_output", path };
}

export function includeNode(name: string, args: IncludeArg[]): IncludeNode {
  return { type: "include", name, args };
}

export function template(nodes: Node[]): Template {
  return { nodes };
}
