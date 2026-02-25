require 'spec_helper'
require 'json'

RSpec.describe 'Pokemon API' do
  describe 'GET /health' do
    it 'returns health status' do
      get '/health'

      expect(last_response.status).to eq(200)
      body = JSON.parse(last_response.body)
      expect(body['status']).to eq('ok')
      expect(body).to have_key('timestamp')
    end
  end

  describe 'GET /pokemon' do
    it 'returns all pokemon' do
      get '/pokemon'

      expect(last_response.status).to eq(200)
      pokemon = JSON.parse(last_response.body)
      expect(pokemon).to be_an(Array)
      expect(pokemon.length).to eq(5)
    end

    it 'returns pokemon with expected attributes' do
      get '/pokemon'

      pokemon = JSON.parse(last_response.body)
      first = pokemon.first
      expect(first).to have_key('id')
      expect(first).to have_key('name')
      expect(first).to have_key('type')
      expect(first).to have_key('hp')
    end
  end

  describe 'GET /pokemon/:id' do
    it 'returns a specific pokemon' do
      get '/pokemon/1'

      expect(last_response.status).to eq(200)
      pokemon = JSON.parse(last_response.body)
      expect(pokemon['id']).to eq(1)
      expect(pokemon['name']).to eq('Bulbasaur')
    end

    it 'returns 404 for non-existent pokemon' do
      get '/pokemon/999'

      expect(last_response.status).to eq(404)
      body = JSON.parse(last_response.body)
      expect(body['error']).to match(/not found/)
    end
  end

  describe 'POST /pokemon' do
    it 'creates a new pokemon' do
      new_pokemon = { name: 'Mewtwo', type: 'Psychic', hp: 110 }

      post '/pokemon', new_pokemon.to_json, { 'CONTENT_TYPE' => 'application/json' }

      expect(last_response.status).to eq(201)
      body = JSON.parse(last_response.body)
      expect(body['id']).to be > 0
      expect(body['name']).to eq('Mewtwo')
      expect(body['type']).to eq('Psychic')
      expect(body['hp']).to eq(110)
    end

    it 'returns 400 when name is missing' do
      invalid = { type: 'Fire' }

      post '/pokemon', invalid.to_json, { 'CONTENT_TYPE' => 'application/json' }

      expect(last_response.status).to eq(400)
      body = JSON.parse(last_response.body)
      expect(body['error']).to match(/Missing required fields/)
    end

    it 'returns 400 when type is missing' do
      invalid = { name: 'Unknown' }

      post '/pokemon', invalid.to_json, { 'CONTENT_TYPE' => 'application/json' }

      expect(last_response.status).to eq(400)
    end

    it 'uses default hp when not provided' do
      new_pokemon = { name: 'Magikarp', type: 'Water' }

      post '/pokemon', new_pokemon.to_json, { 'CONTENT_TYPE' => 'application/json' }

      expect(last_response.status).to eq(201)
      body = JSON.parse(last_response.body)
      expect(body['hp']).to eq(50)
    end
  end

  describe 'unknown endpoints' do
    it 'returns 404' do
      get '/unknown'

      expect(last_response.status).to eq(404)
      body = JSON.parse(last_response.body)
      expect(body['error']).to eq('Endpoint not found')
    end
  end
end
