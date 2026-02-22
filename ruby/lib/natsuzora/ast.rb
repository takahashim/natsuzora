# frozen_string_literal: true

module Natsuzora
  module AST
    class Node
      attr_reader :line, :column

      def initialize(line: nil, column: nil)
        @line = line
        @column = column
      end
    end

    class Template < Node
      attr_reader :nodes

      def initialize(nodes, **)
        super(**)
        @nodes = nodes
      end
    end

    class Text < Node
      attr_reader :content

      def initialize(content, **)
        super(**)
        @content = content
      end
    end

    class Variable < Node
      attr_reader :path, :modifier

      # modifier: nil (default), :nullable (?), :required (!)
      def initialize(path, modifier: nil, **)
        super(**)
        @path = path
        @modifier = modifier
      end
    end

    class IfBlock < Node
      attr_reader :condition, :then_nodes, :else_nodes

      def initialize(condition:, then_nodes:, else_nodes: nil, **)
        super(**)
        @condition = condition
        @then_nodes = then_nodes
        @else_nodes = else_nodes
      end
    end

    class UnlessBlock < Node
      attr_reader :condition, :body_nodes

      def initialize(condition:, body_nodes:, **)
        super(**)
        @condition = condition
        @body_nodes = body_nodes
      end
    end

    class EachBlock < Node
      attr_reader :collection, :item_name, :body_nodes

      def initialize(collection:, item_name:, body_nodes:, **)
        super(**)
        @collection = collection
        @item_name = item_name
        @body_nodes = body_nodes
      end
    end

    class UnsecureOutput < Node
      attr_reader :path

      def initialize(path:, **)
        super(**)
        @path = path
      end
    end

    class Include < Node
      attr_reader :name, :args

      def initialize(name:, args:, **)
        super(**)
        @name = name
        @args = args
      end
    end
  end
end
