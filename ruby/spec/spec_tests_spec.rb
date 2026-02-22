# frozen_string_literal: true

require 'spec_helper'
require 'json'

RSpec.describe 'Spec Tests' do
  SPEC_TESTS_DIR = File.expand_path('../../tests', __dir__)

  # Map error types from spec to Ruby exception classes
  # SyntaxError matches both LexerError and ParseError (implementation detail)
  ERROR_TYPES = {
    'UndefinedVariable' => Natsuzora::UndefinedVariableError,
    'TypeError' => Natsuzora::TypeError,
    'ReservedWordError' => [Natsuzora::ParseError, Natsuzora::ReservedWordError],
    'ShadowingError' => Natsuzora::ShadowingError,
    'SyntaxError' => [Natsuzora::LexerError, Natsuzora::ParseError]
  }.freeze

  def run_test_case(test_case)
    template = test_case['template']
    data = test_case['data']
    expected = test_case['expected']
    error_type = test_case['error']

    if expected
      # Success case
      result = Natsuzora.render(template, data)
      expect(result).to eq(expected), lambda {
        "Template: #{template.inspect}\nData: #{data.inspect}\nExpected: #{expected.inspect}\nGot: #{result.inspect}"
      }
    elsif error_type
      # Error case
      error_classes = ERROR_TYPES[error_type] || Natsuzora::Error
      error_classes = Array(error_classes)
      expect { Natsuzora.render(template, data) }.to(raise_error do |e|
        expect(error_classes.any? { |klass| e.is_a?(klass) }).to be(true),
                                                                 "Expected one of #{error_classes.map(&:name).join(', ')} but got #{e.class.name}\n" \
                                                                 "Template: #{template.inspect}\nData: #{data.inspect}\nError: #{e.message}"
      end)
    else
      raise "Invalid test case: must have 'expected' or 'error'"
    end
  end

  # Skip include tests as they require file system setup
  SKIP_FILES = %w[include.json].freeze

  Dir.glob(File.join(SPEC_TESTS_DIR, '*.json')).each do |file|
    filename = File.basename(file)
    next if SKIP_FILES.include?(filename)

    describe filename do
      test_data = JSON.parse(File.read(file))

      test_data['tests'].each do |test_case|
        it test_case['name'] do
          run_test_case(test_case)
        end
      end
    end
  end
end
