# frozen_string_literal: true

module Natsuzora
  class TemplateLoader
    def initialize(include_root)
      @include_root = include_root ? File.expand_path(include_root) : nil
      @cache = {}
      @include_stack = []
    end

    def load(name)
      validate_include_root!
      validate_name!(name)

      raise IncludeError, "Circular include detected: #{name}" if @include_stack.include?(name)

      @cache[name] ||= load_and_parse(name)
    end

    def with_include(name)
      @include_stack.push(name)
      yield
    ensure
      @include_stack.pop
    end

    private

    def validate_include_root!
      return if @include_root

      raise IncludeError, 'include_root is not configured'
    end

    def validate_name!(name)
      Validator.validate_include_name_runtime!(name)
    end

    def load_and_parse(name)
      path = resolve_path(name)
      validate_path_security!(path)

      raise IncludeError, "Include file not found: #{name} (#{path})" unless File.exist?(path)

      source = File.read(path, encoding: 'UTF-8')
      tokens = Lexer.new(source).tokenize
      Parser.new(tokens).parse
    end

    def resolve_path(name)
      segments = name.split('/').reject(&:empty?)
      segments[-1] = "_#{segments[-1]}"
      "#{File.join(@include_root, *segments)}.tmpl"
    end

    def validate_path_security!(path)
      expanded = File.expand_path(path)
      return if expanded.start_with?(@include_root)

      raise IncludeError, "Path traversal detected: #{path}"
    end
  end
end
