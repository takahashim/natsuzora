# frozen_string_literal: true

module Natsuzora
  # Top-level entry that compiles a template source and renders against a
  # {Payload}.
  #
  # `Template#render` accepts only a {Payload}, which is the explicit
  # boundary between untrusted host data and the internal value space.
  # Callers wishing to render from a raw Hash should either use the
  # convenience facade {Natsuzora.render} or wrap the Hash in
  # `Natsuzora::Payload.new(...)` themselves.
  class Template
    attr_reader :ast

    def initialize(source, include_root: nil)
      @source = source
      @include_root = include_root
      @ast = parse_ruby(source)
    end

    # @param payload [Natsuzora::Payload] prepared render input
    # @return [String] rendered output
    def render(payload)
      loader = @include_root ? TemplateLoader.new(@include_root) : nil
      Renderer.new(@ast, template_loader: loader).render(payload.data)
    end

    private

    def parse_ruby(source)
      tokens = Lexer.new(source).tokenize
      Parser.new(tokens).parse
    end
  end
end
