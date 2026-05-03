# frozen_string_literal: true

require 'json'
require_relative 'types'

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
      def self.validate_with_target(contract_file, data, target: ValidationTarget::CURRENT)
        contract = build_contract_for_target(contract_file, target)
        validate(contract, data)
      end

      def validate_node(contract, data, path)
        case contract
        when AnyContract
          nil # any value is valid
        when ScalarContract
          validate_scalar(contract, data, path)
        when ObjectContract
          validate_object(contract, data, path)
        when ArrayContract
          validate_array(contract, data, path)
        when TypeRefContract
          raise ValidationError.new("unresolved type reference '#{contract.name}'", render_path(path))
        else
          raise ArgumentError, "Unknown contract type: #{contract.class}"
        end
      end

      private

      def validate_scalar(contract, data, path)
        # Handle null
        if data.nil?
          if contract.modifier == ContractModifier::NULLABLE
            return nil
          else
            raise ValidationError.new('null is not allowed', render_path(path))
          end
        end

        # Check type
        valid = case contract.scalar_type
                when ScalarType::STRING
                  if data.is_a?(String)
                    if contract.modifier == ContractModifier::REQUIRED && data.empty?
                      raise ValidationError.new('empty string is not allowed', render_path(path))
                    end

                    true
                  else
                    false
                  end
                when ScalarType::INTEGER
                  data.is_a?(Integer)
                when ScalarType::BOOL
                  data.is_a?(TrueClass) || data.is_a?(FalseClass)
                when ScalarType::SCALAR
                  if data.is_a?(String)
                    if contract.modifier == ContractModifier::REQUIRED && data.empty?
                      raise ValidationError.new('empty string is not allowed', render_path(path))
                    end

                    true
                  else
                    data.is_a?(Integer)
                  end
                else
                  false
                end

        return nil if valid

        raise ValidationError.new("expected #{contract.scalar_type}", render_path(path))
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

        def build_contract_for_target(contract_file, target)
          # Build type definitions map for the target (excluding unavailable types)
          type_defs = {}
          contract_file.types.each do |name, type_def|
            type_defs[name] = type_def.contract if type_def.available?(target)
          end

          # Build root object properties for the target
          properties = {}
          required = []

          contract_file.fields.each do |name, field|
            contract = field.for_target(target)
            next unless contract

            # Resolve type references
            resolved = resolve_type_refs(contract, type_defs)
            properties[name] = resolved
            required << name
          end

          ObjectContract.new(properties, required)
        end

        def resolve_type_refs(contract, type_defs)
          case contract
          when TypeRefContract
            resolved = type_defs[contract.name]
            raise ValidationError.new("undefined type '#{contract.name}'") unless resolved

            resolve_type_refs(resolved, type_defs)
          when ArrayContract
            ArrayContract.new(resolve_type_refs(contract.items, type_defs))
          when ObjectContract
            resolved_props = contract.properties.transform_values do |c|
              resolve_type_refs(c, type_defs)
            end
            ObjectContract.new(resolved_props, contract.required)
          else
            contract
          end
        end
      end
    end

    # Module-level validate functions.
    module_function

    # Validate JSON data against a contract.
    def validate(contract, data)
      Validator.validate(contract, data)
      true
    rescue ValidationError
      raise
    end

    # Validate JSON data against a contract file with diff markers.
    def validate_with_target(contract_file, data, target: ValidationTarget::CURRENT)
      Validator.validate_with_target(contract_file, data, target: target)
      true
    rescue ValidationError
      raise
    end
  end
end
