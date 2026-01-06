# frozen_string_literal: true

module Natsuzora
  class Renderer
    def initialize(ast, template_loader: nil)
      @ast = ast
      @template_loader = template_loader
      @escape_enabled = true
    end

    def render(data)
      @context = Context.new(data)
      render_nodes(@ast.nodes)
    end

    private

    def render_nodes(nodes)
      nodes.map { |node| render_node(node) }.join
    end

    def render_node(node)
      case node
      when AST::Text
        render_text(node)
      when AST::Variable
        render_variable(node)
      when AST::IfBlock
        render_if(node)
      when AST::UnlessBlock
        render_unless(node)
      when AST::EachBlock
        render_each(node)
      when AST::UnsecureBlock
        render_unsecure(node)
      when AST::Include
        render_include(node)
      else
        raise RenderError, "Unknown node type: #{node.class}"
      end
    end

    def render_text(node)
      node.content
    end

    def render_variable(node)
      value = @context.resolve(node.path)
      str = Value.stringify(value)
      @escape_enabled ? HtmlEscape.escape(str) : str
    end

    def render_if(node)
      value = @context.resolve(node.condition.path)
      if Value.truthy?(value)
        render_nodes(node.then_nodes)
      elsif node.else_nodes
        render_nodes(node.else_nodes)
      else
        ''
      end
    end

    def render_unless(node)
      value = @context.resolve(node.condition.path)
      if Value.truthy?(value)
        ''
      else
        render_nodes(node.body_nodes)
      end
    end

    def render_each(node)
      collection = @context.resolve(node.collection.path)
      Value.ensure_array!(collection)

      collection.each_with_index.map do |item, index|
        bindings = { node.item_name => item }
        bindings[node.index_name] = index if node.index_name

        @context.with_scope(bindings) do
          render_nodes(node.body_nodes)
        end
      end.join
    end

    def render_unsecure(node)
      prev_escape = @escape_enabled
      @escape_enabled = false
      result = render_nodes(node.nodes)
      @escape_enabled = prev_escape
      result
    end

    def render_include(node)
      raise IncludeError, 'Template loader not configured for include' unless @template_loader

      partial_ast = @template_loader.load(node.name)

      bindings = {}
      node.args.each do |key, var|
        bindings[key] = @context.resolve(var.path)
      end

      @template_loader.with_include(node.name) do
        @context.with_scope(bindings, include_scope: true) do
          render_nodes(partial_ast.nodes)
        end
      end
    end
  end
end
