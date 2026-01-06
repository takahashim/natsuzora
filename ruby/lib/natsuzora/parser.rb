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

    def parse_nodes(stop_types: [:eof])
      nodes = []
      nodes << parse_node until stop_types.include?(current_type) || block_close?
      nodes
    end

    def parse_node
      case current_type
      when :text
        parse_text
      when :open
        parse_tag
      else
        unexpected_token!
      end
    end

    def parse_text
      token = consume(:text)
      AST::Text.new(token.value, line: token.line, column: token.column)
    end

    def parse_tag
      open_token = consume(:open)

      # Check for illegal whitespace before special characters (#, /, >)
      if current_type == :whitespace
        saved_pos = @pos
        skip_whitespace
        if %i[hash slash gt].include?(current_type)
          raise ParseError.new(
            "Whitespace not allowed after '{[' before '#{current_token.value}'",
            line: open_token.line,
            column: open_token.column
          )
        end
        @pos = saved_pos
      end

      skip_whitespace

      case current_type
      when :hash
        parse_block_open
      when :slash
        unexpected_token!('Unexpected block close')
      when :gt
        parse_include
      else
        parse_variable_node
      end
    end

    def parse_block_open
      consume(:hash)
      skip_whitespace

      case current_type
      when :kw_if
        parse_if_block
      when :kw_unless
        parse_unless_block
      when :kw_each
        parse_each_block
      when :kw_unsecure
        parse_unsecure_block
      when :kw_else
        unexpected_token!("Unexpected 'else' without 'if'")
      else
        unexpected_token!
      end
    end

    def parse_if_block
      token = consume(:kw_if)
      line = token.line
      column = token.column

      consume_required_whitespace
      condition = parse_path
      skip_whitespace
      consume(:close)

      then_nodes = parse_if_body
      else_nodes = nil

      if else_open?
        consume_else
        else_nodes = parse_if_body
      end

      consume_block_close(:kw_if)

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
      nodes << parse_node until block_close?(:kw_if) || else_open?
      nodes
    end

    def parse_unless_block
      token = consume(:kw_unless)
      line = token.line
      column = token.column

      consume_required_whitespace
      condition = parse_path
      skip_whitespace
      consume(:close)

      body_nodes = parse_unless_body

      consume_block_close(:kw_unless)

      AST::UnlessBlock.new(
        condition: condition,
        body_nodes: body_nodes,
        line: line,
        column: column
      )
    end

    def parse_unless_body
      nodes = []
      nodes << parse_node until block_close?(:kw_unless)
      nodes
    end

    def else_open?
      return false unless current_type == :open

      saved_pos = @pos
      advance_token # open

      # If there's whitespace after {{, it cannot be {{#else}}
      if current_type == :whitespace
        @pos = saved_pos
        return false
      end

      result = current_type == :hash
      if result
        advance_token # hash
        skip_whitespace
        result = current_type == :kw_else
      end
      @pos = saved_pos
      result
    end

    def consume_else
      consume(:open)
      skip_whitespace
      consume(:hash)
      skip_whitespace
      consume(:kw_else)
      skip_whitespace
      consume(:close)
    end

    def parse_each_block
      token = consume(:kw_each)
      line = token.line
      column = token.column

      consume_required_whitespace
      collection = parse_path
      consume_required_whitespace
      consume(:kw_as)
      consume_required_whitespace
      item_name = parse_identifier_with_validation

      skip_whitespace
      index_name = nil
      if current_type == :comma
        consume(:comma)
        skip_whitespace
        index_name = parse_identifier_with_validation

        if item_name == index_name
          raise ParseError.new(
            "Item and index cannot have the same name: '#{item_name}'",
            line: line,
            column: column
          )
        end
      end

      skip_whitespace
      consume(:close)

      body_nodes = parse_each_body

      consume_block_close(:kw_each)

      AST::EachBlock.new(
        collection: collection,
        item_name: item_name,
        index_name: index_name,
        body_nodes: body_nodes,
        line: line,
        column: column
      )
    end

    def parse_each_body
      nodes = []
      nodes << parse_node until block_close?(:kw_each)
      nodes
    end

    def parse_unsecure_block
      token = consume(:kw_unsecure)
      line = token.line
      column = token.column

      skip_whitespace
      consume(:close)

      body_nodes = []
      body_nodes << parse_node until block_close?(:kw_unsecure)

      consume_block_close(:kw_unsecure)

      AST::UnsecureBlock.new(body_nodes, line: line, column: column)
    end

    def parse_include
      token = consume(:gt)
      line = token.line
      column = token.column

      skip_whitespace
      name = parse_include_name
      args = parse_include_args
      skip_whitespace
      consume(:close)

      AST::Include.new(name: name, args: args, line: line, column: column)
    end

    def parse_include_name
      token = current_token
      unless token.type == :ident && token.value.start_with?('/')
        raise ParseError.new("Include name must start with '/'", line: token.line, column: token.column)
      end

      Validator.validate_include_name_syntax!(token.value, line: token.line, column: token.column)
      advance_token
      token.value
    end

    def parse_include_args
      args = {}

      while current_type == :whitespace
        skip_whitespace
        break unless current_type == :ident && !current_token.value.start_with?('/')

        key_token = consume(:ident)
        key = key_token.value
        Validator.validate_identifier!(key, line: key_token.line, column: key_token.column)

        if args.key?(key)
          raise ParseError.new("Duplicate include argument: #{key}", line: key_token.line, column: key_token.column)
        end

        skip_whitespace
        consume(:equal)
        skip_whitespace
        value = parse_path
        args[key] = value
      end

      args
    end

    def parse_variable_node
      path = parse_path
      skip_whitespace
      consume(:close)
      path
    end

    def parse_path
      first_token = current_token
      segments = [parse_identifier_with_validation]

      while current_type == :dot
        consume(:dot)
        segments << parse_identifier_with_validation
      end

      AST::Variable.new(segments, line: first_token.line, column: first_token.column)
    end

    def parse_identifier_with_validation
      token = current_token

      # Check if it's a keyword (reserved word used as identifier)
      if keyword_token?(token)
        advance_token
        raise ReservedWordError.new("'#{token.value}' is a reserved word", line: token.line, column: token.column)
      end

      token = consume(:ident)
      Validator.validate_identifier!(token.value, line: token.line, column: token.column)
      token.value
    end

    def keyword_token?(token)
      return false if token.nil?

      %i[kw_if kw_unless kw_else kw_each kw_as kw_unsecure].include?(token.type)
    end

    def block_close?(keyword = nil)
      return false unless current_type == :open

      saved_pos = @pos
      advance_token # open

      # If there's whitespace after {{, it cannot be a block close
      if current_type == :whitespace
        @pos = saved_pos
        return false
      end

      result = current_type == :slash
      if result && keyword
        advance_token # slash
        skip_whitespace
        result = current_type == keyword
      end
      @pos = saved_pos
      result
    end

    def consume_block_close(keyword)
      consume(:open)
      skip_whitespace
      consume(:slash)
      skip_whitespace
      consume(keyword)
      skip_whitespace
      consume(:close)
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
      unexpected_token!('Expected whitespace') unless current_type == :whitespace
      skip_whitespace
    end

    def skip_whitespace
      advance_token while current_type == :whitespace
    end

    def unexpected_token!(message = nil)
      token = current_token
      msg = message || 'Unexpected token'
      msg = "#{msg}: #{token.type}" if token
      raise ParseError.new(msg, line: token&.line, column: token&.column)
    end
  end
end
