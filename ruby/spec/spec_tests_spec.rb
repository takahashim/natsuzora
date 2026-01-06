# frozen_string_literal: true

require 'spec_helper'
require 'json'

RSpec.describe 'Spec Tests' do
  SPEC_TESTS_DIR = File.expand_path('../../tests', __dir__)

  # Map error types from spec to Ruby exception classes
  ERROR_TYPES = {
    'UndefinedVariable' => Natsuzora::UndefinedVariableError,
    'TypeError' => Natsuzora::TypeError,
    'ReservedWordError' => Natsuzora::ReservedWordError,
    'ParseError' => Natsuzora::ParseError,
    'ShadowingError' => Natsuzora::ShadowingError,
    'LexerError' => Natsuzora::LexerError
  }.freeze

  def run_test_case(test_case)
    template = test_case['template']
    data = test_case['data']
    expected = test_case['expected']
    error_type = test_case['error']

    if expected
      # Success case
      result = Natsuzora.render(template, data)
      expect(result).to eq(expected), -> {
        "Template: #{template.inspect}\nData: #{data.inspect}\nExpected: #{expected.inspect}\nGot: #{result.inspect}"
      }
    elsif error_type
      # Error case
      error_class = ERROR_TYPES[error_type] || Natsuzora::Error
      expect { Natsuzora.render(template, data) }.to raise_error(error_class), -> {
        "Expected #{error_type} but no error was raised\nTemplate: #{template.inspect}\nData: #{data.inspect}"
      }
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
