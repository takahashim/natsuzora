# frozen_string_literal: true

module Natsuzora
  class Lexer
    OPEN = '{['
    CLOSE = ']}'

    # Character class patterns as constants for performance
    IDENT_START_PATTERN = /[A-Za-z]/
    IDENT_CONT_PATTERN = /[A-Za-z0-9_]/
    WHITESPACE_PATTERN = /[ \t\r\n]/

    def initialize(source)
      @source = source
      @pos = 0
      @line = 1
      @column = 1
      @tokens = []
      @inside_tag = false
      @at_tag_start = false
      @after_gt = false
      @strip_after_close = false
    end

    def tokenize
      until eof?
        if @inside_tag
          tokenize_inside_tag
        else
          tokenize_text
        end
      end
      @tokens << Token.new(:eof, nil, line: @line, column: @column)
      @tokens
    end

    private

    def tokenize_text
      start_line = @line
      start_column = @column
      text = +''

      # If previous close had -]}, skip leading whitespace and newline
      if @strip_after_close
        @strip_after_close = false
        skip_leading_whitespace_and_newline
      end

      text << advance until eof? || match?(OPEN)

      @tokens << Token.new(:text, text, line: start_line, column: start_column) unless text.empty?

      return if eof?

      consume_open
    end

    def skip_leading_whitespace_and_newline
      # Check if everything up to the first newline is whitespace
      lookahead = 0
      while @pos + lookahead < @source.length
        char = @source[@pos + lookahead]
        break if char == "\n"
        return unless char == ' ' || char == "\t" || char == "\r"

        lookahead += 1
      end

      # Skip the whitespace
      lookahead.times { advance }
      # Skip the newline if present
      advance if current_char == "\n"
    end

    def consume_open
      start_line = @line
      start_column = @column
      advance # {
      advance # [

      # Check for comment: {[! ... ]}
      if current_char == '!'
        skip_comment(start_line, start_column)
        return
      end

      # Check for delimiter escape: {[{]}
      if current_char == '{'
        advance # {
        unless match?(CLOSE)
          raise LexerError.new("Expected ']}' after '{[{'", line: @line, column: @column)
        end
        advance # ]
        advance # }
        @tokens << Token.new(:text, '{[', line: start_line, column: start_column)
        return
      end

      # Check for whitespace control: {[- ... ]}
      if current_char == '-'
        advance # -
        strip_trailing_whitespace_from_last_text
      end

      @tokens << Token.new(:open, OPEN, line: start_line, column: start_column)
      @inside_tag = true
      @at_tag_start = true
    end

    def strip_trailing_whitespace_from_last_text
      # Find the last TEXT token
      last_text_idx = @tokens.rindex { |t| t.type == :text }
      return unless last_text_idx

      text_token = @tokens[last_text_idx]
      value = text_token.value

      # Find the last newline
      newline_pos = value.rindex("\n")
      if newline_pos
        # Check if everything after the newline is whitespace
        suffix = value[(newline_pos + 1)..]
        return unless suffix.match?(/\A[ \t]*\z/)

        # Strip trailing whitespace (keep the newline)
        new_value = value[0..newline_pos]
        @tokens[last_text_idx] = Token.new(:text, new_value, line: text_token.line, column: text_token.column)
      else
        # No newline - check if entire value is whitespace
        return unless value.match?(/\A[ \t]*\z/)

        # Remove the token entirely
        @tokens.delete_at(last_text_idx)
      end
    end

    def skip_comment(start_line, start_column)
      advance # !

      # Skip until ]}
      until eof? || match?(CLOSE)
        advance
      end

      if eof?
        raise LexerError.new('Unclosed comment', line: start_line, column: start_column)
      end

      advance # }
      advance # }
      # Comment is completely ignored - no token emitted
    end

    def tokenize_inside_tag
      check_no_whitespace_before_special_chars
      skip_whitespace_with_token
      @at_tag_start = false

      return if eof?

      if match?(CLOSE)
        consume_close
        return
      end

      # Check for whitespace control: -]}
      if current_char == '-' && peek_char == ']'
        advance # -
        @strip_after_close = true
        consume_close
        return
      end

      case current_char
      when '#'
        add_single_char_token(:hash)
      when '/'
        if @after_gt
          tokenize_include_name
          @after_gt = false
        else
          add_single_char_token(:slash)
        end
      when '>'
        add_single_char_token(:gt)
        @after_gt = true
      when '='
        add_single_char_token(:equal)
      when ','
        add_single_char_token(:comma)
      when '.'
        add_single_char_token(:dot)
      else
        @after_gt = false
        tokenize_identifier_or_name
      end
    end

    def consume_close
      start_line = @line
      start_column = @column
      advance # }
      advance # }
      @tokens << Token.new(:close, CLOSE, line: start_line, column: start_column)
      @inside_tag = false
    end

    def tokenize_identifier_or_name
      if current_char == '/'
        tokenize_include_name
      elsif ident_start?(current_char)
        tokenize_identifier
      else
        raise LexerError.new("Unexpected character: '#{current_char}'", line: @line, column: @column)
      end
    end

    def tokenize_identifier
      start_line = @line
      start_column = @column
      value = +''

      value << advance while ident_cont?(current_char)

      type = Token::KEYWORDS[value] || :ident
      @tokens << Token.new(type, value, line: start_line, column: start_column)
    end

    def tokenize_include_name
      start_line = @line
      start_column = @column
      value = +''

      # Consume leading /
      value << advance

      loop do
        break unless name_seg_char?(current_char) || (current_char == '/' && name_seg_char?(peek_char))

        value << advance
      end

      @tokens << Token.new(:ident, value, line: start_line, column: start_column)
    end

    def check_no_whitespace_before_special_chars
      # Only check at the start of tag content (right after {[ or {[-)
      return unless @at_tag_start
      return unless whitespace?(current_char)

      # Look ahead to find first non-whitespace character
      lookahead = 0
      lookahead += 1 while whitespace?(@source[@pos + lookahead])

      next_char = @source[@pos + lookahead]
      return unless next_char == '#' || next_char == '/' || next_char == '>'

      raise LexerError.new(
        "Whitespace not allowed before '#{next_char}' after tag open",
        line: @line,
        column: @column
      )
    end

    def skip_whitespace_with_token
      return unless whitespace?(current_char)

      start_line = @line
      start_column = @column
      value = +''

      value << advance while whitespace?(current_char)

      @tokens << Token.new(:whitespace, value, line: start_line, column: start_column)
    end

    def add_single_char_token(type)
      @tokens << Token.new(type, advance, line: @line, column: @column - 1)
    end

    def eof?
      @pos >= @source.length
    end

    def current_char
      return nil if eof?

      @source[@pos]
    end

    def peek_char
      return nil if @pos + 1 >= @source.length

      @source[@pos + 1]
    end

    def advance
      char = current_char
      @pos += 1
      if char == "\n"
        @line += 1
        @column = 1
      else
        @column += 1
      end
      char
    end

    def match?(str)
      @source[@pos, str.length] == str
    end

    def ident_start?(char)
      return false if char.nil?

      IDENT_START_PATTERN.match?(char)
    end

    def ident_cont?(char)
      return false if char.nil?

      IDENT_CONT_PATTERN.match?(char)
    end

    # Alias for ident_cont? since they have identical logic
    alias name_seg_char? ident_cont?

    def whitespace?(char)
      return false if char.nil?

      WHITESPACE_PATTERN.match?(char)
    end
  end
end
