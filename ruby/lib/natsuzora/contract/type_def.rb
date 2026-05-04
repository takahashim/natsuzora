# frozen_string_literal: true

require_relative 'diff_marker'
require_relative 'validation_target'

module Natsuzora
  module Contract
    # A type definition with optional diff marker.
    class TypeDef
      attr_reader :marker, :contract

      def initialize(contract, marker: nil)
        @marker = marker
        @contract = contract
      end

      # Check if this type is available for the specified target.
      # NOTE: Diff resolution table here mirrors Field#for_target — keep in sync.
      def available?(target)
        # rubocop:disable Lint/DuplicateBranch
        case [@marker, target]
        in [nil, _]
          true
        in [DiffMarker::ADDED, ValidationTarget::CURRENT]
          false
        in [DiffMarker::ADDED, ValidationTarget::NEXT]
          true
        in [DiffMarker::REMOVED, ValidationTarget::CURRENT]
          true
        in [DiffMarker::REMOVED, ValidationTarget::NEXT]
          false
        in [DiffMarker::CHANGED, _]
          false # Changed not allowed for type defs
        end
        # rubocop:enable Lint/DuplicateBranch
      end
    end
  end
end
