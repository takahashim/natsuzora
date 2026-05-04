# frozen_string_literal: true

require_relative 'contract/types'
require_relative 'contract/compiled_lexer'
require_relative 'contract/parser'
require_relative 'contract/validator'

module Natsuzora
  # Contract notation (formerly the standalone `subaru` gem).
  # Parses `.ntzc` files into a {Contract} tree, validates JSON data against
  # contracts, and supports two-generation diff markers (`+`, `-`, `*`).
  module Contract
    class Error < StandardError; end
  end
end
