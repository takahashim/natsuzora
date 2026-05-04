# frozen_string_literal: true

require 'json'
require_relative 'type_ref_resolver'
require_relative 'validation_target'
require_relative 'scalar_type'
require_relative 'ast/any'
require_relative 'ast/scalar'
require_relative 'ast/record'
require_relative 'ast/list'
require_relative 'ast/ref'

module Natsuzora
  module Contract
    # Error that can occur during validation.
    class ValidationError < StandardError
      attr_reader :path, :error_message

      def initialize(message, path = '$')
        @path = path
        @error_message = message
        super("#{message} at #{path}")
      end
    end

    # Validator for JSON data against contracts.
    class Validator
      # Validate JSON data against a contract.
      def self.validate(contract, data)
        new.validate_node(contract, data, [])
      end

      # Validate JSON data against a contract file with diff markers.
      def self.validate_with_target(document, data, target: ValidationTarget::CURRENT)
        contract = build_contract_for_target(document, target)
        validate(contract, data)
      end

      def validate_node(contract, data, path)
        case contract
        when AST::Any
          nil # any value is valid
        when AST::Scalar
          validate_scalar(contract, data, path)
        when AST::Record
          validate_object(contract, data, path)
        when AST::List
          validate_array(contract, data, path)
        when AST::Ref
          raise ValidationError.new("unresolved type reference '#{contract.name}'", render_path(path))
        else
          raise ArgumentError, "Unknown contract type: #{contract.class}"
        end
      end

      private

      def validate_scalar(contract, data, path)
        return validate_null(contract, path) if data.nil?
        return nil if scalar_value_valid?(contract, data, path)

        raise ValidationError.new("expected #{contract.scalar_type}", render_path(path))
      end

      def validate_null(contract, path)
        return nil if contract.nullable?

        raise ValidationError.new('null is not allowed', render_path(path))
      end

      def scalar_value_valid?(contract, data, path)
        case contract.scalar_type
        when ScalarType::STRING then valid_string?(contract, data, path)
        when ScalarType::INTEGER then data.is_a?(Integer)
        when ScalarType::BOOL then data.is_a?(TrueClass) || data.is_a?(FalseClass)
        when ScalarType::SCALAR then valid_scalar?(contract, data, path)
        else false
        end
      end

      def valid_string?(contract, data, path)
        return false unless data.is_a?(String)

        ensure_not_empty!(contract, data, path)
        true
      end

      def valid_scalar?(contract, data, path)
        return valid_string?(contract, data, path) if data.is_a?(String)

        data.is_a?(Integer)
      end

      def ensure_not_empty!(contract, data, path)
        return unless contract.required? && data.empty?

        raise ValidationError.new('empty string is not allowed', render_path(path))
      end

      def validate_object(contract, data, path)
        unless data.is_a?(Hash)
          raise ValidationError.new('expected object', render_path(path))
        end

        # Check required fields
        contract.required.each do |key|
          unless data.key?(key) || data.key?(key.to_sym)
            raise ValidationError.new('missing required property', render_path(path + [[:key, key]]))
          end
        end

        # Validate properties
        contract.properties.each do |key, child_contract|
          value = if data.key?(key)
                    data[key]
                  elsif data.key?(key.to_sym)
                    data[key.to_sym]
                  end
          next if value.nil? && !data.key?(key) && !data.key?(key.to_sym)

          validate_node(child_contract, value, path + [[:key, key]])
        end

        nil
      end

      def validate_array(contract, data, path)
        unless data.is_a?(Array)
          raise ValidationError.new('expected array', render_path(path))
        end

        data.each_with_index do |item, idx|
          validate_node(contract.items, item, path + [[:index, idx]])
        end

        nil
      end

      def render_path(path)
        result = '$'
        path.each do |(type, value)|
          case type
          when :key
            result += ".#{value}"
          when :index
            result += "[#{value}]"
          end
        end
        result
      end

      class << self
        private

        def build_contract_for_target(document, target)
          resolver = TypeRefResolver.new(
            document.types,
            target: target,
            on_missing: ->(name) { raise ValidationError, "undefined type '#{name}'" },
            on_unavailable: ->(name) { raise ValidationError, "type '#{name}' is not available for #{target}" }
          )

          properties = {}
          required = []

          document.fields.each do |name, field|
            contract = field.for_target(target)
            next unless contract

            properties[name] = resolver.resolve(contract)
            required << name
          end

          AST::Record.new(properties, required)
        end
      end
    end

    # Validate JSON data against a contract.
    def self.validate(contract, data) # rubocop:disable Naming/PredicateMethod
      Validator.validate(contract, data)
      true
    end

    # Validate JSON data against a contract file with diff markers.
    def self.validate_with_target(document, data, target: ValidationTarget::CURRENT) # rubocop:disable Naming/PredicateMethod
      Validator.validate_with_target(document, data, target: target)
      true
    end
  end
end
