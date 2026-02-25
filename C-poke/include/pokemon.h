#ifndef POKEMON_H
#define POKEMON_H

#include <stddef.h>

#define MAX_NAME_LEN 64
#define MAX_TYPE_LEN 32
#define MAX_POKEMON 256

typedef struct {
    int id;
    char name[MAX_NAME_LEN];
    char type1[MAX_TYPE_LEN];
    char type2[MAX_TYPE_LEN];
    int hp;
    int attack;
    int defense;
    int speed;
} Pokemon;

typedef struct {
    Pokemon pokemons[MAX_POKEMON];
    size_t count;
    int next_id;
} PokemonStore;

int pokemon_store_init(PokemonStore *store, const char *csv_path);
Pokemon *pokemon_find_by_id(PokemonStore *store, int id);
int pokemon_add(PokemonStore *store, const Pokemon *pokemon);
Pokemon *pokemon_get_all(PokemonStore *store, size_t *count);

#endif
