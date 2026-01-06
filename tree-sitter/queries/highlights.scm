; Natsuzora syntax highlighting queries

; Comments
(comment) @comment

; Delimiter escape (outputs literal {[)
(delimiter_escape) @string.escape

; Tag delimiters
(tag_open) @punctuation.bracket
(tag_close) @punctuation.bracket

; Keywords
(if_open "if" @keyword.conditional)
(if_close "if" @keyword.conditional)
(else_open "else" @keyword.conditional)
(unless_open "unless" @keyword.conditional)
(unless_close "unless" @keyword.conditional)
(each_open "each" @keyword.repeat)
(each_open "as" @keyword)
(each_close "each" @keyword.repeat)
(unsecure_open "unsecure" @keyword)
(unsecure_close "unsecure" @keyword)

; Block markers
(if_open "#" @punctuation.special)
(else_open "#" @punctuation.special)
(unless_open "#" @punctuation.special)
(each_open "#" @punctuation.special)
(unsecure_open "#" @punctuation.special)
(if_close "/" @punctuation.special)
(unless_close "/" @punctuation.special)
(each_close "/" @punctuation.special)
(unsecure_close "/" @punctuation.special)

; Include
(include ">" @punctuation.special)
(include_name) @string.special

; Include arguments
(include_arg
  (identifier) @variable.parameter
  "=" @operator)

; Variables and paths
(variable
  (path
    (identifier) @variable))

(path
  "." @punctuation.delimiter)

; Each loop variable
(each_open
  (identifier) @variable.parameter)
(each_index
  "," @punctuation.delimiter
  (identifier) @variable.parameter)

; Condition expressions
(if_open
  (path (identifier) @variable))
(unless_open
  (path (identifier) @variable))
(each_open
  (path (identifier) @variable))

; Text content
(text) @none
