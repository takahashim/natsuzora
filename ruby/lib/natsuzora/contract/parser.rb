# frozen_string_literal: true

require_relative 'compiled_lexer'
require_relative 'types'

module Natsuzora
  module Contract
    # Parser for contract notation.
    # Uses LexerKit stream directly without creating Token objects.
    class Parser
      def initialize(input)
        @stream = CompiledLexer.instance.stream(input)
        @types = {}
        @last_line = 1
        @last_col = 1
      end

      # Parse the entire contract file.
      def parse_file
        skip_separators

        # Parse type definitions
        while type_definition?
          parse_type_def
          skip_separators
        end

        # Parse root contract
        root = parse_object_body

        ContractFile.new(
          types: @types.transform_values { |c| TypeDef.new(c) },
          fields: build_fields_from_object(root)
        )
      end

      # Parse with diff markers (2-generation support).
      def parse_file_with_diff
        types = {}
        fields = {}

        skip_separators

        until eof?
          # Check for diff marker
          marker = try_parse_diff_marker

          # Check if this is a type definition
          if type_definition?
            # Validate marker for type def (* is not allowed)
            if marker == DiffMarker::CHANGED
              line, col = @stream.line_col
              raise ParseError.new('* marker is not allowed for type definitions', line, col)
            end

            advance # consume 'type'
            type_name = expect_type_name

            expect_token(:OPEN_BRACE, "expected '{' after type name")
            advance

            contract = parse_object_body

            expect_token(:CLOSE_BRACE, "expected '}'")
            advance

            types[type_name] = TypeDef.new(contract, marker: marker)
          elsif current_type == :IDENTIFIER
            name, field = parse_field_with_diff(marker)
            fields[name] = field
          elsif marker
            line, col = @stream.line_col
            raise ParseError.new("expected field name or 'type' after diff marker", line, col)
          else
            break
          end

          skip_separators
        end

        ContractFile.new(types: types, fields: fields)
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

        if eof?
          raise ParseError.new(message, @last_line, @last_col)
        else
          line, col = @stream.line_col
          raise ParseError.new(message, line, col)
        end
      end

      def expect_identifier
        check_error!
        if eof? || current_type != :IDENTIFIER
          if eof?
            raise ParseError.new('expected field name (lowercase identifier)', @last_line, @last_col)
          else
            line, col = @stream.line_col
            raise ParseError.new('expected field name (lowercase identifier)', line, col)
          end
        end

        advance
      end

      def expect_type_name
        check_error!
        if eof? || current_type != :TYPE_NAME
          if eof?
            raise ParseError.new('expected type name (uppercase identifier)', @last_line, @last_col)
          else
            line, col = @stream.line_col
            raise ParseError.new('expected type name (uppercase identifier)', line, col)
          end
        end

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
        # Consume 'type' identifier
        advance

        type_name = expect_type_name

        expect_token(:OPEN_BRACE, "expected '{' after type name")
        advance

        contract = parse_object_body

        expect_token(:CLOSE_BRACE, "expected '}'")
        advance

        @types[type_name] = contract
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

          contract = if eof?
                       raise ParseError.new("expected ':' or '{'", @last_line, @last_col)
                     else
                       check_error!
                       case current_type
                       when :COLON
                         advance
                         parse_type
                       when :OPEN_BRACE
                         advance
                         inner = parse_object_body
                         expect_token(:CLOSE_BRACE, "expected '}'")
                         advance
                         inner
                       else
                         line, col = @stream.line_col
                         raise ParseError.new("expected ':' or '{'", line, col)
                       end
                     end

          properties[name] = contract
          skip_separators
        end

        ObjectContract.new(properties, required)
      end

      def parse_field_with_diff(marker)
        name = expect_identifier

        current_type_contract, next_type = if eof?
                                             raise ParseError.new("expected ':' or '{'", @last_line, @last_col)
                                           else
                                             check_error!
                                             case current_type
                                             when :COLON
                                               advance
                                               first_type = parse_type

                                               if !eof? && current_type == :ARROW
                                                 unless marker == DiffMarker::CHANGED
                                                   line, col = @stream.line_col
                                                   raise ParseError.new("'->' is only allowed with * marker", line, col)
                                                 end

                                                 advance
                                                 second_type = parse_type
                                                 [first_type, second_type]
                                               else
                                                 if marker == DiffMarker::CHANGED
                                                   line = eof? ? @last_line : @stream.line_col[0]
                                                   col = eof? ? @last_col : @stream.line_col[1]
                                                   raise ParseError.new("* marker requires '->' for type change", line, col)
                                                 end

                                                 [first_type, nil]
                                               end
                                             when :OPEN_BRACE
                                               if marker == DiffMarker::CHANGED
                                                 line, col = @stream.line_col
                                                 raise ParseError.new('* marker is not allowed for nested objects', line, col)
                                               end

                                               advance
                                               inner = parse_object_body
                                               expect_token(:CLOSE_BRACE, "expected '}'")
                                               advance
                                               [inner, nil]
                                             else
                                               line, col = @stream.line_col
                                               raise ParseError.new("expected ':' or '{'", line, col)
                                             end
                                           end

        field = ContractField.new(current_type_contract, marker: marker, next_type: next_type)
        [name, field]
      end

      def parse_type
        return nil if eof?

        check_error!
        # Check for array type: []type or []{...}
        if current_type == :OPEN_BRACKET
          advance
          expect_token(:CLOSE_BRACKET, "expected ']'")
          advance

          items = if eof?
                    raise ParseError.new("expected type after '[]'", @last_line, @last_col)
                  elsif current_type == :OPEN_BRACE
                    advance
                    inner = parse_object_body
                    expect_token(:CLOSE_BRACE, "expected '}'")
                    advance
                    inner
                  elsif current_type == :TYPE_NAME
                    parse_type_reference
                  else
                    parse_scalar_type
                  end

          return ArrayContract.new(items)
        end

        # Check for type reference (uppercase)
        return parse_type_reference if current_type == :TYPE_NAME

        parse_scalar_type
      end

      def parse_type_reference
        type_name = expect_type_name
        TypeRefContract.new(type_name)
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

        modifier = if eof?
                     ContractModifier::NONE
                   else
                     check_error!
                     case current_type
                     when :QUESTION
                       advance
                       ContractModifier::NULLABLE
                     when :EXCLAMATION
                       advance
                       ContractModifier::REQUIRED
                     else
                       ContractModifier::NONE
                     end
                   end

        ScalarContract.new(scalar_type, modifier)
      end

      def build_fields_from_object(object_contract)
        object_contract.properties.transform_values do |contract|
          ContractField.new(contract)
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

    # Parse contract notation into a ContractFile.
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
