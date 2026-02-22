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
      end

      def process
        @tokens.each do |token|
          if @in_comment
            handle_comment_content(token)
            next
          end

          case token.type
          when :PERCENT
            start_comment(token)
          when :DASH
            handle_dash
          when :CLOSE
            handle_close(token)
          when :TEXT
            handle_text(token)
          else
            @result << token
          end
        end

        check_unclosed_comment
        @result
      end

      private

      def handle_dash
        # Strip trailing whitespace from previous TEXT
        strip_trailing_from_last_text
        # Set flag to strip next TEXT (after CLOSE)
        @strip_next_text = true
      end

      def handle_close(token)
        @result << token
        # @strip_next_text remains set for the next TEXT token
      end

      def handle_text(token)
        text_value = token.value

        if @strip_next_text
          @strip_next_text = false
          text_value = strip_leading_whitespace_and_newline(text_value)
        end

        # Only add non-empty text tokens
        return if text_value.empty?

        @result << Token.new(:TEXT, text_value, line: token.line, column: token.column)
      end

      def strip_trailing_from_last_text
        return if @result.empty?

        last_idx = @result.rindex { |t| t.type == :TEXT }
        return unless last_idx

        last_text = @result[last_idx]
        stripped = last_text.value.sub(/[ \t]*\z/, '')
        @result[last_idx] = Token.new(:TEXT, stripped, line: last_text.line, column: last_text.column)
      end

      def strip_leading_whitespace_and_newline(text)
        text.sub(/\A[ \t]*\n?/, '')
      end

      def start_comment(token)
        @in_comment = true
        @comment_start_token = token
      end

      def handle_comment_content(token)
        case token.type
        when :CLOSE
          @in_comment = false
          @comment_start_token = nil
        end
        # All tokens inside comment are ignored (not added to result)
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
