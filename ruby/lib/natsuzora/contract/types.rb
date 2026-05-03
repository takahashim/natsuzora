# frozen_string_literal: true

module Natsuzora
  module Contract
    # Error that can occur during parsing.
    class ParseError < StandardError
      attr_reader :line, :column

      def initialize(message, line = 0, column = 0)
        @line = line
        @column = column
        super("#{message} at line #{line}, column #{column}")
      end
    end

    # Diff marker for 2-generation contracts.
    module DiffMarker
      # `+` - Field will be added in next generation
      ADDED = :added
      # `-` - Field will be removed in next generation
      REMOVED = :removed
      # `*` - Field type will change in next generation
      CHANGED = :changed
    end

    # Validation target for 2-generation contracts.
    module ValidationTarget
      # Validate against current generation (default)
      CURRENT = :current
      # Validate against next generation
      NEXT = :next
    end

    # Scalar type enumeration.
    module ScalarType
      # String type only
      STRING = :string
      # Integer type only
      INTEGER = :integer
      # Boolean type only (for truthiness checks)
      BOOL = :bool
      # String or Integer (stringifiable values, does NOT include bool)
      SCALAR = :scalar
    end

    # Modifier for null/empty handling.
    module ContractModifier
      # Default: null not allowed, empty string allowed
      NONE = :none
      # `?` modifier: null allowed
      NULLABLE = :nullable
      # `!` modifier: null not allowed, empty string not allowed
      REQUIRED = :required
    end

    # Base class for contract types.
    class Contract
      # Convert to hash representation for JSON serialization.
      def to_h
        raise NotImplementedError
      end

      # Deserialize from hash representation.
      def self.from_h(hash)
        case hash['kind']
        when 'any'
          AnyContract.new
        when 'scalar'
          scalar_type = hash['type'].to_sym
          modifier = (hash['modifier'] || 'none').to_sym
          ScalarContract.new(scalar_type, modifier)
        when 'object'
          required = hash['required'] || []
          properties = (hash['properties'] || {}).transform_values { |v| from_h(v) }
          ObjectContract.new(properties, required)
        when 'array'
          items = from_h(hash['items'])
          ArrayContract.new(items)
        when 'ref'
          TypeRefContract.new(hash['name'])
        else
          raise ArgumentError, "Unknown contract kind: #{hash['kind']}"
        end
      end
    end

    # Any value (unconstrained).
    class AnyContract < Contract
      def to_h
        { 'kind' => 'any' }
      end

      def ==(other)
        other.is_a?(AnyContract)
      end
    end

    # Scalar value with type and modifier.
    class ScalarContract < Contract
      attr_reader :scalar_type, :modifier

      def initialize(scalar_type, modifier = ContractModifier::NONE)
        super()
        @scalar_type = scalar_type
        @modifier = modifier
      end

      def to_h
        h = { 'kind' => 'scalar', 'type' => @scalar_type.to_s }
        h['modifier'] = @modifier.to_s unless @modifier == ContractModifier::NONE
        h
      end

      def ==(other)
        other.is_a?(ScalarContract) &&
          other.scalar_type == @scalar_type &&
          other.modifier == @modifier
      end
    end

    # Object with named properties.
    class ObjectContract < Contract
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
        other.is_a?(ObjectContract) &&
          other.properties == @properties &&
          other.required == @required
      end
    end

    # Array of items.
    class ArrayContract < Contract
      attr_reader :items

      def initialize(items)
        super()
        @items = items
      end

      def to_h
        { 'kind' => 'array', 'items' => @items.to_h }
      end

      def ==(other)
        other.is_a?(ArrayContract) && other.items == @items
      end
    end

    # Type reference (used during parsing, resolved before use).
    class TypeRefContract < Contract
      attr_reader :name

      def initialize(name)
        super()
        @name = name
      end

      def to_h
        { 'kind' => 'ref', 'name' => @name }
      end

      def ==(other)
        other.is_a?(TypeRefContract) && other.name == @name
      end
    end

    # A field with optional diff marker and type change information.
    class ContractField
      attr_reader :marker, :current_type, :next_type

      def initialize(current_type, marker: nil, next_type: nil)
        @marker = marker
        @current_type = current_type
        @next_type = next_type
      end

      # Create a field without diff marker.
      def self.new_plain(contract)
        new(contract)
      end

      # Create an added field (+).
      def self.added(contract)
        new(contract, marker: DiffMarker::ADDED)
      end

      # Create a removed field (-).
      def self.removed(contract)
        new(contract, marker: DiffMarker::REMOVED)
      end

      # Create a changed field (*).
      def self.changed(current, next_type)
        new(current, marker: DiffMarker::CHANGED, next_type: next_type)
      end

      # Get the contract for the specified target generation.
      def for_target(target)
        case [@marker, target]
        in [nil, _]
          @current_type
        in [DiffMarker::ADDED, ValidationTarget::CURRENT]
          nil
        in [DiffMarker::ADDED, ValidationTarget::NEXT]
          @current_type
        in [DiffMarker::REMOVED, ValidationTarget::CURRENT]
          @current_type
        in [DiffMarker::REMOVED, ValidationTarget::NEXT]
          nil
        in [DiffMarker::CHANGED, ValidationTarget::CURRENT]
          @current_type
        in [DiffMarker::CHANGED, ValidationTarget::NEXT]
          @next_type
        end
      end
    end

    # A type definition with optional diff marker.
    class TypeDef
      attr_reader :marker, :contract

      def initialize(contract, marker: nil)
        @marker = marker
        @contract = contract
      end

      # Check if this type is available for the specified target.
      def available?(target)
        case [@marker, target]
        in [nil, _]
          true
        in [DiffMarker::ADDED, ValidationTarget::CURRENT]
          false
        in [DiffMarker::ADDED, ValidationTarget::NEXT]
          true
        in [DiffMarker::REMOVED, ValidationTarget::CURRENT]
          true
        in [DiffMarker::REMOVED, ValidationTarget::NEXT]
          false
        in [DiffMarker::CHANGED, _]
          false # Changed not allowed for type defs
        end
      end
    end

    # Contract file with type definitions and root fields.
    class ContractFile
      attr_reader :types, :fields

      def initialize(types: {}, fields: {})
        @types = types   # Hash<String, TypeDef>
        @fields = fields # Hash<String, ContractField>
      end

      # Build a Contract object for the specified target.
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

        ObjectContract.new(properties, required)
      end

      private

      def resolve_type_refs(contract, target)
        case contract
        when TypeRefContract
          type_def = @types[contract.name]
          unless type_def
            raise ParseError.new("undefined type '#{contract.name}'", 0, 0)
          end
          return AnyContract.new unless type_def.available?(target)

          resolve_type_refs(type_def.contract, target)
        when ObjectContract
          resolved_props = contract.properties.transform_values { |c| resolve_type_refs(c, target) }
          ObjectContract.new(resolved_props, contract.required)
        when ArrayContract
          ArrayContract.new(resolve_type_refs(contract.items, target))
        else
          contract
        end
      end
    end
  end
end
