# frozen_string_literal: true

require_relative 'parse_error'
require_relative 'validation_target'
require_relative 'ast/any'
require_relative 'ast/record'
require_relative 'ast/list'
require_relative 'ast/ref'

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
        properties = {}
        required = []

        @fields.each do |name, field|
          contract = field.for_target(target)
          next unless contract

          resolved = resolve_type_refs(contract, target)
          properties[name] = resolved
          required << name
        end

        AST::Record.new(properties, required)
      end

      private

      def resolve_type_refs(contract, target)
        case contract
        when AST::Ref
          type_def = @types[contract.name]
          raise ParseError.new("undefined type '#{contract.name}'", 0, 0) unless type_def
          return AST::Any.new unless type_def.available?(target)

          resolve_type_refs(type_def.contract, target)
        when AST::Record
          resolved_props = contract.properties.transform_values { |c| resolve_type_refs(c, target) }
          AST::Record.new(resolved_props, contract.required)
        when AST::List
          AST::List.new(resolve_type_refs(contract.items, target))
        else
          contract
        end
      end
    end
  end
end
