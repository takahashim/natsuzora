# frozen_string_literal: true

require_relative 'node'

module Natsuzora
  module Contract
    module AST
      # Record-style object with named properties.
      # Serialized as 'kind' => 'object' for JSON compatibility.
      class Record < Node
        attr_reader :properties, :required

        def initialize(properties = {}, required = [])
          super()
          @properties = properties
          @required = required
        end

        def to_h
          h = { 'kind' => 'object', 'properties' => @properties.transform_values(&:to_h) }
          h['required'] = @required unless @required.empty?
          h
        end

        def ==(other)
          other.is_a?(Record) &&
            other.properties == @properties &&
            other.required == @required
        end
      end
    end
  end
end
