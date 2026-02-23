# frozen_string_literal: true

module Natsuzora
  class Lexer
    # Processes tokens to handle whitespace control and comments.
    #
    # Responsibilities:
    # - Consume DASH tokens and apply trim rules
    # - Consume comment tags entirely
    # - Detect unclosed comments
    class TokenProcessor
      def initialize(tokens)
        @tokens = tokens
        @result = []
        @strip_next_text = false
      end

      def process
        idx = 0

        while idx < @tokens.length
          token = @tokens[idx]

          if token.type == :TEXT
            append_text(token)
            idx += 1
          else
            idx = process_tag(idx)
          end
        end

        @result
      end

      private

      def process_tag(start_idx)
        close_idx = find_close_index(start_idx)
        tag_tokens = close_idx ? @tokens[start_idx..close_idx] : @tokens[start_idx..]

        apply_left_trim(tag_tokens)
        apply_right_trim(tag_tokens)

        if comment_tag?(tag_tokens)
          raise_unclosed_comment!(tag_tokens) unless close_idx
          return close_idx + 1
        end

        emit_tag_tokens(tag_tokens)
        close_idx ? close_idx + 1 : @tokens.length
      end

      def append_text(token)
        text_value = token.value

        if @strip_next_text
          @strip_next_text = false
          text_value = strip_leading_whitespace_if_blank_line(text_value)
        end

        return if text_value.empty?

        @result << Token.new(:TEXT, text_value, line: token.line, column: token.column)
      end

      def find_close_index(start_idx)
        idx = start_idx
        while idx < @tokens.length
          return idx if @tokens[idx].type == :CLOSE

          idx += 1
        end
        nil
      end

      def apply_left_trim(tag_tokens)
        strip_trailing_from_last_text_if_blank_line if left_trim?(tag_tokens)
      end

      def apply_right_trim(tag_tokens)
        @strip_next_text = true if right_trim?(tag_tokens)
      end

      def left_trim?(tag_tokens)
        tag_begins_with_dash?(tag_tokens)
      end

      def right_trim?(tag_tokens)
        close_idx = tag_close_index(tag_tokens)
        close_idx&.positive? && tag_tokens[close_idx - 1].type == :DASH
      end

      def tag_begins_with_dash?(tag_tokens)
        tag_tokens.first&.type == :DASH
      end

      def tag_close_index(tag_tokens)
        tag_tokens.index { |token| token.type == :CLOSE }
      end

      def comment_tag?(tag_tokens)
        first = tag_tokens.first
        return false unless first

        return true if first.type == :PERCENT

        first.type == :DASH && tag_tokens[1]&.type == :PERCENT
      end

      def emit_tag_tokens(tag_tokens)
        tag_tokens.each do |token|
          next if token.type == :DASH

          @result << token
        end
      end

      def strip_trailing_from_last_text_if_blank_line
        last_idx = @result.rindex { |token| token.type == :TEXT }
        return unless last_idx

        last_text = @result[last_idx]
        value = last_text.value
        line_start = same_line_start_offset(value)
        trailing_segment = value[line_start..] || ''
        return unless horizontal_whitespace_only?(trailing_segment)

        stripped = value[0...line_start]
        @result[last_idx] = Token.new(:TEXT, stripped, line: last_text.line, column: last_text.column)
      end

      def strip_leading_whitespace_if_blank_line(text)
        bytes = text.bytes
        idx = skip_leading_horizontal_whitespace(bytes)
        return '' if idx >= bytes.length

        newline_advance = leading_newline_advance(bytes, idx)
        return text unless newline_advance

        text[(idx + newline_advance)..] || ''
      end

      def same_line_start_offset(value)
        line_break_idx = [value.rindex("\n"), value.rindex("\r")].compact.max
        line_break_idx ? line_break_idx + 1 : 0
      end

      def horizontal_whitespace_only?(segment)
        segment.match?(/\A[ \t]*\z/)
      end

      def skip_leading_horizontal_whitespace(bytes)
        idx = 0
        idx += 1 while idx < bytes.length && (bytes[idx] == 0x20 || bytes[idx] == 0x09)
        idx
      end

      def leading_newline_advance(bytes, idx)
        return 1 if bytes[idx] == 0x0A # \n

        return nil unless bytes[idx] == 0x0D # \r

        bytes[idx + 1] == 0x0A ? 2 : 1
      end

      def raise_unclosed_comment!(tag_tokens)
        comment_token = tag_tokens.find { |token| token.type == :PERCENT } || tag_tokens.first
        raise LexerError.new('Unclosed comment', line: comment_token.line, column: comment_token.column)
      end
    end
  end
end
