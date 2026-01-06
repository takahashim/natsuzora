# frozen_string_literal: true

module Natsuzora
  module HtmlEscape
    ESCAPE_MAP = {
      '&' => '&amp;',
      '<' => '&lt;',
      '>' => '&gt;',
      '"' => '&quot;',
      "'" => '&#39;'
    }.freeze

    ESCAPE_REGEXP = /[&<>"']/

    class << self
      def escape(string)
        string.gsub(ESCAPE_REGEXP, ESCAPE_MAP)
      end
    end
  end
end
