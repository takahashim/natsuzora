# frozen_string_literal: true

module Natsuzora
  class Parser
    def initialize(tokens)
      @tokens = tokens
      @pos = 0
    end

    def parse
      nodes = parse_nodes
      AST::Template.new(nodes, line: 1, column: 1)
    end

    private

    def parse_nodes(stop_types: [:EOF])
      nodes = []
      nodes << parse_node until stop_types.include?(current_type)
      nodes
    end

    def parse_node
      case current_type
      when :TEXT
        node = parse_text
        parse_tag_content_if_present
        node
      when :HASH, :SLASH, :BANG_UNSECURE, :BANG_INCLUDE, :IDENT, :KW_IF, :KW_UNLESS, :KW_ELSE, :KW_EACH, :KW_AS,
           :WHITESPACE, :QUESTION, :EXCLAMATION, :DOT, :EQUAL, :COMMA
        parse_tag_content
      else
        unexpected_token!
      end
    end

    def parse_text
      token = consume(:TEXT)
      AST::Text.new(token.value, line: token.line, column: token.column)
    end

    def parse_tag_content_if_present
      # No-op: comments are now handled by TokenProcessor
    end

    def parse_tag_content
      first_token = current_token

      check_no_whitespace_before_special(first_token) if current_type == :WHITESPACE

      skip_whitespace

      case current_type
      when :HASH
        parse_block_open
      when :SLASH
        unexpected_token!('Unexpected block close')
      when :BANG_UNSECURE
        parse_unsecure_output
      when :BANG_INCLUDE
        parse_include
      else
        parse_variable_node
      end
    end

    def check_no_whitespace_before_special(ws_token)
      saved_pos = @pos
      skip_whitespace
      if %i[HASH SLASH BANG_UNSECURE BANG_INCLUDE].include?(current_type)
        raise ParseError.new(
          "Whitespace not allowed before '#{current_token.value}' after tag open",
          line: ws_token.line,
          column: ws_token.column
        )
      end
      @pos = saved_pos
    end

    def parse_block_open
      consume(:HASH)
      skip_whitespace

      case current_type
      when :KW_IF
        parse_if_block
      when :KW_UNLESS
        parse_unless_block
      when :KW_EACH
        parse_each_block
      when :KW_ELSE
        unexpected_token!("Unexpected 'else' without 'if'")
      else
        unexpected_token!
      end
    end

    def parse_if_block
      token = consume(:KW_IF)
      line = token.line
      column = token.column

      consume_required_whitespace
      condition = parse_path
      skip_whitespace
      consume(:CLOSE)

      then_nodes = parse_if_body
      else_nodes = nil

      if else_open?
        consume_else
        else_nodes = parse_if_body
      end

      consume_block_close(:KW_IF)

      AST::IfBlock.new(
        condition: condition,
        then_nodes: then_nodes,
        else_nodes: else_nodes,
        line: line,
        column: column
      )
    end

    def parse_if_body
      nodes = []
      nodes << parse_node until block_close?(:KW_IF) || else_open?
      nodes
    end

    def parse_unless_block
      token = consume(:KW_UNLESS)
      line = token.line
      column = token.column

      consume_required_whitespace
      condition = parse_path
      skip_whitespace
      consume(:CLOSE)

      body_nodes = parse_unless_body

      consume_block_close(:KW_UNLESS)

      AST::UnlessBlock.new(
        condition: condition,
        body_nodes: body_nodes,
        line: line,
        column: column
      )
    end

    def parse_unless_body
      nodes = []
      nodes << parse_node until block_close?(:KW_UNLESS)
      nodes
    end

    def else_open?
      return false unless current_type == :HASH

      saved_pos = @pos
      advance_token # hash
      skip_whitespace
      result = current_type == :KW_ELSE
      @pos = saved_pos
      result
    end

    def consume_else
      consume(:HASH)
      skip_whitespace
      consume(:KW_ELSE)
      skip_whitespace
      consume(:CLOSE)
    end

    def parse_each_block
      token = consume(:KW_EACH)
      line = token.line
      column = token.column

      consume_required_whitespace
      collection = parse_path
      consume_required_whitespace
      consume(:KW_AS)
      consume_required_whitespace
      item_name = parse_identifier_with_validation

      skip_whitespace
      consume(:CLOSE)

      body_nodes = parse_each_body

      consume_block_close(:KW_EACH)

      AST::EachBlock.new(
        collection: collection,
        item_name: item_name,
        body_nodes: body_nodes,
        line: line,
        column: column
      )
    end

    def parse_each_body
      nodes = []
      nodes << parse_node until block_close?(:KW_EACH)
      nodes
    end

    def parse_unsecure_output
      token = consume(:BANG_UNSECURE)
      line = token.line
      column = token.column

      consume_required_whitespace
      path = parse_path
      skip_whitespace
      consume(:CLOSE)

      AST::UnsecureOutput.new(path: path, line: line, column: column)
    end

    def parse_include
      token = consume(:BANG_INCLUDE)
      line = token.line
      column = token.column

      consume_required_whitespace
      name = parse_include_name
      args = parse_include_args
      skip_whitespace
      consume(:CLOSE)

      AST::Include.new(name: name, args: args, line: line, column: column)
    end

    def parse_include_name
      first_token = current_token
      unless current_type == :SLASH
        raise ParseError.new("Include name must start with '/'", line: first_token.line, column: first_token.column)
      end

      segments = [parse_include_segment]
      segments << parse_include_segment while current_type == :SLASH

      path = segments.join
      Validator.validate_include_name_syntax!(path, line: first_token.line, column: first_token.column)
      path
    end

    def parse_include_segment
      consume(:SLASH)

      token = current_token
      if current_type == :INVALID
        raise LexerError.new("Invalid character in include path: '#{token.value}'",
                             line: token.line, column: token.column)
      end
      unless current_type == :IDENT
        raise ParseError.new('Expected identifier after /', line: token.line, column: token.column)
      end

      ident_token = consume(:IDENT)
      if ident_token.value.start_with?('_')
        raise LexerError.new("Include segment cannot start with underscore: #{ident_token.value}",
                             line: ident_token.line, column: ident_token.column)
      end

      "/#{ident_token.value}"
    end

    def parse_include_args
      args = {}

      while current_type == :WHITESPACE
        skip_whitespace
        break unless current_type == :IDENT

        key, value, key_token = parse_include_arg
        if args.key?(key)
          raise ParseError.new("Duplicate include argument: #{key}", line: key_token.line, column: key_token.column)
        end

        args[key] = value
      end

      args
    end

    def parse_include_arg
      key_token = consume(:IDENT)
      Validator.validate_identifier!(key_token.value, line: key_token.line, column: key_token.column)

      skip_whitespace
      consume(:EQUAL)
      skip_whitespace
      value = parse_path

      [key_token.value, value, key_token]
    end

    def parse_variable_node
      path = parse_path(allow_modifier: true)
      skip_whitespace
      consume(:CLOSE)
      path
    end

    def parse_path(allow_modifier: false)
      first_token = current_token
      segments = [parse_identifier_with_validation]

      while current_type == :DOT
        consume(:DOT)
        segments << parse_identifier_with_validation
      end

      modifier = nil
      modifier = parse_modifier if allow_modifier

      AST::Variable.new(segments, modifier: modifier, line: first_token.line, column: first_token.column)
    end

    def parse_modifier
      case current_type
      when :QUESTION
        advance_token
        :nullable
      when :EXCLAMATION
        advance_token
        :required
      end
    end

    def parse_identifier_with_validation
      token = current_token

      if keyword_token?(token)
        advance_token
        raise ReservedWordError.new("'#{token.value}' is a reserved word", line: token.line, column: token.column)
      end

      token = consume(:IDENT)

      if Token::RESERVED_WORDS.include?(token.value)
        raise ReservedWordError.new("'#{token.value}' is a reserved word", line: token.line, column: token.column)
      end

      Validator.validate_identifier!(token.value, line: token.line, column: token.column)
      token.value
    end

    def keyword_token?(token)
      return false if token.nil?

      %i[KW_IF KW_UNLESS KW_ELSE KW_EACH KW_AS].include?(token.type)
    end

    def block_close?(keyword = nil)
      return false unless current_type == :SLASH

      return true unless keyword

      saved_pos = @pos
      advance_token # slash
      skip_whitespace
      result = current_type == keyword
      @pos = saved_pos
      result
    end

    def consume_block_close(keyword)
      consume(:SLASH)
      skip_whitespace
      consume(keyword)
      skip_whitespace
      consume(:CLOSE)
    end

    def current_token
      @tokens[@pos]
    end

    def current_type
      current_token&.type
    end

    def advance_token
      @pos += 1
    end

    def consume(type)
      token = current_token
      unexpected_token!("Expected #{type}") if token.nil? || token.type != type
      advance_token
      token
    end

    def consume_required_whitespace
      unexpected_token!('Expected whitespace') unless current_type == :WHITESPACE
      skip_whitespace
    end

    def skip_whitespace
      advance_token while current_type == :WHITESPACE
    end

    def unexpected_token!(message = nil)
      token = current_token
      msg = message || 'Unexpected token'
      msg = "#{msg}: #{token.type}" if token
      raise ParseError.new(msg, line: token&.line, column: token&.column)
    end
  end
end
