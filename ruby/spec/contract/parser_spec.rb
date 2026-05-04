# frozen_string_literal: true

require 'spec_helper'

RSpec.describe Natsuzora::Contract::Parser do
  describe '.parse' do
    it 'parses a simple field' do
      contract = Natsuzora::Contract.parse('name: string')
      expect(contract).to be_a(Natsuzora::Contract::AST::Record)
      expect(contract.properties['name']).to be_a(Natsuzora::Contract::AST::Scalar)
      expect(contract.properties['name'].scalar_type).to eq(Natsuzora::Contract::ScalarType::STRING)
    end

    it 'parses nullable modifier' do
      contract = Natsuzora::Contract.parse('name: string?')
      expect(contract.properties['name'].modifier).to eq(Natsuzora::Contract::Modifier::NULLABLE)
    end

    it 'parses required modifier' do
      contract = Natsuzora::Contract.parse('name: string!')
      expect(contract.properties['name'].modifier).to eq(Natsuzora::Contract::Modifier::REQUIRED)
    end

    it 'parses nested object' do
      contract = Natsuzora::Contract.parse(<<~SCHEMA)
        user {
          name: string!
          age: integer
        }
      SCHEMA

      expect(contract.properties['user']).to be_a(Natsuzora::Contract::AST::Record)
      expect(contract.properties['user'].properties['name']).to be_a(Natsuzora::Contract::AST::Scalar)
      expect(contract.properties['user'].properties['age']).to be_a(Natsuzora::Contract::AST::Scalar)
    end

    it 'parses simple array' do
      contract = Natsuzora::Contract.parse('tags: []string')
      expect(contract.properties['tags']).to be_a(Natsuzora::Contract::AST::List)
      expect(contract.properties['tags'].items).to be_a(Natsuzora::Contract::AST::Scalar)
    end

    it 'parses object array' do
      contract = Natsuzora::Contract.parse(<<~SCHEMA)
        items: []{
          title: string!
          count: integer
        }
      SCHEMA

      expect(contract.properties['items']).to be_a(Natsuzora::Contract::AST::List)
      expect(contract.properties['items'].items).to be_a(Natsuzora::Contract::AST::Record)
    end

    it 'parses type definition' do
      contract = Natsuzora::Contract.parse(<<~SCHEMA)
        type User {
          name: string!
          age: integer
        }

        user: User
      SCHEMA

      expect(contract.properties['user']).to be_a(Natsuzora::Contract::AST::Record)
      expect(contract.properties['user'].properties['name']).to be_a(Natsuzora::Contract::AST::Scalar)
    end

    it 'parses comments' do
      contract = Natsuzora::Contract.parse(<<~SCHEMA)
        # This is a comment
        name: string!
        # Another comment
        age: integer
      SCHEMA

      expect(contract.properties.keys).to contain_exactly('name', 'age')
    end

    it 'parses type as field name' do
      contract = Natsuzora::Contract.parse('type: string')
      expect(contract.properties['type']).to be_a(Natsuzora::Contract::AST::Scalar)
    end

    it 'parses type definition and type field together' do
      contract = Natsuzora::Contract.parse(<<~SCHEMA)
        type User {
          name: string
        }

        type: string
        user: User
      SCHEMA

      expect(contract.properties['type']).to be_a(Natsuzora::Contract::AST::Scalar)
      expect(contract.properties['user']).to be_a(Natsuzora::Contract::AST::Record)
    end

    it 'raises error for unknown type' do
      expect { Natsuzora::Contract.parse('value: unknown') }.to raise_error(Natsuzora::Contract::ParseError)
    end

    it 'raises error for undefined type reference' do
      expect { Natsuzora::Contract.parse('user: UndefinedType') }.to raise_error(Natsuzora::Contract::ParseError)
    end
  end

  describe '.parse_file_with_diff' do
    it 'parses added field' do
      file = Natsuzora::Contract.parse_file_with_diff('+ email: string')
      expect(file.fields['email'].marker).to eq(Natsuzora::Contract::DiffMarker::ADDED)
    end

    it 'parses removed field' do
      file = Natsuzora::Contract.parse_file_with_diff('- legacyId: integer')
      expect(file.fields['legacyId'].marker).to eq(Natsuzora::Contract::DiffMarker::REMOVED)
    end

    it 'parses changed field' do
      file = Natsuzora::Contract.parse_file_with_diff('* age: integer -> scalar')
      expect(file.fields['age'].marker).to eq(Natsuzora::Contract::DiffMarker::CHANGED)
      expect(file.fields['age'].next_type).to be_a(Natsuzora::Contract::AST::Scalar)
    end

    it 'raises error for * marker without arrow' do
      expect { Natsuzora::Contract.parse_file_with_diff('* age: integer') }.to raise_error(Natsuzora::Contract::ParseError)
    end

    it 'raises error for arrow without * marker' do
      expect { Natsuzora::Contract.parse_file_with_diff('age: integer -> scalar') }.to raise_error(Natsuzora::Contract::ParseError)
    end
  end
end
