# frozen_string_literal: true

require_relative 'diff_marker'
require_relative 'validation_target'

module Natsuzora
  module Contract
    # A field with optional diff marker and type change information.
    class Field
      attr_reader :marker, :current_type, :next_type

      def initialize(current_type, marker: nil, next_type: nil)
        @marker = marker
        @current_type = current_type
        @next_type = next_type
      end

      # Create a field without diff marker.
      def self.new_plain(contract)
        new(contract)
      end

      # Create an added field (+).
      def self.added(contract)
        new(contract, marker: DiffMarker::ADDED)
      end

      # Create a removed field (-).
      def self.removed(contract)
        new(contract, marker: DiffMarker::REMOVED)
      end

      # Create a changed field (*).
      def self.changed(current, next_type)
        new(current, marker: DiffMarker::CHANGED, next_type: next_type)
      end

      # Get the contract for the specified target generation.
      # NOTE: Diff resolution table here mirrors TypeDef#available? — keep in sync.
      def for_target(target)
        # rubocop:disable Lint/DuplicateBranch
        case [@marker, target]
        in [nil, _]
          @current_type
        in [DiffMarker::ADDED, ValidationTarget::CURRENT]
          nil
        in [DiffMarker::ADDED, ValidationTarget::NEXT]
          @current_type
        in [DiffMarker::REMOVED, ValidationTarget::CURRENT]
          @current_type
        in [DiffMarker::REMOVED, ValidationTarget::NEXT]
          nil
        in [DiffMarker::CHANGED, ValidationTarget::CURRENT]
          @current_type
        in [DiffMarker::CHANGED, ValidationTarget::NEXT]
          @next_type
        end
        # rubocop:enable Lint/DuplicateBranch
      end
    end
  end
end
