# frozen_string_literal: true

module Natsuzora
  class Lexer
    # Processes tokens to handle whitespace control and comments
    #
    # Responsibilities:
    # - Consume DASH tokens and strip adjacent TEXT whitespace
    # - Consume comment tokens (PERCENT to CLOSE)
    # - Detect unclosed comments
    class TokenProcessor
      def initialize(tokens)
        @tokens = tokens
        @result = []
        @strip_next_text = false
        @in_comment = false
        @comment_start_token = nil
        @in_tag = false
        @tag_token_count = 0
      end

      def process
        @tokens.each_with_index do |token, index|
          next_token = @tokens[index + 1]

          if @in_comment
            handle_comment_content(token, next_token: next_token)
            next
          end

          case token.type
          when :PERCENT
            start_tag_if_needed
            start_comment(token)
            @tag_token_count += 1
          when :DASH
            start_tag_if_needed
            handle_dash(next_token: next_token)
            @tag_token_count += 1
          when :CLOSE
            handle_close(token)
            @in_tag = false
            @tag_token_count = 0
          when :TEXT
            handle_text(token)
            @in_tag = false
            @tag_token_count = 0
          else
            start_tag_if_needed
            @result << token
            @tag_token_count += 1
          end
        end

        check_unclosed_comment
        @result
      end

      private

      def start_tag_if_needed
        return if @in_tag

        @in_tag = true
        @tag_token_count = 0
      end

      def handle_dash(next_token:)
        strip_trailing_from_last_text_if_blank_line if left_trim_dash?
        @strip_next_text = true if right_trim_dash?(next_token)
      end

      def left_trim_dash?
        @tag_token_count.zero?
      end

      def right_trim_dash?(next_token)
        next_token&.type == :CLOSE
      end

      def handle_close(token)
        @result << token
      end

      def handle_text(token)
        text_value = token.value

        if @strip_next_text
          @strip_next_text = false
          text_value = strip_leading_whitespace_if_blank_line(text_value)
        end

        return if text_value.empty?

        @result << Token.new(:TEXT, text_value, line: token.line, column: token.column)
      end

      def strip_trailing_from_last_text_if_blank_line
        return if @result.empty?

        last_idx = @result.rindex { |t| t.type == :TEXT }
        return unless last_idx

        last_text = @result[last_idx]
        value = last_text.value
        line_start = [value.rindex("\n"), value.rindex("\r")].compact.max
        line_start = line_start ? line_start + 1 : 0
        trailing_segment = value[line_start..] || ''
        return unless trailing_segment.match?(/\A[ \t]*\z/)

        stripped = value[0...line_start]
        @result[last_idx] = Token.new(:TEXT, stripped, line: last_text.line, column: last_text.column)
      end

      def strip_leading_whitespace_if_blank_line(text)
        idx = 0
        bytes = text.bytes

        idx += 1 while idx < bytes.length && (bytes[idx] == 0x20 || bytes[idx] == 0x09)
        return '' if idx >= bytes.length

        case bytes[idx]
        when 0x0A # \n
          text[(idx + 1)..] || ''
        when 0x0D # \r
          advance = (bytes[idx + 1] == 0x0A ? 2 : 1)
          text[(idx + advance)..] || ''
        else
          text
        end
      end

      def start_comment(token)
        @in_comment = true
        @comment_start_token = token
      end

      def handle_comment_content(token, next_token:)
        @strip_next_text = true if token.type == :DASH && right_trim_dash?(next_token)

        case token.type
        when :CLOSE
          @in_comment = false
          @comment_start_token = nil
          @in_tag = false
          @tag_token_count = 0
        end
      end

      def check_unclosed_comment
        return unless @in_comment

        raise LexerError.new(
          'Unclosed comment',
          line: @comment_start_token.line,
          column: @comment_start_token.column
        )
      end
    end
  end
end
