# frozen_string_literal: true

module Natsuzora
  # Manages variable scope and resolution during rendering.
  class Context
    def initialize(root_data)
      raise Natsuzora::TypeError, 'Root data must be an object' unless root_data.is_a?(Hash)

      @root = root_data
      @local_stack = []
    end

    def resolve(variable)
      path = variable.path
      line = variable.line
      column = variable.column

      name = path.first
      raise UndefinedVariableError.new('Undefined variable: <empty path>', line: line, column: column) unless name

      value = resolve_name(name, line: line, column: column)

      path[1..].each do |segment|
        value = access_property(value, segment, line: line, column: column)
      end

      value
    end

    def push_scope(bindings = {})
      validate_no_shadowing!(bindings)
      @local_stack.push(bindings)
    end

    def push_include_scope(bindings)
      @local_stack.push(bindings)
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

    def resolve_name(name, line: nil, column: nil)
      @local_stack.reverse_each do |scope|
        return scope[name] if scope.key?(name)
      end

      return @root[name] if @root.key?(name)

      raise UndefinedVariableError.new("Undefined variable: #{name}", line: line, column: column)
    end

    def validate_no_shadowing!(bindings)
      bindings.each_key do |name|
        name_str = name.to_s
        origin = binding_origin(name_str)
        next unless origin

        raise ShadowingError,
              "Cannot shadow existing variable '#{name_str}' (already defined in #{origin})"
      end
    end

    def binding_origin(name)
      return 'root data' if @root.key?(name)

      @local_stack.reverse_each do |scope|
        return 'outer local scope' if scope.key?(name)
      end
      nil
    end

    def access_property(value, key, line: nil, column: nil)
      raise TypeError, "Cannot access property '#{key}' on non-object" unless value.is_a?(Hash)

      unless value.key?(key)
        raise UndefinedVariableError.new("Undefined property: #{key}", line: line, column: column)
      end

      value[key]
    end
  end
end
