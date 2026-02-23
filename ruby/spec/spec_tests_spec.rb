# frozen_string_literal: true

require 'spec_helper'
require 'json'
require 'tmpdir'
require 'fileutils'

RSpec.describe 'Spec Tests' do # rubocop:disable RSpec/DescribeClass
  def setup_partials(partials)
    dir = Dir.mktmpdir('natsuzora_test')
    partials.each do |name, content|
      segments = name.split('/').reject(&:empty?)
      segments[-1] = "_#{segments[-1]}"
      path = "#{File.join(dir, *segments)}.ntzr"
      FileUtils.mkdir_p(File.dirname(path))
      File.write(path, content)
    end
    dir
  end

  def run_test_case(test_case)
    # Map error types from spec to Ruby exception classes
    # SyntaxError matches both LexerError and ParseError (implementation detail)
    error_types = {
      'UndefinedVariable' => Natsuzora::UndefinedVariableError,
      'NullValueError' => Natsuzora::TypeError,
      'EmptyStringError' => Natsuzora::TypeError,
      'TypeError' => Natsuzora::TypeError,
      'ReservedWordError' => [Natsuzora::ParseError, Natsuzora::ReservedWordError],
      'ShadowingError' => Natsuzora::ShadowingError,
      'SyntaxError' => [Natsuzora::LexerError, Natsuzora::ParseError],
      'IncludeError' => Natsuzora::IncludeError
    }.freeze

    template = test_case['template']
    data = test_case['data']
    expected = test_case['expected']
    error_type = test_case['error']
    partials = test_case['partials']

    include_root = partials ? setup_partials(partials) : nil

    if expected
      # Success case
      result = Natsuzora.render(template, data, include_root: include_root)
      expect(result).to eq(expected), lambda {
        "Template: #{template.inspect}\nData: #{data.inspect}\nExpected: #{expected.inspect}\nGot: #{result.inspect}"
      }
    elsif error_type
      # Error case
      error_classes = error_types[error_type] || Natsuzora::Error
      error_classes = Array(error_classes)
      expect { Natsuzora.render(template, data, include_root: include_root) }.to(raise_error do |e|
        expect(error_classes.any? { |klass| e.is_a?(klass) }).to be(true),
                                                                 "Expected one of #{error_classes.map(&:name).join(', ')} but got #{e.class.name}\n" \
                                                                 "Template: #{template.inspect}\nData: #{data.inspect}\nError: #{e.message}"
      end)
    else
      raise "Invalid test case: must have 'expected' or 'error'"
    end
  ensure
    FileUtils.rm_rf(include_root) if include_root
  end

  spec_tests_dir = File.expand_path('../../tests', __dir__)
  Dir.glob(File.join(spec_tests_dir, '*.json')).each do |file|
    filename = File.basename(file)

    describe filename do
      test_data = JSON.parse(File.read(file))

      test_data['tests'].each do |test_case|
        it test_case['name'] do # rubocop:disable RSpec/NoExpectationExample
          run_test_case(test_case)
        end
      end
    end
  end
end
