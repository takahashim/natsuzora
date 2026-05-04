# frozen_string_literal: true

require 'lexer_kit'

LexerKit.build do
  token :ARROW, '->'
  token :COLON, ':'
  token :OPEN_BRACE, '{'
  token :CLOSE_BRACE, '}'
  token :OPEN_BRACKET, '['
  token :CLOSE_BRACKET, ']'
  token :QUESTION, '?'
  token :EXCLAMATION, '!'
  token :PLUS, '+'
  token :MINUS, '-'
  token :STAR, '*'

  token :COMMENT, /#[^\n]*/
  token :TYPE_NAME, /[A-Z][a-zA-Z0-9_]*/
  token :IDENTIFIER, /[a-z][a-zA-Z0-9_]*/
  token :NEWLINE, /\n/

  token :WS, /[ \t\r]+/, skip: true
end
