# frozen_string_literal: true

module Natsuzora
  class TemplateLoader
    class IncludePathResolver
      def initialize(include_root)
        @include_root = File.expand_path(include_root)
        @include_root_realpath = nil
      end

      def resolve_template_path(name)
        segments = name.split('/').reject(&:empty?)
        segments[-1] = "_#{segments[-1]}"
        "#{File.join(@include_root, *segments)}.ntzr"
      end

      def ensure_within_root!(path)
        candidate = canonicalize_candidate(path)
        root = canonical_root
        return if within_root?(candidate, root)

        raise IncludeError, "Path traversal detected: #{path}"
      end

      private

      def canonical_root
        return @include_root_realpath if @include_root_realpath

        @include_root_realpath = File.realpath(@include_root)
      rescue Errno::ENOENT, Errno::EACCES => e
        raise IncludeError, "Invalid include root: #{e.message}"
      end

      def canonicalize_candidate(path)
        absolute = File.expand_path(path)
        return File.realpath(absolute) if File.exist?(absolute)

        existing_parent, missing_segments = split_existing_parent(absolute)
        File.join(File.realpath(existing_parent), *missing_segments)
      rescue Errno::ENOENT, Errno::EACCES => e
        raise IncludeError, "Failed to resolve include path: #{e.message}"
      end

      def split_existing_parent(path)
        cursor = path
        missing_segments = []

        until File.exist?(cursor)
          missing_segments.unshift(File.basename(cursor))
          parent = File.dirname(cursor)
          break if parent == cursor

          cursor = parent
        end

        [cursor, missing_segments]
      end

      def within_root?(path, root)
        return true if path == root

        root_prefix = root.end_with?(File::SEPARATOR) ? root : "#{root}#{File::SEPARATOR}"
        path.start_with?(root_prefix)
      end
    end

    def initialize(include_root)
      @path_resolver = include_root ? IncludePathResolver.new(include_root) : nil
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
      return if @path_resolver

      raise IncludeError, 'include_root is not configured'
    end

    def validate_name!(name)
      Validator.validate_include_name_runtime!(name)
    end

    def load_and_parse(name)
      path = @path_resolver.resolve_template_path(name)
      @path_resolver.ensure_within_root!(path)

      raise IncludeError, "Include file not found: #{name} (#{path})" unless File.file?(path)

      source = File.read(path, encoding: 'UTF-8')
      tokens = Lexer.new(source).tokenize
      Parser.new(tokens).parse
    end

    def include_stack_trace
      parts = @include_stack.map do |name|
        path = @path_resolver.resolve_template_path(name)
        "#{name} (#{path})"
      end
      (parts + ['current include']).join(' > ')
    end
  end
end
