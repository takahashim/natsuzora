# frozen_string_literal: true

module Natsuzora
  class TemplateLoader
    def initialize(include_root)
      @include_root = include_root ? File.expand_path(include_root) : nil
      @include_root_realpath = nil
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
    rescue StandardError => e
      raise e.class, "#{e.message}\n  within include #{include_stack_trace}", e.backtrace
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
      "#{File.join(@include_root, *segments)}.ntzr"
    end

    def include_stack_trace
      parts = @include_stack.map do |name|
        path = resolve_path(name)
        "#{name} (#{path})"
      end
      (parts + ['current include']).join(' > ')
    end

    def validate_path_security!(path)
      expanded = realpath_for_security(path)
      root = include_root_realpath
      return if path_within_root?(expanded, root)

      raise IncludeError, "Path traversal detected: #{path}"
    end

    def include_root_realpath
      return @include_root_realpath if @include_root_realpath

      @include_root_realpath = File.realpath(@include_root)
    rescue Errno::ENOENT, Errno::EACCES => e
      raise IncludeError, "Invalid include root: #{e.message}"
    end

    def path_within_root?(path, root)
      return true if path == root

      root_prefix = root.end_with?(File::SEPARATOR) ? root : "#{root}#{File::SEPARATOR}"
      path.start_with?(root_prefix)
    end

    def realpath_for_security(path)
      expanded = File.expand_path(path)
      return File.realpath(expanded) if File.exist?(expanded)

      existing_path = expanded
      tail_segments = []

      until File.exist?(existing_path)
        tail_segments.unshift(File.basename(existing_path))
        parent = File.dirname(existing_path)
        break if parent == existing_path

        existing_path = parent
      end

      resolved_existing = File.realpath(existing_path)
      File.join(resolved_existing, *tail_segments)
    rescue Errno::ENOENT, Errno::EACCES => e
      raise IncludeError, "Failed to resolve include path: #{e.message}"
    end
  end
end
