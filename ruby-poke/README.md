# Ruby Pokemon REST API

A simple Pokemon REST API built with Sinatra.

## Setup

```bash
cd ruby-poke
bundle install
```

## Run

```bash
bundle exec rackup -p 8082
# or
bundle exec puma -p 8082
```

## Endpoints

| Method | Endpoint        | Description           |
|--------|-----------------|-----------------------|
| GET    | /health         | Health check          |
| GET    | /pokemon        | List all Pokemon      |
| GET    | /pokemon/:id    | Get Pokemon by ID     |
| POST   | /pokemon        | Create new Pokemon    |

## curl Examples

### Health Check
```bash
curl http://localhost:8082/health
```

### List All Pokemon
```bash
curl http://localhost:8082/pokemon
```

### Get Pokemon by ID
```bash
curl http://localhost:8082/pokemon/1
```

### Create New Pokemon
```bash
curl -X POST http://localhost:8082/pokemon \
  -H "Content-Type: application/json" \
  -d '{"name":"Mewtwo","type":"Psychic","hp":110}'
```

## Run Tests

```bash
bundle exec rspec
```

## Seeded Pokemon

| ID | Name      | Type        | HP |
|----|-----------|-------------|----|
| 1  | Bulbasaur | Grass/Poison| 45 |
| 2  | Charmander| Fire        | 39 |
| 3  | Squirtle  | Water       | 44 |
| 4  | Pikachu   | Electric    | 35 |
| 5  | Eevee     | Normal      | 55 |
