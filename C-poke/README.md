# C-Poke: Pokemon REST API Server

A lightweight Pokemon REST API server written in pure C using POSIX sockets.

## Features

- Pure C implementation with no external dependencies
- POSIX sockets for HTTP server
- JSON request/response handling
- In-memory Pokemon storage loaded from CSV
- Robust error handling

## Requirements

- GCC compiler
- POSIX-compliant system (Linux, macOS, BSD)

## Build

```bash
cd C-poke
make
```

## Run

```bash
# Default port 8081
./c-poke

# Custom port
./c-poke -p 3000

# Custom data file
./c-poke -d /path/to/pokemon.csv

# Show help
./c-poke --help
```

## Endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/health` | Health check |
| GET | `/pokemon` | List all Pokemon |
| GET | `/pokemon/{id}` | Get Pokemon by ID |
| POST | `/pokemon` | Create new Pokemon |

## API Examples

### Health Check

```bash
curl http://localhost:8081/health
```

Response:
```json
{
  "status": "healthy"
}
```

### List All Pokemon

```bash
curl http://localhost:8081/pokemon
```

Response:
```json
[
  {
    "id": 1,
    "name": "Bulbasaur",
    "type1": "Grass",
    "type2": "Poison",
    "hp": 45,
    "attack": 49,
    "defense": 49,
    "speed": 45
  },
  ...
]
```

### Get Pokemon by ID

```bash
curl http://localhost:8081/pokemon/25
```

Response:
```json
{
  "id": 25,
  "name": "Pikachu",
  "type1": "Electric",
  "type2": "",
  "hp": 35,
  "attack": 55,
  "defense": 40,
  "speed": 90
}
```

### Create New Pokemon

```bash
curl -X POST http://localhost:8081/pokemon \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Eevee",
    "type1": "Normal",
    "type2": "",
    "hp": 55,
    "attack": 55,
    "defense": 50,
    "speed": 55
  }'
```

Response:
```json
{
  "id": 27,
  "name": "Eevee",
  "type1": "Normal",
  "type2": "",
  "hp": 55,
  "attack": 55,
  "defense": 50,
  "speed": 55
}
```

### Error Responses

404 Not Found:
```bash
curl http://localhost:8081/pokemon/999
```
```json
{
  "error": "Pokemon not found"
}
```

## Data Format

The CSV file should have the following columns:
```
id,name,type1,type2,hp,attack,defense,speed
```

## Clean Build

```bash
make clean
make
```

## Debug Build

```bash
make debug
```

## License

MIT
