/**
 * Tree-sitter grammar for Natsuzora template language v2.0
 */

module.exports = grammar({
  name: 'natsuzora',

  extras: _ => [],

  conflicts: $ => [
    [$.else_clause],
    [$.include_args],
  ],

  rules: {
    template: $ => repeat($._node),

    _node: $ => choice(
      $.comment,
      $.delimiter_escape,
      $.if_block,
      $.unless_block,
      $.each_block,
      $.unsecure_block,
      $.include,
      $.variable,
      $.text,
    ),

    // Text content: anything that's not {[
    // Uses repeat1 pattern for proper tree-sitter GLR parsing
    // prec(-1) ensures tag_open '{[' is tried before single '{'
    text: $ => prec(-1, repeat1($._text_char)),

    _text_char: _ => choice(
      /[^{\]]+/,      // Characters except { and ]
      '{',            // Single { (GLR prefers {[ as tag_open due to prec)
      ']',            // Single ] (GLR prefers ]} as tag_close due to prec)
    ),

    // Variable: {[ path ]}
    variable: $ => seq(
      $.tag_open,
      optional($._ws),
      $.path,
      optional($._ws),
      $.tag_close,
    ),

    // If block: {{#if expr}} ... {{#else}} ... {{/if}}
    if_block: $ => seq(
      $.if_open,
      repeat($._node),
      optional($.else_clause),
      $.if_close,
    ),

    if_open: $ => seq(
      $.tag_open,
      '#',
      optional($._ws),
      'if',
      $._ws,
      $.path,
      optional($._ws),
      $.tag_close,
    ),

    if_close: $ => seq(
      $.tag_open,
      '/',
      optional($._ws),
      'if',
      optional($._ws),
      $.tag_close,
    ),

    else_clause: $ => seq(
      $.else_open,
      repeat($._node),
    ),

    else_open: $ => seq(
      $.tag_open,
      '#',
      optional($._ws),
      'else',
      optional($._ws),
      $.tag_close,
    ),

    // Unless block: {{#unless expr}} ... {{/unless}}
    unless_block: $ => seq(
      $.unless_open,
      repeat($._node),
      $.unless_close,
    ),

    unless_open: $ => seq(
      $.tag_open,
      '#',
      optional($._ws),
      'unless',
      $._ws,
      $.path,
      optional($._ws),
      $.tag_close,
    ),

    unless_close: $ => seq(
      $.tag_open,
      '/',
      optional($._ws),
      'unless',
      optional($._ws),
      $.tag_close,
    ),

    // Each block: {{#each expr as item, index}} ... {{/each}}
    each_block: $ => seq(
      $.each_open,
      repeat($._node),
      $.each_close,
    ),

    each_open: $ => seq(
      $.tag_open,
      '#',
      optional($._ws),
      'each',
      $._ws,
      $.path,
      $._ws,
      'as',
      $._ws,
      $.identifier,
      optional($.each_index),
      optional($._ws),
      $.tag_close,
    ),

    each_index: $ => seq(
      optional($._ws),
      ',',
      optional($._ws),
      $.identifier,
    ),

    each_close: $ => seq(
      $.tag_open,
      '/',
      optional($._ws),
      'each',
      optional($._ws),
      $.tag_close,
    ),

    // Unsecure block: {{#unsecure}} ... {{/unsecure}}
    unsecure_block: $ => seq(
      $.unsecure_open,
      repeat($._node),
      $.unsecure_close,
    ),

    unsecure_open: $ => seq(
      $.tag_open,
      '#',
      optional($._ws),
      'unsecure',
      optional($._ws),
      $.tag_close,
    ),

    unsecure_close: $ => seq(
      $.tag_open,
      '/',
      optional($._ws),
      'unsecure',
      optional($._ws),
      $.tag_close,
    ),

    // Include: {{> /path/to/partial key=value}}
    include: $ => seq(
      $.tag_open,
      '>',
      optional($._ws),
      $.include_name,
      optional($.include_args),
      optional($._ws),
      $.tag_close,
    ),

    include_args: $ => repeat1(seq(
      $._ws,
      $.include_arg,
    )),

    include_arg: $ => seq(
      $.identifier,
      optional($._ws),
      '=',
      optional($._ws),
      $.path,
    ),

    comment: _ => token(seq(
      '{[',
      optional('-'),
      '!',
      repeat(choice(
        /[^-\]]/,
        seq('-', /[^]]/),
        seq(']', /[^}]/),
        seq('-', ']', /[^}]/),
      )),
      optional('-'),
      ']}',
    )),

    // Delimiter escape: {[{]} outputs literal {[
    delimiter_escape: _ => '{[{]}',

    // Tag open/close with optional whitespace control
    tag_open: _ => token(choice('{[', '{[-')),
    tag_close: _ => token(choice(']}', '-]}')),

    // Path: identifier.identifier.identifier
    path: $ => seq(
      $.identifier,
      repeat(seq('.', $.identifier)),
    ),

    // Identifier
    identifier: _ => /[A-Za-z][A-Za-z0-9_]*/,

    // Include name: /path/to/partial
    include_name: _ => /\/[A-Za-z0-9_]+(\/[A-Za-z0-9_]+)*/,

    // Whitespace (inside tags)
    _ws: _ => /[ \t\r\n]+/,
  },
});
