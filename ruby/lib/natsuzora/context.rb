# frozen_string_literal: true

module Natsuzora
  class Context
    def initialize(root_data)
      @root = normalize_data(root_data)
      @local_stack = []
    end

    def resolve(path)
      name = path.first
      value = resolve_name(name)

      path[1..].each do |segment|
        value = access_property(value, segment)
      end

      value
    end

    def push_scope(bindings = {})
      validate_no_shadowing!(bindings)
      @local_stack.push(normalize_data(bindings))
    end

    def push_include_scope(bindings)
      @local_stack.push(normalize_data(bindings))
    end

    def pop_scope
      @local_stack.pop
    end

    def with_scope(bindings, include_scope: false)
      if include_scope
        push_include_scope(bindings)
      else
        push_scope(bindings)
      end
      yield
    ensure
      pop_scope
    end

    private

    def resolve_name(name)
      @local_stack.reverse_each do |scope|
        return scope[name] if scope.key?(name)
      end

      return @root[name] if @root.key?(name)

      raise UndefinedVariableError, "Undefined variable: #{name}"
    end

    def validate_no_shadowing!(bindings)
      bindings.each_key do |name|
        name_str = name.to_s
        next unless name_exists?(name_str)

        raise ShadowingError, "Cannot shadow existing variable: #{name_str}"
      end
    end

    def name_exists?(name)
      @local_stack.any? { |scope| scope.key?(name) } || @root.key?(name)
    end

    def access_property(value, key)
      raise TypeError, "Cannot access property '#{key}' on non-object" unless value.is_a?(Hash)

      raise UndefinedVariableError, "Undefined property: #{key}" unless value.key?(key)

      value[key]
    end

    def normalize_data(data)
      case data
      when Hash
        data.transform_keys(&:to_s).transform_values { |v| normalize_data(v) }
      when Array
        data.map { |v| normalize_data(v) }
      else
        data
      end
    end
  end
end
