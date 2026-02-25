require 'sinatra/base'
require 'json'

class PokemonApi < Sinatra::Base
  set :port, 8082
  set :bind, '0.0.0.0'
  set :protection, false
  set :static, false

  POKEMON_DATA = [
    { id: 1, name: 'Bulbasaur', type: 'Grass/Poison', hp: 45 },
    { id: 2, name: 'Charmander', type: 'Fire', hp: 39 },
    { id: 3, name: 'Squirtle', type: 'Water', hp: 44 },
    { id: 4, name: 'Pikachu', type: 'Electric', hp: 35 },
    { id: 5, name: 'Eevee', type: 'Normal', hp: 55 }
  ].freeze

  attr_reader :pokemon

  def initialize
    super
    @pokemon = POKEMON_DATA.map(&:dup)
  end

  configure do
    middleware = Sinatra::Base.middleware
    middleware.delete_if { |m| m.first.to_s.include?('Protection') }
  end

  before do
    content_type :json
  end

  get '/health' do
    { status: 'ok', timestamp: Time.now.utc.iso8601 }.to_json
  end

  get '/pokemon' do
    @pokemon.to_json
  end

  get '/pokemon/:id' do
    id = params[:id].to_i
    poke = @pokemon.find { |p| p[:id] == id }

    if poke
      poke.to_json
    else
      status 404
      { error: "Pokemon with id #{id} not found" }.to_json
    end
  end

  post '/pokemon' do
    body = JSON.parse(request.body.read, symbolize_names: true)

    unless body[:name] && body[:type]
      status 400
      return { error: 'Missing required fields: name, type' }.to_json
    end

    new_id = (@pokemon.map { |p| p[:id] }.max || 0) + 1
    new_pokemon = {
      id: new_id,
      name: body[:name],
      type: body[:type],
      hp: body[:hp] || 50
    }

    @pokemon << new_pokemon
    status 201
    new_pokemon.to_json
  end

  not_found do
    { error: 'Endpoint not found' }.to_json
  end
end
