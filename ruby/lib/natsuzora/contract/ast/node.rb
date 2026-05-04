# frozen_string_literal: true

module Natsuzora
  module Contract
    module AST
      # Base class for contract AST nodes.
      class Node
        # Convert to hash representation for JSON serialization.
        def to_h
          raise NotImplementedError
        end

        # Deserialize from hash representation.
        def self.from_h(hash)
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
end
