# frozen_string_literal: true

require 'spec_helper'
require 'json'

# Ruby 側で `natsuzora/tests/contract/*.json` の共有テストを消費する。
# Rust 側 (`natsuzora-contract/tests/shared_tests.rs`) と同じ JSON ファイルを読み、
# 両言語で同じ結果になることで実装間ドリフトを検知する。
RSpec.describe Natsuzora::Contract do
  describe 'Shared Natsuzora::Contract Tests' do
    shared_dir = File.expand_path('../../../tests/contract', __dir__)

    describe 'parser_basic.json' do
      suite = JSON.parse(File.read(File.join(shared_dir, 'parser_basic.json')))

      suite['tests'].each do |tc|
        it tc['name'] do
          if tc['should_parse']
            expect { Natsuzora::Contract.parse(tc['schema']) }.not_to raise_error
          else
            expect { Natsuzora::Contract.parse(tc['schema']) }.to raise_error(StandardError)
          end
        end
      end
    end

    describe 'validator_basic.json' do
      suite = JSON.parse(File.read(File.join(shared_dir, 'validator_basic.json')))

      suite['tests'].each do |tc|
        it tc['name'] do
          contract = Natsuzora::Contract.parse(tc['schema'])
          if tc['valid']
            expect { Natsuzora::Contract.validate(contract, tc['data']) }.not_to raise_error
          else
            expect { Natsuzora::Contract.validate(contract, tc['data']) }.to raise_error(Natsuzora::Contract::ValidationError)
          end
        end
      end
    end
  end
end
