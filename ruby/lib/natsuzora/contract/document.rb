# frozen_string_literal: true

require_relative 'parse_error'
require_relative 'validation_target'
require_relative 'type_ref_resolver'
require_relative 'ast/any'
require_relative 'ast/record'

module Natsuzora
  module Contract
    # Parsed contract document with type definitions and root fields.
    class Document
      attr_reader :types, :fields

      def initialize(types: {}, fields: {})
        @types = types   # Hash<String, TypeDef>
        @fields = fields # Hash<String, Field>
      end

      # Build the resolved root AST::Record for the specified target.
      def to_contract(target = ValidationTarget::CURRENT)
        resolver = TypeRefResolver.new(
          @types,
          target: target,
          on_missing: ->(name) { raise ParseError.new("undefined type '#{name}'", 0, 0) },
          on_unavailable: ->(_) { AST::Any.new },
          on_cyclic: ->(name) { raise ParseError.new("cyclic type reference '#{name}'", 0, 0) }
        )

        properties = {}
        required = []

        @fields.each do |name, field|
          contract = field.for_target(target)
          next unless contract

          properties[name] = resolver.resolve(contract)
          required << name
        end

        AST::Record.new(properties, required)
      end
    end
  end
end
