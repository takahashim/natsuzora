# frozen_string_literal: true

require_relative 'ast/record'
require_relative 'ast/list'
require_relative 'ast/ref'

module Natsuzora
  module Contract
    # Walks an AST tree and replaces AST::Ref nodes with concrete contracts
    # looked up from a TypeDef registry. Missing or unavailable refs are
    # delegated to caller-supplied callbacks.
    class TypeRefResolver
      def initialize(types, target:, on_missing:, on_unavailable:, on_cyclic:)
        @types = types
        @target = target
        @on_missing = on_missing
        @on_unavailable = on_unavailable
        @on_cyclic = on_cyclic
        @visiting = Set.new
      end

      def resolve(contract)
        case contract
        when AST::Ref
          resolve_ref(contract.name)
        when AST::Record
          AST::Record.new(
            contract.properties.transform_values { |c| resolve(c) },
            contract.required
          )
        when AST::List
          AST::List.new(resolve(contract.items))
        else
          contract
        end
      end

      private

      def resolve_ref(name)
        return @on_cyclic.call(name) if @visiting.include?(name)

        type_def = @types[name]
        return @on_missing.call(name) unless type_def
        return @on_unavailable.call(name) unless type_def.available?(@target)

        @visiting.add(name)
        begin
          resolve(type_def.contract)
        ensure
          @visiting.delete(name)
        end
      end
    end
  end
end
