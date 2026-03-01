from app.models import Pokemon

POKEMON_DATA: list[Pokemon] = [
    Pokemon(
        id=1, name="bulbasaur", types=["grass", "poison"], height=7, weight=69, base_experience=64
    ),
    Pokemon(id=4, name="charmander", types=["fire"], height=6, weight=85, base_experience=62),
    Pokemon(id=7, name="squirtle", types=["water"], height=5, weight=90, base_experience=63),
    Pokemon(id=25, name="pikachu", types=["electric"], height=4, weight=60, base_experience=112),
    Pokemon(
        id=39, name="jigglypuff", types=["normal", "fairy"], height=5, weight=55, base_experience=95
    ),
    Pokemon(id=52, name="meowth", types=["normal"], height=4, weight=42, base_experience=58),
    Pokemon(id=54, name="psyduck", types=["water"], height=8, weight=196, base_experience=64),
    Pokemon(id=63, name="abra", types=["psychic"], height=9, weight=195, base_experience=62),
    Pokemon(
        id=94, name="gengar", types=["ghost", "poison"], height=15, weight=405, base_experience=225
    ),
    Pokemon(id=133, name="eevee", types=["normal"], height=3, weight=65, base_experience=65),
    Pokemon(id=143, name="snorlax", types=["normal"], height=21, weight=4600, base_experience=189),
    Pokemon(id=150, name="mewtwo", types=["psychic"], height=20, weight=1220, base_experience=340),
    Pokemon(id=151, name="mew", types=["psychic"], height=4, weight=40, base_experience=270),
]
