# frozen_string_literal: true

require 'lexer_kit'

LexerKit.build do
  delimited :TEXT, delimiter: '{[', escape: '{[{]}' do
    # Comment body is raw-scanned to the first `]}` (spec 4.5.4): the
    # tempered pattern cannot consume a `]}`, so it always terminates at
    # the first occurrence. COMMENT_UNCLOSED catches the no-`]}` case
    # (longest match guarantees COMMENT_BODY wins when a `]}` exists).
    token :COMMENT_BODY, /%([^\]]|\]+[^}\]])*\]*\]\}/, pop: true
    token :COMMENT_UNCLOSED, /%([^\]]|\]+[^}\]])*\]*/
    token :DASH, '-'
    token :CLOSE, ']}', pop: true
    token :HASH, '#'
    token :SLASH, '/'

    token :BANG_UNSECURE, '!unsecure'
    token :BANG_INCLUDE, '!include'
    token :EXCLAMATION, '!'

    token :KW_IF, 'if'
    token :KW_UNLESS, 'unless'
    token :KW_ELSE, 'else'
    token :KW_EACH, 'each'
    token :KW_AS, 'as'

    token :DOT, '.'
    token :COMMA, ','
    token :EQUAL, '='
    token :QUESTION, '?'

    token :WHITESPACE, /[ \t\r\n]+/
    token :IDENT, /[A-Za-z][A-Za-z0-9_]*/
  end
end
