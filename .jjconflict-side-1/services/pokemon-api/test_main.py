from app.main import app
from fastapi.testclient import TestClient

client = TestClient(app)


def test_health_check():
    response = client.get("/health")
    assert response.status_code == 200
    data = response.json()
    assert data["status"] == "healthy"
    assert data["service"] == "pokemon-api"


def test_list_pokemon():
    response = client.get("/pokemon")
    assert response.status_code == 200
    data = response.json()
    assert "total" in data
    assert "pokemon" in data
    assert data["total"] >= 1


def test_list_pokemon_with_pagination():
    response = client.get("/pokemon?limit=2&offset=0")
    assert response.status_code == 200
    data = response.json()
    assert len(data["pokemon"]) <= 2


def test_list_pokemon_filter_by_type():
    response = client.get("/pokemon?type=fire")
    assert response.status_code == 200
    data = response.json()
    for p in data["pokemon"]:
        assert "fire" in [t.lower() for t in p["types"]]


def test_get_pokemon_by_id():
    response = client.get("/pokemon/25")
    assert response.status_code == 200
    data = response.json()
    assert data["id"] == 25
    assert data["name"] == "pikachu"


def test_get_pokemon_by_id_not_found():
    response = client.get("/pokemon/99999")
    assert response.status_code == 404


def test_get_pokemon_by_name():
    response = client.get("/pokemon/name/charmander")
    assert response.status_code == 200
    data = response.json()
    assert data["name"] == "charmander"
    assert data["id"] == 4


def test_get_pokemon_by_name_not_found():
    response = client.get("/pokemon/name/nonexistent")
    assert response.status_code == 404


def test_search_pokemon():
    response = client.get("/search?q=char")
    assert response.status_code == 200
    data = response.json()
    assert data["total"] >= 1
    assert any("char" in p["name"] for p in data["pokemon"])


def test_search_pokemon_by_type():
    response = client.get("/search?q=electric")
    assert response.status_code == 200
    data = response.json()
    assert data["total"] >= 1
