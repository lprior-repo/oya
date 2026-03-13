from fastapi import FastAPI, HTTPException, Query

from app.data import POKEMON_DATA
from app.models import HealthResponse, Pokemon, PokemonListResponse

app = FastAPI(
    title="Pokemon API",
    description="A FastAPI service for Pokemon data",
    version="1.0.0",
)


@app.get("/health", response_model=HealthResponse)
async def health_check() -> HealthResponse:
    return HealthResponse(status="healthy", service="pokemon-api")


@app.get("/pokemon", response_model=PokemonListResponse)
async def list_pokemon(
    limit: int = Query(default=10, ge=1, le=100),
    offset: int = Query(default=0, ge=0),
    type: str | None = Query(default=None),
) -> PokemonListResponse:
    filtered = POKEMON_DATA
    if type:
        filtered = [p for p in filtered if type.lower() in [t.lower() for t in p.types]]
    total = len(filtered)
    paginated = filtered[offset : offset + limit]
    return PokemonListResponse(total=total, pokemon=paginated)


@app.get("/pokemon/{pokemon_id}", response_model=Pokemon)
async def get_pokemon(pokemon_id: int) -> Pokemon:
    for pokemon in POKEMON_DATA:
        if pokemon.id == pokemon_id:
            return pokemon
    raise HTTPException(status_code=404, detail=f"Pokemon with id {pokemon_id} not found")


@app.get("/pokemon/name/{name}", response_model=Pokemon)
async def get_pokemon_by_name(name: str) -> Pokemon:
    for pokemon in POKEMON_DATA:
        if pokemon.name.lower() == name.lower():
            return pokemon
    raise HTTPException(status_code=404, detail=f"Pokemon '{name}' not found")


@app.get("/search", response_model=PokemonListResponse)
async def search_pokemon(
    q: str = Query(..., min_length=1, description="Search query"),
    limit: int = Query(default=10, ge=1, le=100),
) -> PokemonListResponse:
    query = q.lower()
    results = [
        p
        for p in POKEMON_DATA
        if query in p.name.lower() or any(query in t.lower() for t in p.types)
    ]
    return PokemonListResponse(total=len(results), pokemon=results[:limit])
