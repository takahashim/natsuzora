# frozen_string_literal: true

require_relative 'contract/parse_error'
require_relative 'contract/diff_marker'
require_relative 'contract/validation_target'
require_relative 'contract/scalar_type'
require_relative 'contract/modifier'
require_relative 'contract/ast'
require_relative 'contract/field'
require_relative 'contract/type_def'
require_relative 'contract/document'
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
