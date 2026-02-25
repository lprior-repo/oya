#ifndef JSON_H
#define JSON_H

#include "pokemon.h"
#include <stddef.h>

int json_pokemon_to_string(const Pokemon *p, char *buffer, size_t size);
int json_pokemon_list_to_string(Pokemon *pokemons, size_t count, char *buffer, size_t size);
int json_error_to_string(const char *message, char *buffer, size_t size);
int json_health_to_string(const char *status, char *buffer, size_t size);
int json_parse_pokemon(const char *json, Pokemon *pokemon);

#endif
