# frozen_string_literal: true

require_relative 'ast/node'
require_relative 'ast/any'
require_relative 'ast/scalar'
require_relative 'ast/record'
require_relative 'ast/list'
require_relative 'ast/ref'

module Natsuzora
  module Contract
    # Contract AST nodes and their serialization helpers.
    module AST
      module_function

      # Build an AST::Node tree from its hash representation.
      def from_h(hash)
        case hash['kind']
        when 'any'
          Any.new
        when 'scalar'
          scalar_type = hash['type'].to_sym
          modifier = (hash['modifier'] || 'none').to_sym
          Scalar.new(scalar_type, modifier)
        when 'object'
          required = hash['required'] || []
          properties = (hash['properties'] || {}).transform_values { |v| from_h(v) }
          Record.new(properties, required)
        when 'array'
          items = from_h(hash['items'])
          List.new(items)
        when 'ref'
          Ref.new(hash['name'])
        else
          raise ArgumentError, "Unknown contract kind: #{hash['kind']}"
        end
      end
    end
  end
end
