# frozen_string_literal: true

require_relative 'compiled_lexer'
require_relative 'parse_error'
require_relative 'diff_marker'
require_relative 'scalar_type'
require_relative 'modifier'
require_relative 'ast/scalar'
require_relative 'ast/record'
require_relative 'ast/list'
require_relative 'ast/ref'
require_relative 'field'
require_relative 'type_def'
require_relative 'document'

module Natsuzora
  module Contract
    # Parser for contract notation.
    # Uses LexerKit stream directly without creating Token objects.
    class Parser
      def initialize(input)
        @stream = CompiledLexer.instance.stream(input)
        # Position of the most recently consumed token; used for EOF error messages
        # and for errors that point back at the just-read identifier (e.g. unknown scalar type).
        @last_line = 1
        @last_col = 1
      end

      # Parse the entire contract file.
      def parse_file
        types = {}

        skip_separators

        while type_definition?
          type_name, contract = parse_type_def
          types[type_name] = TypeDef.new(contract)
          skip_separators
        end

        root = parse_object_body

        Document.new(
          types: types,
          fields: build_fields_from_object(root)
        )
      end

      # Parse with diff markers (2-generation support).
      def parse_file_with_diff
        types = {}
        fields = {}

        skip_separators

        until eof?
          marker = try_parse_diff_marker

          if type_definition?
            type_name, type_def = parse_type_def_with_diff(marker)
            types[type_name] = type_def
          elsif current_type == :IDENTIFIER
            name, field = parse_field_with_diff(marker)
            fields[name] = field
          elsif marker
            raise_current_parse_error("expected field name or 'type' after diff marker")
          else
            break
          end

          skip_separators
        end

        Document.new(types: types, fields: fields)
      end

      private

      def current_type
        @stream.token_name
      end

      def current_text
        @stream.text
      end

      def peek_type
        @stream.peek_token_name
      end

      def eof?
        @stream.eof?
      end

      def advance
        @last_line, @last_col = @stream.line_col unless eof?
        text = @stream.text
        @stream.advance
        text
      end

      def check_error!
        return unless @stream.error?

        line, col = @stream.line_col
        raise ParseError.new("unexpected character '#{@stream.text}'", line, col)
      end

      def skip_separators
        while !eof? && (current_type == :NEWLINE || current_type == :COMMENT)
          check_error!
          advance
        end
        check_error! unless eof?
      end

      def expect_token(type, message)
        return if !eof? && current_type == type

        raise_at_current_or_last(message)
      end

      def expect_identifier
        check_error!
        raise_at_current_or_last('expected field name (lowercase identifier)') if eof? || current_type != :IDENTIFIER

        advance
      end

      def expect_type_name
        check_error!
        raise_at_current_or_last('expected type name (uppercase identifier)') if eof? || current_type != :TYPE_NAME

        advance
      end

      def try_parse_diff_marker
        return nil if eof?

        check_error!
        case current_type
        when :PLUS
          advance
          DiffMarker::ADDED
        when :MINUS
          advance
          DiffMarker::REMOVED
        when :STAR
          advance
          DiffMarker::CHANGED
        end
      end

      def type_definition?
        return false if eof?
        return false unless current_type == :IDENTIFIER && current_text == 'type'

        peek_type == :TYPE_NAME
      end

      def parse_type_def
        advance # consume 'type'
        type_name = expect_type_name
        expect_token(:OPEN_BRACE, "expected '{' after type name")
        [type_name, parse_braced_object_body]
      end

      def parse_type_def_with_diff(marker)
        raise_current_parse_error('* marker is not allowed for type definitions') if marker == DiffMarker::CHANGED

        type_name, contract = parse_type_def
        [type_name, TypeDef.new(contract, marker: marker)]
      end

      def parse_object_body
        properties = {}
        required = []

        skip_separators

        while !eof? && current_type != :CLOSE_BRACE
          skip_separators
          break if eof? || current_type == :CLOSE_BRACE

          name = expect_identifier
          required << name

          raise_at_current_or_last("expected ':' or '{'") if eof?
          check_error!
          contract = case current_type
                     when :COLON
                       advance
                       parse_type
                     when :OPEN_BRACE
                       parse_braced_object_body
                     else
                       raise_current_parse_error("expected ':' or '{'")
                     end

          properties[name] = contract
          skip_separators
        end

        AST::Record.new(properties, required)
      end

      def parse_field_with_diff(marker)
        name = expect_identifier
        current_type_contract, next_type = parse_field_contract_with_diff(marker)
        field = Field.new(current_type_contract, marker: marker, next_type: next_type)
        [name, field]
      end

      def parse_field_contract_with_diff(marker)
        raise_at_current_or_last("expected ':' or '{'") if eof?

        check_error!
        case current_type
        when :COLON
          parse_typed_field_with_diff(marker)
        when :OPEN_BRACE
          parse_nested_object_field_with_diff(marker)
        else
          raise_current_parse_error("expected ':' or '{'")
        end
      end

      def parse_typed_field_with_diff(marker)
        advance
        first_type = parse_type

        if !eof? && current_type == :ARROW
          raise_current_parse_error("'->' is only allowed with * marker") unless marker == DiffMarker::CHANGED

          advance
          return [first_type, parse_type]
        end

        raise_at_current_or_last("* marker requires '->' for type change") if marker == DiffMarker::CHANGED

        [first_type, nil]
      end

      def parse_nested_object_field_with_diff(marker)
        raise_current_parse_error('* marker is not allowed for nested objects') if marker == DiffMarker::CHANGED

        [parse_braced_object_body, nil]
      end

      def parse_braced_object_body
        advance # consume '{'
        inner = parse_object_body
        expect_token(:CLOSE_BRACE, "expected '}'")
        advance
        inner
      end

      def raise_current_parse_error(message)
        line, col = @stream.line_col
        raise ParseError.new(message, line, col)
      end

      def raise_at_current_or_last(message)
        line, col = eof? ? [@last_line, @last_col] : @stream.line_col
        raise ParseError.new(message, line, col)
      end

      def parse_type
        return nil if eof?

        check_error!
        # Check for array type: []type or []{...}
        if current_type == :OPEN_BRACKET
          advance
          expect_token(:CLOSE_BRACKET, "expected ']'")
          advance

          raise_at_current_or_last("expected type after '[]'") if eof?

          items = case current_type
                  when :OPEN_BRACE
                    parse_braced_object_body
                  when :TYPE_NAME
                    parse_type_reference
                  else
                    parse_scalar_type
                  end

          return AST::List.new(items)
        end

        # Check for type reference (uppercase)
        return parse_type_reference if current_type == :TYPE_NAME

        parse_scalar_type
      end

      def parse_type_reference
        type_name = expect_type_name
        AST::Ref.new(type_name)
      end

      def parse_scalar_type
        type_name = expect_identifier

        scalar_type = case type_name
                      when 'string' then ScalarType::STRING
                      when 'integer' then ScalarType::INTEGER
                      when 'bool' then ScalarType::BOOL
                      when 'scalar' then ScalarType::SCALAR
                      else
                        raise ParseError.new(
                          "unknown type '#{type_name}' (did you mean to use uppercase for a type reference?)",
                          @last_line, @last_col
                        )
                      end

        AST::Scalar.new(scalar_type, parse_modifier)
      end

      def parse_modifier
        return Modifier::NONE if eof?

        check_error!
        case current_type
        when :QUESTION
          advance
          Modifier::NULLABLE
        when :EXCLAMATION
          advance
          Modifier::REQUIRED
        else
          Modifier::NONE
        end
      end

      def build_fields_from_object(object_contract)
        object_contract.properties.transform_values do |contract|
          Field.new(contract)
        end
      end
    end

    # Module-level parse functions.
    module_function

    # Parse contract notation into a resolved Contract.
    def parse(input)
      parser = Parser.new(input)
      file = parser.parse_file
      file.to_contract
    end

    # Parse contract notation into a Document.
    def parse_file(input)
      parser = Parser.new(input)
      parser.parse_file
    end

    # Parse contract notation with diff markers.
    def parse_file_with_diff(input)
      parser = Parser.new(input)
      parser.parse_file_with_diff
    end
  end
end
