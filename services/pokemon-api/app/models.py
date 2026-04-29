from pydantic import BaseModel


class Pokemon(BaseModel):
    id: int
    name: str
    types: list[str]
    height: int
    weight: int
    base_experience: int | None = None


class PokemonListResponse(BaseModel):
    total: int
    pokemon: list[Pokemon]


class HealthResponse(BaseModel):
    status: str
    service: str
