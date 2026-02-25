require 'rack/test'
require 'rspec'
require_relative '../app/app'

RSpec.configure do |config|
  config.expect_with :rspec do |expectations|
    expectations.include_chain_clauses_in_custom_matcher_descriptions = true
  end

  config.mock_with :rspec do |mocks|
    mocks.verify_partial_doubles = true
  end

  config.shared_context_metadata_behavior = :apply_to_host_groups
  config.order = :random

  config.include Rack::Test::Methods

  config.before(:each) do
    header 'Host', 'localhost:8082'
  end

  def app
    PokemonApi.new
  end
end
