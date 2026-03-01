# Pokemon API

A FastAPI service providing Pokemon data with list, get, and search endpoints.

## Quick Start

```bash
cd services/pokemon-api

# Create virtual environment
python -m venv .venv
source .venv/bin/activate

# Install dependencies
pip install -e ".[dev]"

# Run the server
uvicorn app.main:app --reload --port 8000
```

## Endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/health` | Health check |
| GET | `/pokemon` | List Pokemon (paginated, filterable by type) |
| GET | `/pokemon/{id}` | Get Pokemon by ID |
| GET | `/pokemon/name/{name}` | Get Pokemon by name |
| GET | `/search?q={query}` | Search Pokemon by name or type |

## Examples

```bash
# Health check
curl http://localhost:8000/health

# List first 10 Pokemon
curl http://localhost:8000/pokemon

# List fire-type Pokemon
curl "http://localhost:8000/pokemon?type=fire"

# Get Pokemon by ID
curl http://localhost:8000/pokemon/25

# Get Pokemon by name
curl http://localhost:8000/pokemon/name/pikachu

# Search for Pokemon
curl "http://localhost:8000/search?q=psy"
```

## API Documentation

Interactive docs available at:
- Swagger UI: http://localhost:8000/docs
- ReDoc: http://localhost:8000/redoc

## Development

```bash
# Run tests
pytest

# Lint
ruff check app/
ruff format app/
```
