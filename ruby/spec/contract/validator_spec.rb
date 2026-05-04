# frozen_string_literal: true

require 'spec_helper'

RSpec.describe Natsuzora::Contract::Validator do
  describe '.validate' do
    it 'validates simple object' do
      contract = Natsuzora::Contract.parse('name: string')
      expect { Natsuzora::Contract.validate(contract, { 'name' => 'Alice' }) }.not_to raise_error
    end

    it 'raises error for missing required field' do
      contract = Natsuzora::Contract.parse('name: string')
      expect { Natsuzora::Contract.validate(contract, {}) }.to raise_error(Natsuzora::Contract::ValidationError, /missing required/)
    end

    it 'validates nullable modifier' do
      contract = Natsuzora::Contract::AST::Scalar.new(Natsuzora::Contract::ScalarType::STRING, Natsuzora::Contract::AST::Scalar::Modifier::NULLABLE)
      expect { Natsuzora::Contract.validate(contract, nil) }.not_to raise_error
    end

    it 'raises error for null without nullable modifier' do
      contract = Natsuzora::Contract::AST::Scalar.new(Natsuzora::Contract::ScalarType::STRING, Natsuzora::Contract::AST::Scalar::Modifier::NONE)
      expect { Natsuzora::Contract.validate(contract, nil) }.to raise_error(Natsuzora::Contract::ValidationError, /null is not allowed/)
    end

    it 'validates required modifier rejects empty string' do
      contract = Natsuzora::Contract::AST::Scalar.new(Natsuzora::Contract::ScalarType::STRING, Natsuzora::Contract::AST::Scalar::Modifier::REQUIRED)
      expect { Natsuzora::Contract.validate(contract, '') }.to raise_error(Natsuzora::Contract::ValidationError, /empty string/)
    end

    it 'validates integer type' do
      contract = Natsuzora::Contract::AST::Scalar.new(Natsuzora::Contract::ScalarType::INTEGER, Natsuzora::Contract::AST::Scalar::Modifier::NONE)
      expect { Natsuzora::Contract.validate(contract, 42) }.not_to raise_error
      expect { Natsuzora::Contract.validate(contract, 'not an integer') }.to raise_error(Natsuzora::Contract::ValidationError)
    end

    it 'validates bool type' do
      contract = Natsuzora::Contract::AST::Scalar.new(Natsuzora::Contract::ScalarType::BOOL, Natsuzora::Contract::AST::Scalar::Modifier::NONE)
      expect { Natsuzora::Contract.validate(contract, true) }.not_to raise_error
      expect { Natsuzora::Contract.validate(contract, false) }.not_to raise_error
      expect { Natsuzora::Contract.validate(contract, 'not a bool') }.to raise_error(Natsuzora::Contract::ValidationError)
    end

    it 'validates scalar type (string or integer)' do
      contract = Natsuzora::Contract::AST::Scalar.new(Natsuzora::Contract::ScalarType::SCALAR, Natsuzora::Contract::AST::Scalar::Modifier::NONE)
      expect { Natsuzora::Contract.validate(contract, 'hello') }.not_to raise_error
      expect { Natsuzora::Contract.validate(contract, 42) }.not_to raise_error
      expect { Natsuzora::Contract.validate(contract, true) }.to raise_error(Natsuzora::Contract::ValidationError)
    end

    it 'validates arrays' do
      contract = Natsuzora::Contract::AST::List.new(
        Natsuzora::Contract::AST::Scalar.new(Natsuzora::Contract::ScalarType::STRING)
      )
      expect { Natsuzora::Contract.validate(contract, %w[a b c]) }.not_to raise_error
      expect { Natsuzora::Contract.validate(contract, 'not an array') }.to raise_error(Natsuzora::Contract::ValidationError, /expected array/)
    end

    it 'validates nested objects' do
      contract = Natsuzora::Contract.parse(<<~SCHEMA)
        user {
          name: string
        }
      SCHEMA
      expect { Natsuzora::Contract.validate(contract, { 'user' => { 'name' => 'Alice' } }) }.not_to raise_error
      expect { Natsuzora::Contract.validate(contract, { 'user' => {} }) }.to raise_error(Natsuzora::Contract::ValidationError)
    end

    it 'reports correct path in error' do
      contract = Natsuzora::Contract.parse('items: []{ title: string }')
      expect { Natsuzora::Contract.validate(contract, { 'items' => [{ 'title' => 123 }] }) }
        .to raise_error(Natsuzora::Contract::ValidationError) do |error|
          expect(error.path).to eq('$.items[0].title')
        end
    end
  end

  describe '.validate_with_target' do
    it 'validates added field for current (not required)' do
      file = Natsuzora::Contract.parse_file_with_diff(<<~SCHEMA)
        name: string
        + email: string
      SCHEMA

      expect { Natsuzora::Contract.validate_with_target(file, { 'name' => 'Alice' }) }.not_to raise_error
    end

    it 'validates added field for next (required)' do
      file = Natsuzora::Contract.parse_file_with_diff(<<~SCHEMA)
        name: string
        + email: string
      SCHEMA

      expect { Natsuzora::Contract.validate_with_target(file, { 'name' => 'Alice' }, target: Natsuzora::Contract::ValidationTarget::NEXT) }
        .to raise_error(Natsuzora::Contract::ValidationError, /missing required/)
    end

    it 'validates removed field for current (required)' do
      file = Natsuzora::Contract.parse_file_with_diff(<<~SCHEMA)
        name: string
        - legacyId: integer
      SCHEMA

      expect { Natsuzora::Contract.validate_with_target(file, { 'name' => 'Alice' }) }
        .to raise_error(Natsuzora::Contract::ValidationError, /missing required/)
    end

    it 'validates removed field for next (not required)' do
      file = Natsuzora::Contract.parse_file_with_diff(<<~SCHEMA)
        name: string
        - legacyId: integer
      SCHEMA

      expect { Natsuzora::Contract.validate_with_target(file, { 'name' => 'Alice' }, target: Natsuzora::Contract::ValidationTarget::NEXT) }
        .not_to raise_error
    end

    it 'validates changed field uses current type for current' do
      file = Natsuzora::Contract.parse_file_with_diff('* age: integer -> scalar')

      expect { Natsuzora::Contract.validate_with_target(file, { 'age' => 30 }) }.not_to raise_error
      expect { Natsuzora::Contract.validate_with_target(file, { 'age' => '30' }) }.to raise_error(Natsuzora::Contract::ValidationError)
    end

    it 'validates changed field uses next type for next' do
      file = Natsuzora::Contract.parse_file_with_diff('* age: integer -> scalar')

      expect { Natsuzora::Contract.validate_with_target(file, { 'age' => 30 }, target: Natsuzora::Contract::ValidationTarget::NEXT) }
        .not_to raise_error
      expect { Natsuzora::Contract.validate_with_target(file, { 'age' => '30' }, target: Natsuzora::Contract::ValidationTarget::NEXT) }
        .not_to raise_error
    end
  end
end
